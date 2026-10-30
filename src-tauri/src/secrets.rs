//! Secrets in the login Keychain: generic passwords under the app's name (`APP_NAME`), so the debug build
//! keeps its own. Never in files, logs or the webview: no command returns a secret.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use zeroize::Zeroizing;

use crate::lock::MutexExt;

/// A secret value; `Debug` never shows it and the memory is wiped when dropped.
#[derive(Clone)]
pub struct Secret(Zeroizing<String>);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(Zeroizing::new(value.into()))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(…)")
    }
}

pub trait SecretStore: Send + Sync {
    fn get(&self, account: &str) -> Result<Option<Secret>, String>;
    fn set(&self, account: &str, secret: &Secret) -> Result<(), String>;
    /// Removing what isn't there succeeds.
    fn remove(&self, account: &str) -> Result<(), String>;
}

/// Keeps what `inner` answered in memory, so a request doesn't read the Keychain again. Writes go through and
/// update it; `forget` drops it, so the next read asks `inner`. A failed read is not kept.
pub struct Remembered {
    inner: Arc<dyn SecretStore>,
    known: Mutex<HashMap<String, Option<Secret>>>,
}

impl Remembered {
    pub fn new(inner: Arc<dyn SecretStore>) -> Self {
        Self {
            inner,
            known: Mutex::default(),
        }
    }

    pub fn forget(&self) {
        self.known.lock_unpoisoned().clear();
    }
}

impl SecretStore for Remembered {
    fn get(&self, account: &str) -> Result<Option<Secret>, String> {
        if let Some(known) = self.known.lock_unpoisoned().get(account) {
            return Ok(known.clone());
        }
        let secret = self.inner.get(account)?;
        self.known
            .lock_unpoisoned()
            .insert(account.into(), secret.clone());
        Ok(secret)
    }

    fn set(&self, account: &str, secret: &Secret) -> Result<(), String> {
        self.known.lock_unpoisoned().remove(account);
        self.inner.set(account, secret)?;
        self.known
            .lock_unpoisoned()
            .insert(account.into(), Some(secret.clone()));
        Ok(())
    }

    fn remove(&self, account: &str) -> Result<(), String> {
        self.known.lock_unpoisoned().remove(account);
        self.inner.remove(account)?;
        self.known.lock_unpoisoned().insert(account.into(), None);
        Ok(())
    }
}

const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;

pub struct Keychain {
    service: &'static str,
}

impl Keychain {
    pub fn new(service: &'static str) -> Self {
        Self { service }
    }
}

impl SecretStore for Keychain {
    fn get(&self, account: &str) -> Result<Option<Secret>, String> {
        match security_framework::passwords::get_generic_password(self.service, account) {
            Ok(bytes) => String::from_utf8(bytes)
                .map(|text| Some(Secret::new(text)))
                .map_err(|_| "The stored secret is not text".into()),
            Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn set(&self, account: &str, secret: &Secret) -> Result<(), String> {
        security_framework::passwords::set_generic_password(
            self.service,
            account,
            secret.expose().as_bytes(),
        )
        .map_err(|e| e.to_string())
    }

    fn remove(&self, account: &str) -> Result<(), String> {
        match security_framework::passwords::delete_generic_password(self.service, account) {
            Err(e) if e.code() != ERR_SEC_ITEM_NOT_FOUND => Err(e.to_string()),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::FakeSecrets;

    #[test]
    fn debug_output_hides_the_value() {
        assert_eq!(format!("{:?}", Secret::new("hunter2")), "Secret(…)");
    }

    #[test]
    fn expose_returns_the_value() {
        assert_eq!(Secret::new("hunter2").expose(), "hunter2");
    }

    const ACCOUNT: &str = "alex.morgan@example.com";

    fn remembered(inner: &Arc<FakeSecrets>) -> Remembered {
        Remembered::new(inner.clone())
    }

    fn value(store: &impl SecretStore) -> Option<String> {
        store
            .get(ACCOUNT)
            .unwrap()
            .map(|secret| secret.expose().to_string())
    }

    #[test]
    fn a_remembered_secret_is_read_from_the_inner_store_first() {
        let inner = Arc::new(FakeSecrets::with(ACCOUNT, "token-1"));
        assert_eq!(value(&remembered(&inner)), Some("token-1".into()));
    }

    #[test]
    fn a_remembered_secret_does_not_see_changes_made_in_the_inner_store_until_forgotten() {
        let inner = Arc::new(FakeSecrets::with(ACCOUNT, "token-1"));
        let store = remembered(&inner);
        assert_eq!(value(&store), Some("token-1".into()));

        inner.set(ACCOUNT, &Secret::new("token-2")).unwrap();
        assert_eq!(value(&store), Some("token-1".into()));

        store.forget();
        assert_eq!(value(&store), Some("token-2".into()));
    }

    #[test]
    fn a_missing_secret_is_remembered_as_missing() {
        let inner = Arc::new(FakeSecrets::default());
        let store = remembered(&inner);
        assert_eq!(value(&store), None);

        inner.set(ACCOUNT, &Secret::new("token-1")).unwrap();
        assert_eq!(value(&store), None);

        store.forget();
        assert_eq!(value(&store), Some("token-1".into()));
    }

    #[test]
    fn setting_goes_through_to_the_inner_store_and_is_remembered() {
        let inner = Arc::new(FakeSecrets::with(ACCOUNT, "old"));
        let store = remembered(&inner);
        assert_eq!(value(&store), Some("old".into()));

        store.set(ACCOUNT, &Secret::new("new")).unwrap();

        assert_eq!(value(&store), Some("new".into()));
        assert_eq!(value(&*inner), Some("new".into()));
    }

    #[test]
    fn removing_goes_through_to_the_inner_store_and_is_remembered() {
        let inner = Arc::new(FakeSecrets::with(ACCOUNT, "token-1"));
        let store = remembered(&inner);
        assert_eq!(value(&store), Some("token-1".into()));

        store.remove(ACCOUNT).unwrap();

        assert_eq!(value(&store), None);
        assert_eq!(value(&*inner), None);
    }

    #[test]
    fn secrets_of_other_accounts_are_remembered_separately() {
        let inner = Arc::new(FakeSecrets::with(ACCOUNT, "token-1"));
        let store = remembered(&inner);
        store
            .set("sam.lee@example.com", &Secret::new("token-2"))
            .unwrap();

        assert_eq!(value(&store), Some("token-1".into()));
        assert_eq!(
            store.get("sam.lee@example.com").unwrap().unwrap().expose(),
            "token-2"
        );
    }

    struct FailsOnce(Mutex<bool>);

    impl SecretStore for FailsOnce {
        fn get(&self, _: &str) -> Result<Option<Secret>, String> {
            let mut failed = self.0.lock().unwrap();
            if *failed {
                Ok(Some(Secret::new("token-1")))
            } else {
                *failed = true;
                Err("The Keychain is locked".into())
            }
        }

        fn set(&self, _: &str, _: &Secret) -> Result<(), String> {
            Ok(())
        }

        fn remove(&self, _: &str) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn a_failed_read_is_not_remembered() {
        let store = Remembered::new(Arc::new(FailsOnce(Mutex::new(false))));

        assert_eq!(store.get(ACCOUNT).unwrap_err(), "The Keychain is locked");
        assert_eq!(value(&store), Some("token-1".into()));
    }
}
