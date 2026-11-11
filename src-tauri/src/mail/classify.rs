//! The user\'s corrections of the person/bulk detection, per sender address.

use std::collections::HashMap;
use std::sync::RwLock;

/// `true` = bulk ("Rauschen"), `false` = person. Keys are lowercase addresses.
#[derive(Default)]
pub struct SenderKinds(RwLock<HashMap<String, bool>>);

impl SenderKinds {
    pub fn new(map: HashMap<String, bool>) -> Self {
        Self(RwLock::new(map.into_iter().map(|(k, v)| (k.to_lowercase(), v)).collect()))
    }

    pub fn get(&self, addr: &str) -> Option<bool> {
        self.0.read().unwrap().get(&addr.to_lowercase()).copied()
    }

    /// `None` removes the correction.
    pub fn set(&self, addr: &str, bulk: Option<bool>) {
        let mut map = self.0.write().unwrap();
        match bulk {
            Some(b) => map.insert(addr.to_lowercase(), b),
            None => map.remove(&addr.to_lowercase()),
        };
    }
}
