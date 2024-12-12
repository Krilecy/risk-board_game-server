// player.rs
use crate::card::Card;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Player {
    pub id: usize,
    pub name: String,
    pub territories: HashSet<String>,
    pub armies: HashMap<String, u16>,
    pub cards: Vec<Card>,
    pub army_supply: u16,
    pub total_armies: u16,
}

impl Player {
    pub fn new(id: usize, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            territories: HashSet::new(),
            armies: HashMap::new(),
            cards: Vec::new(),
            army_supply: 0,
            total_armies: 0,
        }
    }

    pub fn add_territory(&mut self, territory: &str) {
        self.territories.insert(territory.to_string());
        self.armies.insert(territory.to_string(), 0);
    }

    pub fn remove_territory(&mut self, territory: &str) {
        self.territories.remove(territory);
        self.armies.remove(territory);
    }

    pub fn reinforce(&mut self, territory: &str, num_armies: u16) {
        *self.armies.entry(territory.to_string()).or_insert(0) += num_armies;
    }

    pub fn remove_armies(&mut self, territory: &str, num_armies: u16) {
        if let Some(armies) = self.armies.get_mut(territory) {
            *armies = armies.saturating_sub(num_armies);
        }
    }

    pub fn get_armies(&self, territory: &str) -> u16 {
        *self.armies.get(territory).unwrap_or(&0)
    }

    pub fn set_armies(&mut self, territory: &str, armies: u16) {
        self.armies.insert(territory.to_string(), armies);
    }

    pub fn fortify(&mut self, from: &str, to: &str, num_armies: u16) {
        if let Some(from_armies) = self.armies.get_mut(from) {
            if *from_armies >= num_armies {
                *from_armies -= num_armies;
                *self.armies.entry(to.to_string()).or_insert(0) += num_armies;
            }
        }
    }

    pub fn calculate_total_armies(&mut self) {
        self.total_armies = self.armies.values().sum();
    }
}