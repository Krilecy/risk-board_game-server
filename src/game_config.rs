use crate::board::Board;
use crate::card::{Card, CardKind};
use crate::continent::Continent;
use crate::player::Player;
use crate::territory::Territory;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub players: Vec<PlayerConfig>,
    pub territories: Vec<TerritoryConfig>,
    pub continents: Vec<ContinentConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerConfig {
    pub id: usize,
    pub name: String,
    pub territories: Vec<PlayerTerritoryConfig>,
    pub cards: Vec<CardConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerTerritoryConfig {
    pub name: String,
    pub armies: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardConfig {
    pub territory: Option<String>,
    pub kind: CardKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerritoryConfig {
    pub name: String,
    pub continent: String,
    pub adjacent_territories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinentConfig {
    pub name: String,
    pub bonus_armies: u16,
    pub territories: Vec<String>,
}

impl GameConfig {
    pub fn to_board_and_players(&self) -> (Board, Vec<Player>) {
        let mut board = Board::new();
        let mut players = Vec::new();
        let mut all_territories = HashSet::new();
        let mut assigned_territories = HashSet::new();
        let mut duplicate_territories = HashSet::new();

        for continent_config in &self.continents {
            let mut continent = Continent::new(&continent_config.name, continent_config.bonus_armies);
            for territory_name in &continent_config.territories {
                continent.add_territory(territory_name);
                all_territories.insert(territory_name.clone());
            }
            board.add_continent(continent);
        }

        for territory_config in &self.territories {
            let mut territory = Territory::new(&territory_config.name, &territory_config.continent);
            for adjacent in &territory_config.adjacent_territories {
                territory.add_adjacent(adjacent);
            }
            board.add_territory(territory);
        }

        for player_config in &self.players {
            let mut player = Player::new(player_config.id, &player_config.name);
            for territory in &player_config.territories {
                if !assigned_territories.insert(territory.name.clone()) {
                    duplicate_territories.insert(territory.name.clone());
                }
                player.add_territory(&territory.name);
                player.set_armies(&territory.name, territory.armies);
            }
            for card in &player_config.cards {
                let card = Card::new(card.territory.clone(), card.kind.clone());
                player.cards.push(card);
            }
            players.push(player);
        }

        // Assert no duplicate territories
        assert!(duplicate_territories.is_empty(), "Duplicate territories found: {:?}", duplicate_territories);

        // Assert all territories are assigned
        for territory in &all_territories {
            assert!(assigned_territories.contains(territory), "Territory not assigned: {}", territory);
        }

        (board, players)
    }

    pub fn load_from_file(filename: &str) -> Result<Self, std::io::Error> {
        let data = std::fs::read_to_string(filename)?;
        let config: GameConfig = serde_json::from_str(&data)?;
        Ok(config)
    }
}
