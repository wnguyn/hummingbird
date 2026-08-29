//! Subsonic token authentication.
//!
//! Subsonic authenticates every request with three query parameters:
//! `u` (username), `s` (a random salt), and `t` (the MD5 of `password + salt`).
//! The plaintext password never appears in a URL.

/// Computes the authentication token for a request.
///
/// `t = md5(password + salt)`.
pub fn token(password: &str, salt: &str) -> String {
    let mut context = md5::Context::new();
    context.consume(password.as_bytes());
    context.consume(salt.as_bytes());
    format!("{:x}", context.compute())
}

/// Generates a fresh random salt.
pub fn salt() -> String {
    format!("{:016x}", rand::random::<u64>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_md5_of_password_plus_salt() {
        // Verified against the Subsonic spec example: md5("sesame" + "c19b2d").
        let digest = token("sesame", "c19b2d");
        assert_eq!(digest, "26719a1196d2a940705a59634eb18eab");
    }

    #[test]
    fn token_differs_for_different_salts() {
        assert_ne!(token("secret", "aaaa"), token("secret", "bbbb"));
    }

    #[test]
    fn salt_is_hex_and_non_empty() {
        let s = salt();
        assert_eq!(s.len(), 16);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
