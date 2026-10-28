//! An in-memory `SecretStore` for tests.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::secrets::{Secret, SecretStore};

#[derive(Default)]
pub struct FakeSecrets {
    values: Mutex<HashMap<String, Secret>>,
}

impl FakeSecrets {
    /// A store holding `secret` for `account`.
    pub fn with(account: &str, secret: &str) -> Self {
        let store = Self::default();
        store
            .values
            .lock()
            .unwrap()
            .insert(account.into(), Secret::new(secret));
        store
    }
}

impl SecretStore for FakeSecrets {
    fn get(&self, account: &str) -> Result<Option<Secret>, String> {
        Ok(self.values.lock().unwrap().get(account).cloned())
    }

    fn set(&self, account: &str, secret: &Secret) -> Result<(), String> {
        self.values
            .lock()
            .unwrap()
            .insert(account.into(), secret.clone());
        Ok(())
    }

    fn remove(&self, account: &str) -> Result<(), String> {
        self.values.lock().unwrap().remove(account);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACCOUNT: &str = "alex.morgan@example.com";

    #[test]
    fn a_stored_secret_reads_back_until_it_is_removed() {
        let store = FakeSecrets::default();
        assert!(store.get(ACCOUNT).unwrap().is_none());
        store.set(ACCOUNT, &Secret::new("token-1")).unwrap();
        assert_eq!(store.get(ACCOUNT).unwrap().unwrap().expose(), "token-1");
        store.remove(ACCOUNT).unwrap();
        assert!(store.get(ACCOUNT).unwrap().is_none());
    }

    #[test]
    fn setting_again_replaces_the_secret() {
        let store = FakeSecrets::with(ACCOUNT, "old");
        store.set(ACCOUNT, &Secret::new("new")).unwrap();
        assert_eq!(store.get(ACCOUNT).unwrap().unwrap().expose(), "new");
    }

    #[test]
    fn removing_a_missing_account_succeeds() {
        assert_eq!(FakeSecrets::default().remove("nobody"), Ok(()));
    }

    #[test]
    fn with_prefills_one_account() {
        let store = FakeSecrets::with(ACCOUNT, "token-1");
        assert_eq!(store.get(ACCOUNT).unwrap().unwrap().expose(), "token-1");
        assert!(store.get("other").unwrap().is_none());
    }
}
