// territory.rs
use crate::game::Game;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Territory {
    pub name: String,
    pub continent: String,
    pub adjacent_territories: HashSet<String>,
}

impl Territory {
    pub fn new(name: &str, continent: &str) -> Self {
        Self {
            name: name.to_string(),
            continent: continent.to_string(),
            adjacent_territories: HashSet::new(),
        }
    }

    pub fn add_adjacent(&mut self, adjacent: &str) {
        self.adjacent_territories.insert(adjacent.to_string());
    }

    pub fn is_adjacent(&self, territory: &str) -> bool {
        self.adjacent_territories.contains(territory)
    }

    pub fn get_continent(&self) -> &str {
        &self.continent
    }
}

impl Game {
    pub fn are_territories_connected(&self, player_id: usize, from: &str, to: &str) -> bool {
        let player = &self.players[player_id];
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![from];

        while let Some(territory) = stack.pop() {
            if territory == to {
                return true;
            }
            if !visited.insert(territory) {
                continue;
            }
            let adjacents = self
                .board
                .get_territory(territory)
                .unwrap()
                .adjacent_territories
                .iter();
            for adjacent in adjacents {
                if player.territories.contains(adjacent) {
                    stack.push(adjacent);
                }
            }
        }

        false
    }
}
