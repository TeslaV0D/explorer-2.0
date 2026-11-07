//! Registry access behind a trait: the real one on Windows, an in-memory one
//! for tests (and Linux) – the whole journal/reset logic runs against both.

use crate::model::{KeyPath, RegData};
use std::collections::BTreeMap;
use std::io;

pub trait Registry {
    fn get(&self, key: &KeyPath, name: &str) -> io::Result<Option<RegData>>;
    /// Creates missing keys on the way.
    fn set(&mut self, key: &KeyPath, name: &str, data: &RegData) -> io::Result<()>;
    /// Missing value/key = Ok.
    fn delete_value(&mut self, key: &KeyPath, name: &str) -> io::Result<()>;
    fn key_exists(&self, key: &KeyPath) -> io::Result<bool>;
    /// No values and no subkeys.
    fn key_is_empty(&self, key: &KeyPath) -> io::Result<bool>;
    /// Only an empty key; missing = Ok.
    fn delete_empty_key(&mut self, key: &KeyPath) -> io::Result<()>;
    /// Key with everything below it; missing = Ok. Only for our own namespace.
    fn delete_tree(&mut self, key: &KeyPath) -> io::Result<()>;
    /// Tell running programs that `HKCU\Environment` changed.
    fn notify_environment(&mut self) {}
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct MemKey {
    /// norm name → (name, value)
    values: BTreeMap<String, (String, RegData)>,
}

/// Case-insensitive in-memory registry (`==` compares the full content, which
/// is exactly the "snapshot before == snapshot after" check of R-001).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemRegistry {
    keys: BTreeMap<String, MemKey>,
    pub env_notifications: usize,
}

impl MemRegistry {
    pub fn new() -> MemRegistry {
        MemRegistry::default()
    }

    pub fn create_key(&mut self, key: &KeyPath) {
        for k in key.lineage() {
            self.keys.entry(k.norm()).or_default();
        }
    }

    /// Same content (ignores the notification counter).
    pub fn same_content(&self, other: &MemRegistry) -> bool {
        self.keys == other.keys
    }

    pub fn key_count(&self) -> usize {
        self.keys.len()
    }
}

fn denied(what: &str) -> io::Error {
    io::Error::other(what.to_string())
}

impl Registry for MemRegistry {
    fn get(&self, key: &KeyPath, name: &str) -> io::Result<Option<RegData>> {
        Ok(self
            .keys
            .get(&key.norm())
            .and_then(|k| k.values.get(&name.to_lowercase()))
            .map(|(_, d)| d.clone()))
    }

    fn set(&mut self, key: &KeyPath, name: &str, data: &RegData) -> io::Result<()> {
        self.create_key(key);
        let k = self.keys.get_mut(&key.norm()).expect("created");
        k.values
            .insert(name.to_lowercase(), (name.to_string(), data.clone()));
        Ok(())
    }

    fn delete_value(&mut self, key: &KeyPath, name: &str) -> io::Result<()> {
        if let Some(k) = self.keys.get_mut(&key.norm()) {
            k.values.remove(&name.to_lowercase());
        }
        Ok(())
    }

    fn key_exists(&self, key: &KeyPath) -> io::Result<bool> {
        Ok(self.keys.contains_key(&key.norm()))
    }

    fn key_is_empty(&self, key: &KeyPath) -> io::Result<bool> {
        let n = key.norm();
        let Some(k) = self.keys.get(&n) else {
            return Ok(true);
        };
        let prefix = format!("{n}\\");
        Ok(k.values.is_empty() && !self.keys.keys().any(|o| o.starts_with(&prefix)))
    }

    fn delete_empty_key(&mut self, key: &KeyPath) -> io::Result<()> {
        if !self.key_is_empty(key)? {
            return Err(denied("key not empty"));
        }
        self.keys.remove(&key.norm());
        Ok(())
    }

    fn delete_tree(&mut self, key: &KeyPath) -> io::Result<()> {
        let n = key.norm();
        let prefix = format!("{n}\\");
        self.keys.retain(|k, _| k != &n && !k.starts_with(&prefix));
        Ok(())
    }

    fn notify_environment(&mut self) {
        self.env_notifications += 1;
    }
}
