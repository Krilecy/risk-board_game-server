// board.rs
use crate::continent::Continent;
use crate::player::Player;
use crate::territory::Territory;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Board {
    pub territories: HashMap<String, Territory>,
    pub continents: HashMap<String, Continent>,
}

impl Board {
    pub fn new() -> Self {
        Self {
            territories: HashMap::new(),
            continents: HashMap::new(),
        }
    }

    pub fn add_territory(&mut self, territory: Territory) {
        self.territories.insert(territory.name.clone(), territory);
    }

    pub fn add_continent(&mut self, continent: Continent) {
        self.continents.insert(continent.name.clone(), continent);
    }

    pub fn get_territory(&self, name: &str) -> Option<&Territory> {
        self.territories.get(name)
    }

    pub fn get_continent(&self, name: &str) -> Option<&Continent> {
        self.continents.get(name)
    }

    pub fn shuffle_and_distribute_territories(&mut self, players: &mut Vec<Player>) {
        let mut territories: Vec<&String> = self.territories.keys().collect();
        let mut rng = rand::thread_rng();
        territories.shuffle(&mut rng);

        let mut continent_territory_map: HashMap<String, Vec<&String>> = HashMap::new();

        // Map territories to their continents
        for territory in &territories {
            let continent_name = &self.territories[*territory].continent;
            continent_territory_map.entry(continent_name.clone())
                .or_insert(Vec::new())
                .push(territory);
        }

        // Distribute territories ensuring no player gets all territories of a continent
        let mut player_index = 0;
        for (_, continent_territories) in &mut continent_territory_map {
            continent_territories.shuffle(&mut rng);

            for territory in continent_territories {
                players[player_index].add_territory(territory);
                players[player_index].reinforce(territory, 1);
                player_index = (player_index + 1) % players.len();
            }
        }

        // Distribute remaining armies
        let mut remaining_armies = players.len() as u16 * 5;  // Example: Each player gets 5 additional armies to distribute
        while remaining_armies > 0 {
            for player in players.iter_mut() {
                if remaining_armies == 0 {
                    break;
                }
                let territories: Vec<String> = player.territories.iter().cloned().collect();
                for territory in territories {
                    player.reinforce(&territory, 1);
                    remaining_armies -= 1;
                    if remaining_armies == 0 {
                        break;
                    }
                }
            }
        }
    }
}
