// SPDX-License-Identifier: GPL-3.0-or-later
//! Bring-your-own-token storage boundary.
//!
//! The personal Concept2 API token is the only credential the app ever holds.
//! It lives in OS keychain-backed storage ([`KeyringTokenStore`], the `keyring`
//! crate) and never in preferences, plain files, logs, fixtures or analytics.

use std::sync::Mutex;

use rowplay_core::privacy::PrivacySafeLogger;

use crate::logging::logger;

/// Service name recorded in the OS credential store (macOS Keychain, Windows
/// Credential Manager, Linux Secret Service).
pub const KEYCHAIN_SERVICE: &str = "com.rowplay-qt.concept2-token";

/// Account name recorded in the OS credential store.
pub const KEYCHAIN_ACCOUNT: &str = "default";

/// Environment variable that opts the real-keychain tests in.
///
/// CI has no Secret Service and macOS would prompt, so the tests that touch a
/// live credential store run only when this is set to `1`.
pub const KEYRING_TEST_ENV: &str = "ROWPLAY_KEYRING_TESTS";

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

/// OS keychain-backed token store (Studio `KeychainTokenStore`).
///
/// Uses `keyring`, whose platform backends are selected by Cargo features:
/// macOS Keychain (`apple-native`), Windows Credential Manager
/// (`windows-native`) and the Linux/BSD Secret Service
/// (`sync-secret-service`). Every target names its backend explicitly, because
/// keyring has no default features and would otherwise substitute an in-memory
/// mock store — `the_default_credential_store_is_not_the_mock` guards that.
///
/// Note: `keyring::Entry::get_password` hands back an ordinary `String`, so the
/// stored token exists in a non-zeroed buffer for the duration of the load
/// call; [`SecretToken`] copies it into memory it wipes on drop. Compiling
/// keyring's `zeroize` usage into our wrapper is not possible through that API.
pub struct KeyringTokenStore {
    entry: keyring::Entry,
    logger: PrivacySafeLogger<'static>,
}

impl std::fmt::Debug for KeyringTokenStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KeyringTokenStore")
    }
}

impl KeyringTokenStore {
    /// A store for the app's Concept2 entry.
    pub fn new() -> Result<Self, TokenStoreError> {
        Self::with_service(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
    }

    /// A store for an explicit service/account pair (used by the opt-in tests).
    pub fn with_service(service: &str, account: &str) -> Result<Self, TokenStoreError> {
        let entry = keyring::Entry::new(service, account)
            .map_err(|error| TokenStoreError::Unavailable(backend_class(&error).to_owned()))?;
        Ok(KeyringTokenStore {
            entry,
            logger: logger("token-store"),
        })
    }

    /// Log a redacted line and convert a backend failure.
    fn fail(&self, action: &str, error: &keyring::Error) -> TokenStoreError {
        let class = backend_class(error);
        self.logger.warn(
            "Concept2 token store operation failed",
            &[
                &action as &dyn std::fmt::Display,
                &class as &dyn std::fmt::Display,
            ],
        );
        TokenStoreError::Unavailable(class.to_owned())
    }
}

impl TokenStore for KeyringTokenStore {
    fn load(&self) -> Result<Option<SecretToken>, TokenStoreError> {
        match self.entry.get_password() {
            Ok(password) => SecretToken::new(&password).map(Some),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(self.fail("load", &error)),
        }
    }

    fn save(&self, token: &SecretToken) -> Result<(), TokenStoreError> {
        self.entry
            .set_password(token.expose())
            .map_err(|error| self.fail("save", &error))
    }

    fn clear(&self) -> Result<(), TokenStoreError> {
        match self.entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(self.fail("clear", &error)),
        }
    }
}

