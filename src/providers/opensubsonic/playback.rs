//! Buffered HTTP range streaming for remote playback.
//!
//! [`SubsonicStreamSource`] implements `Read + Seek` (and Symphonia's
//! [`MediaSource`]) over the Subsonic `stream` endpoint. It reads ahead in
//! bounded chunks using HTTP `Range` requests, so playback can begin without
//! downloading the whole track and seeking is supported when the server honors
//! ranges.
//!
//! All network I/O happens on the playback thread (never the realtime audio
//! callback, which only drains the sample ring buffer), and memory use is
//! bounded by [`CHUNK_SIZE`].

use std::io::{self, Read, Seek, SeekFrom};
use std::time::Duration;

use symphonia::core::io::MediaSource;
use tracing::debug;

use super::{client::OpenSubsonicClient, errors::SubsonicError};

/// Read-ahead chunk size. One chunk is held in memory at a time.
const CHUNK_SIZE: usize = 1024 * 1024;
/// Per-chunk request timeout.
const CHUNK_TIMEOUT: Duration = Duration::from_secs(30);

/// A streamed, seekable view over a remote Subsonic track.
pub struct SubsonicStreamSource {
    client: OpenSubsonicClient,
    track_id: String,
    max_bit_rate: Option<u32>,
    /// Total length in bytes, when the server reports it.
    length: Option<u64>,
    /// Current logical read position.
    position: u64,
    /// Read-ahead buffer contents.
    buffer: Vec<u8>,
    /// Absolute file offset of `buffer[0]`.
    buffer_start: u64,
    /// Whether the server honors `Range` requests.
    seekable: bool,
}

impl SubsonicStreamSource {
    /// Opens a track for streaming, probing for length and range support.
    pub fn open(
        client: OpenSubsonicClient,
        track_id: String,
        max_bit_rate: Option<u32>,
    ) -> Result<Self, SubsonicError> {
        crate::RUNTIME.block_on(Self::open_async(client, track_id, max_bit_rate))
    }

    async fn open_async(
        client: OpenSubsonicClient,
        track_id: String,
        max_bit_rate: Option<u32>,
    ) -> Result<Self, SubsonicError> {
        let mut source = Self {
            client,
            track_id,
            max_bit_rate,
            length: None,
            position: 0,
            buffer: Vec::new(),
            buffer_start: 0,
            seekable: false,
        };

        // Probe with a one-byte range request to learn length + seekability.
        let request = source.client.stream_request(&source.track_id, source.max_bit_rate)?;
        let response = request
            .header("Range", "bytes=0-0")
            .timeout(CHUNK_TIMEOUT)
            .send()
            .await
            .map_err(SubsonicError::Transport)?;

        let status = response.status();
        if status == zed_reqwest::StatusCode::UNAUTHORIZED
            || status == zed_reqwest::StatusCode::FORBIDDEN
        {
            return Err(SubsonicError::AuthenticationFailed);
        }
        if status == zed_reqwest::StatusCode::NOT_FOUND {
            return Err(SubsonicError::NotFound);
        }
        if !status.is_success() && status != zed_reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(SubsonicError::Api {
                code: 0,
                message: format!("HTTP {status}"),
            });
        }

        if status == zed_reqwest::StatusCode::PARTIAL_CONTENT {
            source.seekable = true;
            source.length = response
                .headers()
                .get("content-range")
                .and_then(|v| v.to_str().ok())
                .and_then(parse_content_range_total);
        } else {
            // Server ignored the range; fall back to reading the whole body.
            source.length = response.content_length();
        }

        // Seed the buffer with the probe response's bytes if any.
        let bytes = response.bytes().await.map_err(SubsonicError::Transport)?;
        source.buffer = bytes.to_vec();
        source.buffer_start = 0;

        debug!(
            track_id = %source.track_id,
            length = ?source.length,
            seekable = source.seekable,
            "opened remote stream"
        );

        Ok(source)
    }

    /// Fetches the chunk starting at `self.position` into the read-ahead buffer.
    fn fill_buffer(&mut self) -> io::Result<()> {
        let start = self.position;
        let end = start.saturating_add(CHUNK_SIZE as u64 - 1);
        let range = format!("bytes={start}-{end}");

        let fetch = async {
            let request = self
                .client
                .stream_request(&self.track_id, self.max_bit_rate)
                .map_err(SubsonicError::Transport)?;
            let response = request
                .header("Range", &range)
                .timeout(CHUNK_TIMEOUT)
                .send()
                .await
                .map_err(SubsonicError::Transport)?;

            let status = response.status();
            if !status.is_success() && status != zed_reqwest::StatusCode::PARTIAL_CONTENT {
                return Err(SubsonicError::Api {
                    code: 0,
                    message: format!("HTTP {status}"),
                });
            }

            if status == zed_reqwest::StatusCode::PARTIAL_CONTENT
                && let Some(total) = response
                    .headers()
                    .get("content-range")
                    .and_then(|v| v.to_str().ok())
                    .and_then(parse_content_range_total)
            {
                self.length = Some(total);
                self.seekable = true;
            }

            let bytes = response.bytes().await.map_err(SubsonicError::Transport)?;
            Ok::<_, SubsonicError>(bytes)
        };

        let bytes = crate::RUNTIME
            .block_on(fetch)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        self.buffer_start = start;
        self.buffer = bytes.to_vec();
        Ok(())
    }

    fn byte_len_io(&self) -> u64 {
        self.length.unwrap_or(u64::MAX)
    }
}

impl Read for SubsonicStreamSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        if self.position >= self.byte_len_io() {
            return Ok(0);
        }

        let covered = self.position >= self.buffer_start
            && self.position < self.buffer_start + self.buffer.len() as u64;

        if !covered {
            self.fill_buffer()?;
        }

        let offset = (self.position - self.buffer_start) as usize;
        if offset >= self.buffer.len() {
            // Buffer exhausted and server returned no more data: EOF.
            return Ok(0);
        }

        let available = self.buffer.len() - offset;
        let n = available.min(buf.len());
        buf[..n].copy_from_slice(&self.buffer[offset..offset + n]);
        self.position += n as u64;
        Ok(n)
    }
}

impl Seek for SubsonicStreamSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let base = match pos {
            SeekFrom::Start(p) => p,
            SeekFrom::End(offset) => {
                let len = self.length.ok_or_else(|| {
                    io::Error::new(io::ErrorKind::Unsupported, "unknown stream length")
                })?;
                len.saturating_add_signed(offset)
            }
            SeekFrom::Current(offset) => self.position.saturating_add_signed(offset),
        };
        self.position = base;
        Ok(self.position)
    }
}

impl MediaSource for SubsonicStreamSource {
    fn is_seekable(&self) -> bool {
        self.seekable
    }

    fn byte_len(&self) -> Option<u64> {
        self.length
    }
}

/// Extracts the total byte length from a `Content-Range: bytes 0-0/12345` header.
fn parse_content_range_total(value: &str) -> Option<u64> {
    value.split('/').nth(1)?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_content_range_total() {
        assert_eq!(parse_content_range_total("bytes 0-0/12345"), Some(12345));
        assert_eq!(parse_content_range_total("bytes 0-0/*"), None);
        assert_eq!(parse_content_range_total("garbage"), None);
    }
}
