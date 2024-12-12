// continent.rs
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Continent {
    pub name: String,                 // Make this field public
    pub bonus_armies: u16,             // Make this field public
    pub territories: HashSet<String>, // Make this field public
}

impl Continent {
    pub fn new(name: &str, bonus_armies: u16) -> Self {
        Self {
            name: name.to_string(),
            bonus_armies,
            territories: HashSet::new(),
        }
    }

    pub fn add_territory(&mut self, territory: &str) {
        self.territories.insert(territory.to_string());
    }

    pub fn get_bonus(&self) -> u16 {
        self.bonus_armies
    }
}
