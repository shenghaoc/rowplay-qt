// SPDX-License-Identifier: GPL-3.0-or-later
//! Bring-your-own-token storage boundary.
//!
//! The personal Concept2 API token is the only credential the app ever holds.
//! It must live in OS keychain-backed storage (Phase 3 wires the `keyring`
//! crate) and never in preferences, plain files, logs, fixtures or analytics.

use std::sync::Mutex;

/// Upper bound on token length; anything longer is rejected before storage.
pub const MAX_TOKEN_LEN: usize = 512;

/// A Concept2 personal API token.
///
/// The value is only reachable through [`SecretToken::expose`]. `Debug` prints
/// a placeholder, there is no `Display`, and the type is deliberately not
/// serialisable. The bytes are overwritten when the value is dropped.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretToken(Vec<u8>);

impl SecretToken {
    /// Validate and wrap a token: trimmed, non-empty, printable ASCII, bounded length.
    pub fn new(raw: &str) -> Result<Self, TokenStoreError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(TokenStoreError::Invalid("token is empty"));
        }
        if trimmed.len() > MAX_TOKEN_LEN {
            return Err(TokenStoreError::Invalid("token is too long"));
        }
        if !trimmed.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(TokenStoreError::Invalid(
                "token contains non-printable or non-ASCII characters",
            ));
        }
        Ok(SecretToken(trimmed.as_bytes().to_vec()))
    }

    /// The raw token, for the HTTP `Authorization` header only.
    #[must_use]
    pub fn expose(&self) -> &str {
        std::str::from_utf8(&self.0).expect("validated ASCII")
    }

    /// Length in bytes (safe to log).
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Always false for a constructed token.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Debug for SecretToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretToken([REDACTED])")
    }
}

impl Drop for SecretToken {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

/// Token store failures. Messages never include the token.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TokenStoreError {
    /// The token failed validation.
    #[error("invalid Concept2 token: {0}")]
    Invalid(&'static str),
    /// The backing store is unavailable (keychain locked, denied, …).
    #[error("token store unavailable: {0}")]
    Unavailable(String),
}

/// Keychain-style storage for the Concept2 token.
pub trait TokenStore: Send + Sync {
    /// The stored token, if any.
    fn load(&self) -> Result<Option<SecretToken>, TokenStoreError>;
    /// Store (replace) the token.
    fn save(&self, token: &SecretToken) -> Result<(), TokenStoreError>;
    /// Remove the token; succeeds when none is stored.
    fn clear(&self) -> Result<(), TokenStoreError>;
}

/// Process-local token store for tests and demo mode.
#[derive(Debug, Default)]
pub struct InMemoryTokenStore {
    token: Mutex<Option<SecretToken>>,
}

impl TokenStore for InMemoryTokenStore {
    fn load(&self) -> Result<Option<SecretToken>, TokenStoreError> {
        Ok(self.token.lock().expect("token lock").clone())
    }

    fn save(&self, token: &SecretToken) -> Result<(), TokenStoreError> {
        *self.token.lock().expect("token lock") = Some(token.clone());
        Ok(())
    }

    fn clear(&self) -> Result<(), TokenStoreError> {
        *self.token.lock().expect("token lock") = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_tokens() {
        assert!(SecretToken::new("").is_err());
        assert!(SecretToken::new("   ").is_err());
        assert!(SecretToken::new("has space").is_err());
        assert!(SecretToken::new("ünïcode").is_err());
        assert!(SecretToken::new(&"a".repeat(MAX_TOKEN_LEN + 1)).is_err());
        let token = SecretToken::new("  abc123DEF  ").unwrap();
        assert_eq!(token.expose(), "abc123DEF");
        assert_eq!(token.len(), 9);
        assert!(!token.is_empty());
    }

    #[test]
    fn debug_never_prints_the_token() {
        let token = SecretToken::new("supersecretvalue").unwrap();
        let debug = format!("{token:?}");
        assert!(!debug.contains("supersecretvalue"));
        assert!(debug.contains("REDACTED"));
    }

    #[test]
    fn in_memory_store_round_trips() {
        let store = InMemoryTokenStore::default();
        assert_eq!(store.load().unwrap(), None);
        let token = SecretToken::new("abc123").unwrap();
        store.save(&token).unwrap();
        assert_eq!(
            store.load().unwrap().as_ref().map(SecretToken::expose),
            Some("abc123")
        );
        store.clear().unwrap();
        assert_eq!(store.load().unwrap(), None);
        store.clear().unwrap();
    }

    #[test]
    fn errors_do_not_leak_tokens() {
        let err = TokenStoreError::Unavailable("keychain locked".into());
        assert_eq!(err.to_string(), "token store unavailable: keychain locked");
    }
}
