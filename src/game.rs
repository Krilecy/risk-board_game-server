use crate::board::Board;
use crate::card::Card;
use crate::game_config::GameConfig;
use crate::player::Player;
use crate::turn_phase::TurnPhase;
use itertools::Itertools;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, HashMap};
use std::fs::File;
use std::io::Read;
use bincode;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameState {
    pub current_player: String,
    pub current_turn: usize,
    pub round: usize,
    pub turn_phase: TurnPhase,
    pub conquered_territory: bool,
    pub reinforcement_armies: u16,
    pub initial_reinforcement_armies: u16,
    pub defeated_players: Vec<usize>,
    pub possible_actions: Vec<Action>,
    pub players: Vec<Player>,
    pub board: Board,
    pub conquer_probs: Vec<(String, String, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Game {
    pub players: Vec<Player>,
    pub board: Board,
    pub current_turn: usize,
    pub round: usize,
    pub turn_phase: TurnPhase,
    pub reinforcement_armies: u16,
    pub initial_reinforcement_armies: u16,
    pub deck: Vec<Card>,
    pub discard_pile: Vec<Card>,
    pub conquered_territory: bool,
    pub defeated_players: Vec<usize>,
    pub last_attack_from: Option<String>,
    pub last_attack_to: Option<String>,
    pub last_attack_dice: Option<u16>,
    pub active_players: Vec<usize>,
    pub conquer_probs: Vec<(String, String, f64)>,
    prob_cache: HashMap<(u16, u16), f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Action {
    Reinforce {
        territory: String,
        max_armies: u16,
    },
    Attack {
        from: String,
        to: String,
        max_dice: u16,
    },
    Fortify {
        from: String,
        to: String,
        max_armies: u16,
    },
    TradeCards {
        card_indices: Vec<usize>,
    },
    MoveArmies {
        from: String,
        to: String,
        max_armies: u16,
        min_armies: u16,
    },
    EndPhase,
}

#[derive(Serialize, Deserialize)]
struct ProbabilityCache {
    cache: HashMap<(u16, u16), f64>,
}

impl Game {
    pub fn new(config: Option<GameConfig>, num_players: Option<usize>) -> Self {
        let (board, players) = match config {
            Some(cfg) => cfg.to_board_and_players(),
            None => {
                let num_players = num_players.unwrap_or(6);
                let mut board = Game::create_board_from_config();
                let players = Game::create_random_players(num_players, &mut board);
                (board, players)
            }
        };

        let deck = Game::create_deck(&board);
        let active_players = (0..players.len()).collect();

        let mut game = Self {
            players,
            board,
            current_turn: 0,
            round: 0,
            turn_phase: TurnPhase::Reinforce,
            deck,
            discard_pile: vec![],
            reinforcement_armies: 0,
            initial_reinforcement_armies: 0,
            conquered_territory: false,
            defeated_players: vec![],
            last_attack_from: None,
            last_attack_to: None,
            last_attack_dice: None,
            active_players,
            prob_cache: HashMap::new(),
            conquer_probs: vec![],
        };

        game.load_conquer_probabilities("conquer_probabilities.bin");

        game.start_turn();
        game.initial_reinforcement_armies = game.reinforcement_armies;
        game
    }

    fn create_board_from_config() -> Board {
        let config_data = include_str!("config.json");
        
        let config: GameConfig =
            serde_json::from_str(&config_data).expect("Unable to parse config file");
    
        let (board, _) = config.to_board_and_players();
        board
    }

    fn create_random_players(num_players: usize, board: &mut Board) -> Vec<Player> {
        let initial_armies = match num_players {
            3 => 35,
            4 => 30,
            5 => 25,
            _ => 20, // Default to 20 armies for 6 or more players
        };

        let mut players = Vec::new();
        for i in 0..num_players {
            players.push(Player::new(i, &format!("Player {}", i + 1)));
        }

        // Shuffle and distribute territories
        board.shuffle_and_distribute_territories(&mut players);

        // Calculate the total initial armies already on the board
        let armies_on_board: Vec<u16> = players
            .iter()
            .map(|player| {
                player
                    .territories
                    .iter()
                    .map(|territory| player.get_armies(territory))
                    .sum()
            })
            .collect();

        // Distribute remaining armies to ensure each player reaches initial_armies threshold
        let mut remaining_armies: Vec<u16> = armies_on_board
            .iter()
            .map(|&armies| initial_armies - armies)
            .collect();

        while remaining_armies.iter().any(|&armies| armies > 0) {
            for (player_index, player) in players.iter_mut().enumerate() {
                if remaining_armies[player_index] == 0 {
                    continue;
                }

                let territories: Vec<String> = player.territories.iter().cloned().collect();
                for territory in territories {
                    if remaining_armies[player_index] > 0 {
                        player.reinforce(&territory, 1);
                        remaining_armies[player_index] -= 1;
                    } else {
                        break;
                    }
                }
            }
        }

        players
    }

    pub fn reinforce(
        &mut self,
        player_id: usize,
        territory: &str,
        num_armies: u16,
    ) -> Result<(), String> {
        if self.turn_phase != TurnPhase::Reinforce {
            return Err("It's not the reinforcement phase.".to_string());
        }

        let player = self.players.get_mut(player_id).ok_or("Invalid player ID")?;
        if !player.territories.contains(territory) {
            return Err(format!(
                "Territory '{}' does not belong to player with ID {}",
                territory, player_id
            ));
        }

        if num_armies > self.reinforcement_armies {
            return Err("Not enough reinforcement armies available.".to_string());
        }

        player.reinforce(territory, num_armies);
        self.reinforcement_armies -= num_armies;

        // Check if all reinforcement armies have been placed
        if (self.reinforcement_armies == 0) & (player.cards.len() < 5) {
            self.turn_phase = TurnPhase::Attack;
        }

        Ok(())
    }

    pub fn attack(
        &mut self,
        attacker_id: usize,
        from_territory: &str,
        to_territory: &str,
        mut num_dice: u16,
        repeat: bool, // New parameter for repeated attacks
    ) -> Result<(), String> {
        if self.turn_phase != TurnPhase::Attack {
            return Err("It's not the attack phase.".to_string());
        }
    
        let defender_index = self
            .players
            .iter()
            .position(|p| p.territories.contains(to_territory))
            .ok_or("No player owns the to territory")?;
    
        let attacker_index = attacker_id;
    
        // Ensure the attacker owns the from_territory and that the territories are adjacent
        {
            let attacker = self
                .players
                .get(attacker_index)
                .ok_or("Invalid attacker ID")?;
            if !attacker.territories.contains(from_territory) {
                return Err("From territory does not belong to the attacker".to_string());
            }
            let from = self
                .board
                .get_territory(from_territory)
                .ok_or("Invalid from territory")?;
            if !from.is_adjacent(to_territory) {
                return Err("To territory is not adjacent to from territory".to_string());
            }
        }
    
        // Assert that attacker and defender are not the same
        assert!(
            attacker_index != defender_index,
            "A player cannot attack themselves"
        );
    
        loop {
            // Borrow mutable references to the attacker and defender
            let (attacker, defender) = if attacker_index < defender_index {
                let (left, right) = self.players.split_at_mut(defender_index);
                (&mut left[attacker_index], &mut right[0])
            } else {
                let (left, right) = self.players.split_at_mut(attacker_index);
                (&mut right[0], &mut left[defender_index])
            };
    
            // Dynamically adjust the number of dice based on the attacker's remaining armies
            let attacker_armies = attacker.get_armies(from_territory);
            num_dice = std::cmp::min(num_dice, attacker_armies - 1).min(3) as u16;
    
            // Roll dice
            let mut attacker_rolls: Vec<u16> = (0..num_dice)
                .map(|_| thread_rng().gen_range(1..=6))
                .collect();
            let defender_dice = defender.get_armies(to_territory).min(2);
            let defender_rolls: Vec<u16> = (0..defender_dice)
                .map(|_| thread_rng().gen_range(1..=6))
                .collect();
    
            attacker_rolls.sort_unstable_by(|a, b| b.cmp(a));
            let mut defender_rolls_sorted = defender_rolls.clone();
            defender_rolls_sorted.sort_unstable_by(|a, b| b.cmp(a));
    
            let mut attacker_losses = 0;
            let mut defender_losses = 0;
    
            for (attack, defend) in attacker_rolls.iter().zip(defender_rolls_sorted.iter()) {
                if attack > defend {
                    defender_losses += 1;
                } else {
                    attacker_losses += 1;
                }
            }
    
            // Apply losses
            attacker.remove_armies(from_territory, attacker_losses);
            defender.remove_armies(to_territory, defender_losses);
    
            // Check if defender lost the territory
            if defender.get_armies(to_territory) == 0 {
                defender.remove_territory(to_territory);
                attacker.add_territory(to_territory);
                self.conquered_territory = true;
    
                if defender.territories.is_empty() {
                    self.defeated_players.push(defender_index);
                    self.active_players
                        .retain(|&player_idx| player_idx != defender_index);
                    attacker.cards.extend(std::mem::take(&mut defender.cards));
                }
    
                // Set last attack fields
                self.last_attack_from = Some(from_territory.to_string());
                self.last_attack_to = Some(to_territory.to_string());
                self.last_attack_dice = Some(num_dice);
                self.turn_phase = TurnPhase::MoveArmies;
    
                if self.check_win_conditions() {
                    self.turn_phase = TurnPhase::GameOver;
                }
    
                return Ok(());
            }
    
            // If repeat is true, continue attacking until one side is defeated
            if !repeat || attacker.get_armies(from_territory) <= 1 || defender.get_armies(to_territory) == 0 {
                break;
            }
        }
    
        Ok(())
    }

    pub fn move_armies_after_attack(
        &mut self,
        player_id: usize,
        from_territory: &str,
        to_territory: &str,
        num_armies: u16,
    ) -> Result<(), String> {
        let player = self.players.get_mut(player_id).ok_or("Invalid player ID")?;
        if !player.territories.contains(from_territory)
            || !player.territories.contains(to_territory)
        {
            return Err("One or both territories do not belong to the player".to_string());
        }

        if num_armies >= player.get_armies(from_territory) {
            return Err("Cannot move all armies or more than available.".to_string());
        }

        player.fortify(from_territory, to_territory, num_armies);
        self.turn_phase = TurnPhase::Attack;

        self.last_attack_from = None;
        self.last_attack_to = None;
        self.last_attack_dice = None;
        if player.cards.len() >= 5 {
            self.turn_phase = TurnPhase::Reinforce;
        }

        Ok(())
    }

    pub fn fortify(
        &mut self,
        player_id: usize,
        from_territory: &str,
        to_territory: &str,
        num_armies: u16,
    ) -> Result<(), String> {
        if self.turn_phase != TurnPhase::Fortify {
            return Err("It's not the fortification phase.".to_string());
        }

        if !self.are_territories_connected_via_player(player_id, from_territory, to_territory) {
            return Err("Territories are not connected.".to_string());
        }

        let player = self.players.get_mut(player_id).ok_or("Invalid player ID")?;
        if !player.territories.contains(from_territory)
            || !player.territories.contains(to_territory)
        {
            return Err("One or both territories do not belong to the player".to_string());
        }

        if num_armies >= player.get_armies(from_territory) {
            return Err("Cannot move all armies or more than available.".to_string());
        }

        player.fortify(from_territory, to_territory, num_armies);
        self.end_turn(); // End the turn immediately after fortification
        Ok(())
    }

    pub fn calculate_reinforcements(&self, player_id: usize) -> u16 {
        let player = &self.players[player_id];
        let territories_owned = player.territories.len() as u16;
        let base_reinforcements = std::cmp::max(territories_owned / 3, 3);

        // Calculate continent bonuses
        let mut continent_bonus = 0 as u16;
        for continent in self.board.continents.values() {
            if continent
                .territories
                .iter()
                .all(|t| player.territories.contains(t))
            {
                continent_bonus += continent.bonus_armies;
            }
        }

        base_reinforcements + continent_bonus
    }

    pub fn check_win_conditions(&self) -> bool {
        self.players
            .iter()
            .any(|p| p.territories.len() == self.board.territories.len())
    }

    // Calculate the probability of attacker winning after all possible rolls
    fn calculate_conquer_probability(&mut self, attacker_armies: u16, defender_armies: u16) -> f64 {

    // Check if the probability is already in the cache
    if let Some(&prob) = self.prob_cache.get(&(attacker_armies, defender_armies)) {
        return prob;
    }
    
        fn calculate_attack_probability(attacker_armies: u32, defender_armies: u32) -> f64 {
            if attacker_armies <= 1 {
                return 0.0;
            }
            if defender_armies == 0 {
                return 1.0;
            }

            let a = attacker_armies as usize;
            let d = defender_armies as usize;

            if a >= 3 && d >= 2 {
                let p_win2 = p_win2(a);
                let p_lose2 = p_lose2(a);
                let p_win1_lose1 = 1.0 - p_win2 - p_lose2;
                
                p_win2 * calculate_attack_probability(a as u32, d as u32 - 2)
                + p_win1_lose1 * calculate_attack_probability(a as u32 - 1, d as u32 - 1)
                + p_lose2 * calculate_attack_probability(a as u32 - 2, d as u32)
            } else {
                let p_win1 = p_win1(a, d);
                
                p_win1 * calculate_attack_probability(a as u32, d as u32 - 1)
                + (1.0 - p_win1) * calculate_attack_probability(a as u32 - 1, d as u32)
            }
        }

        fn dice_distribution(n: usize) -> Vec<(usize, usize)> {
            match n {
                1 => (1..=6).map(|i| (1, i)).collect(),
                2 => (1..=6)
                    .flat_map(|i| (i..=6).map(move |j| (i, j)))
                    .collect(),
                3 => (1..=6)
                    .flat_map(|i| (i..=6).flat_map(move |j| (j..=6).map(move |k| (j, k))))
                    .collect(),
                _ => panic!("Invalid number of dice"),
            }
        }

        fn p_win2(a: usize) -> f64 {
            let attacker_dist = dice_distribution(std::cmp::min(a - 1, 3));
            let defender_dist = dice_distribution(2);
            let total = attacker_dist.len() * defender_dist.len();
            let wins = attacker_dist
                .iter()
                .flat_map(|&(a1, a2)| {
                    defender_dist
                        .iter()
                        .filter(move |&&(d1, d2)| a1 > d1 && a2 > d2)
                })
                .count();
            wins as f64 / total as f64
        }

        fn p_lose2(a: usize) -> f64 {
            let attacker_dist = dice_distribution(std::cmp::min(a - 1, 3));
            let defender_dist = dice_distribution(2);
            let total = attacker_dist.len() * defender_dist.len();
            let losses = attacker_dist
                .iter()
                .flat_map(|&(a1, a2)| {
                    defender_dist
                        .iter()
                        .filter(move |&&(d1, d2)| a1 <= d1 && a2 <= d2)
                })
                .count();
            losses as f64 / total as f64
        }

        fn p_win1(a: usize, d: usize) -> f64 {
            let attacker_dist = dice_distribution(std::cmp::min(a - 1, 3));
            let defender_dist = dice_distribution(std::cmp::min(d, 2));
            let total = attacker_dist.len() * defender_dist.len();
            let wins = attacker_dist
                .iter()
                .flat_map(|&(_, a2)| defender_dist.iter().filter(move |&&(_, d2)| a2 > d2))
                .count();
            wins as f64 / total as f64
        }

        // Call the recursive function to calculate probability
        let probability = calculate_attack_probability(attacker_armies as u32, defender_armies as u32);
        let rounded_probability = (probability * 10000.0).round() / 100.0;

        // Store the computed probability in the cache
        self.prob_cache.insert((attacker_armies, defender_armies), rounded_probability);
        
        rounded_probability

    }

    pub fn load_conquer_probabilities(&mut self, filename: &str) {
        let mut file = File::open(filename).expect("Failed to open probabilities file");
        let mut encoded = Vec::new();
        file.read_to_end(&mut encoded).expect("Failed to read file");

        let probability_cache: ProbabilityCache = bincode::deserialize(&encoded).expect("Failed to deserialize data");
        self.prob_cache = probability_cache.cache;
    }


    // New function to calculate conquer probabilities
    fn calculate_conquer_probabilities(&mut self) -> Vec<(String, String, f64)> {
        let mut conquer_probs = Vec::new();
        for (i, player) in self.players.iter().enumerate() {
            for territory in &player.territories {
                let adjacent_territories = &self
                    .board
                    .get_territory(territory)
                    .unwrap()
                    .adjacent_territories;
                let attacker_armies = player.get_armies(territory);

                if attacker_armies > 1 {
                    for adjacent in adjacent_territories {
                        if let Some(defender) = self
                            .players
                            .iter()
                            .find(|p| p.territories.contains(adjacent))
                        {
                            if defender.id != i {
                                let defender_armies = defender.get_armies(adjacent);
                                conquer_probs.push((territory.clone(), adjacent.clone(), (attacker_armies, defender_armies)));
                            }
                        }
                    }
                }
            }
        }
        conquer_probs.into_iter().map(|(from, to, (att, def))| {
            (from, to, self.calculate_conquer_probability(att, def))
        }).collect()
    }

    // Update the existing get_game_state method
    pub fn get_game_state(&mut self) -> GameState {
        let army_supply: Vec<u16> = self
            .players
            .iter()
            .map(|player| self.calculate_reinforcements(player.id))
            .collect();

        for (player, &army_supply) in self.players.iter_mut().zip(army_supply.iter()) {
            player.army_supply = army_supply;
            player.calculate_total_armies();
            }

        // Calculate conquer probabilities
        let conquer_probs = self.calculate_conquer_probabilities();

        GameState {
            players: self.players.clone(),
            board: self.board.clone(),
            current_turn: self.current_turn,
            round: self.round,
            current_player: self.players[self.current_turn].name.clone(),
            turn_phase: self.turn_phase.clone(),
            reinforcement_armies: self.reinforcement_armies,
            initial_reinforcement_armies: self.initial_reinforcement_armies,
            conquered_territory: self.conquered_territory,
            defeated_players: self.defeated_players.clone(),
            possible_actions: self.get_possible_actions(),
            conquer_probs, // New field added to include attack probabilities
        }
    }

    pub fn get_possible_actions(&self) -> Vec<Action> {
        match self.turn_phase {
            TurnPhase::Reinforce => {
                let mut actions = self.get_possible_reinforcements();
                actions.extend(self.get_possible_trades());
                if (self.reinforcement_armies == 0)
                    & (self.players[self.current_turn].cards.len() < 5)
                {
                    actions.push(Action::EndPhase);
                }
                actions
            }
            TurnPhase::Attack => self.get_possible_attacks(),
            TurnPhase::Fortify => self.get_possible_fortifications(),
            TurnPhase::MoveArmies => self.get_possible_army_moves(),
            TurnPhase::GameOver => vec![],
        }
    }

    fn get_possible_army_moves(&self) -> Vec<Action> {
        let mut actions = Vec::new();
        if let (Some(ref from_territory), Some(ref to_territory), Some(dice_used)) = (
            &self.last_attack_from,
            &self.last_attack_to,
            self.last_attack_dice,
        ) {
            let max_armies = self.players[self.current_turn].get_armies(from_territory) - 1;
            let min_armies = dice_used;
            actions.push(Action::MoveArmies {
                from: from_territory.clone(),
                to: to_territory.clone(),
                max_armies: max_armies,
                min_armies: min_armies,
            });
        }
        actions
    }

    fn get_possible_reinforcements(&self) -> Vec<Action> {
        let mut actions = Vec::new();
        if self.reinforcement_armies != 0 {
            for territory in &self.players[self.current_turn].territories {
                actions.push(Action::Reinforce {
                    territory: territory.clone(),
                    max_armies: self.reinforcement_armies,
                });
            }
        }
        actions
    }

    fn get_possible_trades(&self) -> Vec<Action> {
        let player = &self.players[self.current_turn];
        let mut actions = Vec::new();

        if player.cards.len() < 3 {
            return actions;
        }

        let mut seen_combinations = HashSet::new();
        let card_combinations = (0..player.cards.len()).combinations(3);

        for combo in card_combinations {
            let card_kinds = combo
                .iter()
                .map(|&i| &player.cards[i].kind)
                .collect::<Vec<_>>();
            if self.is_valid_trade(&card_kinds) {
                let sorted_combo = {
                    let mut sorted_combo = combo.clone();
                    sorted_combo.sort();
                    sorted_combo
                };

                if seen_combinations.insert(sorted_combo.clone()) {
                    actions.push(Action::TradeCards {
                        card_indices: sorted_combo,
                    });
                }
            }
        }

        actions
    }

    fn get_possible_attacks(&self) -> Vec<Action> {
        let mut actions = Vec::new();
        for territory in &self.players[self.current_turn].territories {
            let adjacent_territories = &self
                .board
                .get_territory(territory)
                .unwrap()
                .adjacent_territories;
            for adjacent in adjacent_territories {
                if !self.players[self.current_turn]
                    .territories
                    .contains(adjacent)
                {
                    let max_dice = self.players[self.current_turn]
                        .get_armies(territory)
                        .saturating_sub(1)
                        .min(3);
                    if max_dice > 0 {
                        actions.push(Action::Attack {
                            from: territory.clone(),
                            to: adjacent.clone(),
                            max_dice,
                        });
                    }
                }
            }
        }
        actions.push(Action::EndPhase);
        actions
    }

    fn are_territories_connected_via_player(
        &self,
        player_id: usize,
        from_territory: &str,
        to_territory: &str,
    ) -> bool {
        let player = &self.players[player_id];
        if !player.territories.contains(from_territory) || !player.territories.contains(to_territory) {
            return false;
        }
        
        let mut visited = HashSet::new();
        let mut stack = vec![from_territory.to_string()];
        
        while let Some(current) = stack.pop() {
            if current == to_territory {
                return true;
            }
            
            if !visited.insert(current.clone()) {
                continue;
            }
            
            if let Some(adjacent_territories) = self.board.get_territory(&current) {
                for adjacent in &adjacent_territories.adjacent_territories {
                    if player.territories.contains(adjacent) && !visited.contains(adjacent) {
                        stack.push(adjacent.clone());
                    }
                }
            }
        }
        false
    }

    fn get_possible_fortifications(&self) -> Vec<Action> {
        let mut actions = Vec::new();
        let current_player = &self.players[self.current_turn];

        for from_territory in &current_player.territories {
            let mut visited = HashSet::new();
            let mut stack = vec![from_territory];

            while let Some(current) = stack.pop() {
                if !visited.insert(current) {
                    continue;
                }

                if current != from_territory {
                    let max_armies = current_player.get_armies(from_territory) - 1;
                    if max_armies > 0 {
                        actions.push(Action::Fortify {
                            from: from_territory.clone(),
                            to: current.clone(),
                            max_armies,
                        });
                    }
                }

                if let Some(adjacent_territories) = self.board.get_territory(current) {
                    for adjacent in &adjacent_territories.adjacent_territories {
                        if current_player.territories.contains(adjacent) && !visited.contains(adjacent) {
                            stack.push(adjacent);
                        }
                    }
                }
            }
        }
        actions.push(Action::EndPhase);
        actions
    }
}