/// Coarse, privacy-safe classification of a keyring failure.
///
/// The underlying error is never rendered: `keyring::Error::Ambiguous` formats
/// its credentials with `Debug`, and a peer-controlled store could put anything
/// there. The class says enough to act on.
fn backend_class(error: &keyring::Error) -> &'static str {
    match error {
        keyring::Error::PlatformFailure(_) => "platform credential store failure",
        keyring::Error::NoStorageAccess(_) => "credential store is not accessible",
        keyring::Error::NoEntry => "no stored credential",
        keyring::Error::BadEncoding(_) => "stored credential is not UTF-8",
        keyring::Error::TooLong(_, _) => "credential attribute is too long",
        keyring::Error::Invalid(_, _) => "credential attribute is invalid",
        keyring::Error::Ambiguous(_) => "stored credential is ambiguous",
        _ => "unknown credential store failure",
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

    /// The regression guard for keyring's silent mock fallback: keyring 3 has
    /// no default features, and without a backend feature the crate swaps in an
    /// in-memory store that forgets the token when the process exits. Every
    /// native store reports `UntilDelete`; only the mock reports `EntryOnly`.
    #[test]
    fn the_default_credential_store_is_not_the_mock() {
        use keyring::credential::CredentialPersistence;

        let builder = keyring::default::default_credential_builder();
        assert!(
            !matches!(builder.persistence(), CredentialPersistence::EntryOnly),
            "keyring fell back to its in-memory mock store: the native backend \
             feature for this target is missing from Cargo.toml"
        );
        assert!(
            matches!(builder.persistence(), CredentialPersistence::UntilDelete),
            "expected a persistent credential store"
        );
    }

    /// Building an entry performs no store access, so this runs everywhere.
    #[test]
    fn the_keyring_store_constructs_with_the_app_identifier() {
        let store = KeyringTokenStore::new().expect("keyring entry");
        assert_eq!(format!("{store:?}"), "KeyringTokenStore");
    }

    #[test]
    fn backend_classes_are_privacy_safe() {
        for error in [
            keyring::Error::NoEntry,
            keyring::Error::BadEncoding(vec![b's', b'e', b'c']),
            keyring::Error::TooLong("service".into(), 32),
            keyring::Error::Invalid("service".into(), "cannot be empty".into()),
        ] {
            let class = backend_class(&error);
            let message = TokenStoreError::Unavailable(class.to_owned()).to_string();
            assert!(!message.contains("sec"), "{message}");
            assert!(message.len() < 80, "{message}");
        }
    }

    fn keyring_tests_enabled() -> bool {
        std::env::var(KEYRING_TEST_ENV).is_ok_and(|value| value == "1")
    }

    /// Opt-in round trip against the real OS keychain (`ROWPLAY_KEYRING_TESTS=1`).
    ///
    /// The account is per-process so the test can never clobber a real token.
    #[test]
    fn keychain_round_trip_is_opt_in() {
        if !keyring_tests_enabled() {
            return;
        }
        let account = format!("rowplay-qt-test-{}", std::process::id());
        let store = KeyringTokenStore::with_service(KEYCHAIN_SERVICE, &account).unwrap();
        store.clear().unwrap();
        assert_eq!(store.load().unwrap(), None);

        let token = SecretToken::new("test-token-abc123").unwrap();
        store.save(&token).unwrap();
        assert_eq!(
            store.load().unwrap().as_ref().map(SecretToken::expose),
            Some("test-token-abc123")
        );

        // A second store instance proves the value really reached the OS
        // credential store: keyring's mock builds a fresh, empty credential per
        // entry, so this assertion fails if a backend feature is missing.
        let reopened = KeyringTokenStore::with_service(KEYCHAIN_SERVICE, &account).unwrap();
        assert_eq!(
            reopened.load().unwrap().as_ref().map(SecretToken::expose),
            Some("test-token-abc123"),
            "the token did not survive a second entry"
        );

        store
            .save(&SecretToken::new("replaced-token").unwrap())
            .unwrap();
        assert_eq!(
            store.load().unwrap().as_ref().map(SecretToken::expose),
            Some("replaced-token")
        );

        store.clear().unwrap();
        store.clear().unwrap();
        assert_eq!(store.load().unwrap(), None);
    }
}
