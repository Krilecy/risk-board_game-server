// turn_phase.rs
use crate::game::Game;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum TurnPhase {
    Reinforce,
    Attack,
    Fortify,
    MoveArmies,
    GameOver
}

impl Game {
    pub fn start_turn(&mut self) {
        self.reinforcement_armies = self.calculate_reinforcements(self.current_turn);
        self.initial_reinforcement_armies = self.reinforcement_armies;
        self.conquered_territory = false;
        self.turn_phase = TurnPhase::Reinforce;
    }

    pub fn advance_phase(&mut self) {
        match self.turn_phase {
            TurnPhase::Reinforce => {
                if self.reinforcement_armies == 0 {
                    self.turn_phase = TurnPhase::Attack;
                }
            }
            TurnPhase::Attack => {
                self.turn_phase = TurnPhase::Fortify;
            }
            TurnPhase::Fortify => {
                self.end_turn();
            }
            _ => {}
        }
    }

    pub fn end_turn(&mut self) {
        if self.conquered_territory {
            if let Some(card) = self.deck.pop() {
                self.players[self.current_turn].cards.push(card);
            }
        }

        // Find the index of the current player in the active_players list
        if let Some(current_index) = self.active_players.iter().position(|&p| p == self.current_turn) {
            // Move to the next player in the active_players list
            let next_index = (current_index + 1) % self.active_players.len();
            self.current_turn = self.active_players[next_index];

            // Increment the round count if we completed a full round
            if next_index == 0 {
                self.round += 1;
            }
        }

        self.start_turn();
    }
}
