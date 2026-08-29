use std::{
    ffi::OsStr,
    fs::File,
    path::Path,
    sync::{Arc, LazyLock},
};

use symphonia::core::io::MediaSource;

// use tokio rwlock because it is write-preferring
use tokio::sync::RwLock;
use tracing::info;

use crate::media::traits::{MediaProvider, MediaProviderFeatures, MediaStream};

type LookupTableInnerType = Arc<RwLock<Vec<Box<dyn MediaProvider>>>>;

pub static LOOKUP_TABLE: LazyLock<LookupTableInnerType> =
    LazyLock::new(|| Arc::new(RwLock::new(Vec::new())));

pub fn add_provider(provider: Box<dyn MediaProvider>) {
    info!(
        "Attempting to register media provider \"{}\"",
        provider.name()
    );

    let mut write = LOOKUP_TABLE.blocking_write();
    write.push(provider);
}

#[allow(clippy::borrowed_box)]
fn provider_can_read(
    path: &Path,
    required_features: MediaProviderFeatures,
    provider: &Box<dyn MediaProvider>,
) -> anyhow::Result<bool> {
    // mime-types are more reliable but windows is too slow to use them
    // so now we only use extensions
    if let Some(ext) = path.extension().and_then(|v| v.to_str())
        && provider_can_read_ext(ext, required_features, provider)
    {
        return Ok(true);
    }

    Ok(false)
}

/// Whether a provider can read a given file extension with the required features.
#[allow(clippy::borrowed_box)]
fn provider_can_read_ext(
    ext: &str,
    required_features: MediaProviderFeatures,
    provider: &Box<dyn MediaProvider>,
) -> bool {
    provider
        .supported_extensions()
        .iter()
        .any(|v| v.eq_ignore_ascii_case(ext))
        && provider.supported_features() & required_features == required_features
}

pub fn can_be_read(path: &Path, required_features: MediaProviderFeatures) -> anyhow::Result<bool> {
    let read = LOOKUP_TABLE.blocking_read();
    for provider in read.iter() {
        if provider_can_read(path, required_features, provider)? {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn try_open_media(
    path: &Path,
    required_features: MediaProviderFeatures,
) -> anyhow::Result<Option<Box<dyn MediaStream>>> {
    let read = LOOKUP_TABLE.blocking_read();
    let mut last_error = None;

    for provider in read.iter() {
        if provider_can_read(path, required_features, provider)? {
            let file = File::open(path)?;
            match provider.open(Box::new(file), path.extension()) {
                Ok(stream) => return Ok(Some(stream)),
                Err(e) => last_error = Some(e),
            }
        }
    }

    if let Some(e) = last_error {
        Err(e.into())
    } else {
        Ok(None)
    }
}

/// Opens a media stream from an arbitrary `Read + Seek` source (e.g. an
/// HTTP-backed range stream), selecting a provider by file extension.
///
/// The source is consumed by value, so only a single provider may attempt to
/// open it. Extension + feature filtering selects a unique provider in
/// practice (Symphonia is the sole decoder provider).
pub fn try_open_media_source(
    source: Box<dyn MediaSource>,
    ext: Option<&OsStr>,
    required_features: MediaProviderFeatures,
) -> anyhow::Result<Option<Box<dyn MediaStream>>> {
    let Some(ext) = ext.and_then(|e| e.to_str()) else {
        return Ok(None);
    };

    let read = LOOKUP_TABLE.blocking_read();
    for provider in read.iter() {
        if provider_can_read_ext(ext, required_features, provider) {
            return match provider.open(source, Some(OsStr::new(ext))) {
                Ok(stream) => Ok(Some(stream)),
                Err(e) => Err(e.into()),
            };
        }
    }

    Ok(None)
}
