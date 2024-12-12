// card.rs
use crate::board::Board;
use crate::game::Game;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Card {
    pub territory: Option<String>,
    pub kind: CardKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CardKind {
    Infantry,
    Cavalry,
    Artillery,
    Joker,
}

impl Card {
    pub fn new(territory: Option<String>, kind: CardKind) -> Self {
        Self { territory, kind }
    }

    pub fn get_type(&self) -> &CardKind {
        &self.kind
    }
}

impl Game {
    pub fn create_deck(board: &Board) -> Vec<Card> {
        let mut deck = Vec::new();
        let mut rng = thread_rng();
        let mut card_types = vec![CardKind::Infantry, CardKind::Cavalry, CardKind::Artillery];

        for territory_name in board.territories.keys() {
            card_types.shuffle(&mut rng);
            deck.push(Card::new(
                Some(territory_name.clone()),
                card_types[0].clone(),
            ));
        }

        // Add 2 joker cards
        deck.push(Card::new(None, CardKind::Joker));
        deck.push(Card::new(None, CardKind::Joker));

        deck.shuffle(&mut rng);
        deck
    }

    pub fn is_valid_trade(&self, card_kinds: &[&CardKind]) -> bool {
        let infantry_count = card_kinds.iter().filter(|&&kind| kind == &CardKind::Infantry).count();
        let cavalry_count = card_kinds.iter().filter(|&&kind| kind == &CardKind::Cavalry).count();
        let artillery_count = card_kinds.iter().filter(|&&kind| kind == &CardKind::Artillery).count();
        let joker_count = card_kinds.iter().filter(|&&kind| kind == &CardKind::Joker).count();
    
        if infantry_count == 3 || cavalry_count == 3 || artillery_count == 3 {
            return true;
        }
    
        if infantry_count == 1 && cavalry_count == 1 && artillery_count == 1 {
            return true;
        }
    
        if joker_count > 0 {
            if infantry_count + cavalry_count + artillery_count + joker_count == 3 {
                return true;
            }
        }
    
        false
    }

    pub fn trade_cards(
        &mut self,
        player_id: usize,
        card_indices: Vec<usize>,
    ) -> Result<u16, String> {
        // First, validate the trade with an immutable borrow
        let card_kinds = {
            let player = self.players.get(player_id).ok_or("Invalid player ID")?;
            if card_indices.len() != 3 {
                return Err(format!(
                    "You must trade exactly 3 cards. Provided indices: {:?}",
                    card_indices
                ));
            }
            let mut card_kinds = vec![];
            for &index in &card_indices {
                if index >= player.cards.len() {
                    return Err(format!(
                        "Invalid card index: {}. Provided indices: {:?}",
                        index, card_indices
                    ));
                }
                card_kinds.push(&player.cards[index].kind);
            }
            card_kinds
        };

        // Validate trade
        if !self.is_valid_trade(&card_kinds) {
            return Err(format!(
                "Invalid card combination: {:?}. Provided indices: {:?}",
                card_kinds, card_indices
            ));
        }

        // Calculate bonus armies
        let bonus_armies = calculate_trade_in_bonus(&card_kinds)?;

        // Perform the trade with a mutable borrow
        let player = self.players.get_mut(player_id).ok_or("Invalid player ID")?;
        let mut territory_to_reinforce: Option<String> = None;
        for &index in card_indices.iter().rev() {
            let card = player.cards.remove(index);
            if let Some(ref territory) = card.territory {
                if territory_to_reinforce.is_none() && player.territories.contains(territory) {
                    territory_to_reinforce = Some(territory.clone());
                }
            }
            self.discard_pile.push(card);
        }
        if let Some(territory) = territory_to_reinforce {
            player.reinforce(&territory, 2);
        }

        self.reinforcement_armies += bonus_armies;
        Ok(bonus_armies)
    }
}

pub fn calculate_trade_in_bonus(card_kinds: &[&CardKind]) -> Result<u16, String> {
    let infantry_count = card_kinds.iter().filter(|&&kind| kind == &CardKind::Infantry).count();
    let cavalry_count = card_kinds.iter().filter(|&&kind| kind == &CardKind::Cavalry).count();
    let artillery_count = card_kinds.iter().filter(|&&kind| kind == &CardKind::Artillery).count();
    let joker_count = card_kinds.iter().filter(|&&kind| kind == &CardKind::Joker).count();

    if infantry_count == 3 {
        Ok(4)
    } else if cavalry_count == 3 {
        Ok(6)
    } else if artillery_count == 3 {
        Ok(8)
    } else if infantry_count == 1 && cavalry_count == 1 && artillery_count == 1 {
        Ok(10)
    } else if joker_count > 0 {
        let valid_sets = vec![
            infantry_count + joker_count == 3,
            cavalry_count + joker_count == 3,
            artillery_count + joker_count == 3,
            infantry_count + cavalry_count + joker_count == 3,
            infantry_count + artillery_count + joker_count == 3,
            cavalry_count + artillery_count + joker_count == 3,
        ];
        if valid_sets.into_iter().any(|v| v) {
            Ok(10)
        } else {
            Err("Invalid combination of cards.".to_string())
        }
    } else {
        Err("Invalid combination of cards.".to_string())
    }
}