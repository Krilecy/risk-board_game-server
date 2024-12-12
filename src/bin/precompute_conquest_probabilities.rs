use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::time::Instant;

struct Args {
    max_attack_armies: u16,
    max_defend_armies: u16,
}

// Parse command line arguments to get max army counts
lazy_static::lazy_static! {
    static ref ARGS: Args = {
        let args: Vec<String> = std::env::args().collect();

        // Default values if no args provided
        let mut max_attack = 100;
        let mut max_defend = 100;

        // Parse args (format: cargo run --bin precompute_conquest_probabilities <max_attack> <max_defend>)
        if args.len() > 2 {
            max_attack = args[1].parse().unwrap_or(100);
            max_defend = args[2].parse().unwrap_or(100);
        }

        Args {
            max_attack_armies: max_attack,
            max_defend_armies: max_defend,
        }
    };
}

fn calculate_attack_probability(
    attacker_armies: u32,
    defender_armies: u32,
    prob_cache: &Arc<Mutex<HashMap<(u16, u16), f64>>>,
) -> f64 {
    // Check if the probability is already in the cache
    if let Some(&prob) = prob_cache
        .lock()
        .unwrap()
        .get(&(attacker_armies as u16, defender_armies as u16))
    {
        return prob;
    }

    if attacker_armies <= 1 {
        return 0.0;
    }
    if defender_armies == 0 {
        return 1.0;
    }

    let a = attacker_armies as usize;
    let d = defender_armies as usize;

    let prob = if a >= 3 && d >= 2 {
        let p_win2 = p_win2(a);
        let p_lose2 = p_lose2(a);
        let p_win1_lose1 = 1.0 - p_win2 - p_lose2;

        p_win2 * calculate_attack_probability(a as u32, d as u32 - 2, prob_cache)
            + p_win1_lose1 * calculate_attack_probability(a as u32 - 1, d as u32 - 1, prob_cache)
            + p_lose2 * calculate_attack_probability(a as u32 - 2, d as u32, prob_cache)
    } else {
        let p_win1 = p_win1(a, d);

        p_win1 * calculate_attack_probability(a as u32, d as u32 - 1, prob_cache)
            + (1.0 - p_win1) * calculate_attack_probability(a as u32 - 1, d as u32, prob_cache)
    };

    // Store the calculated probability in the cache
    prob_cache
        .lock()
        .unwrap()
        .insert((attacker_armies as u16, defender_armies as u16), prob);

    prob
}

fn dice_distribution(n: usize) -> Vec<(usize, usize)> {
    match n {
        1 => (1..=6).map(|i| (1, i)).collect(),
        2 => (1..=6).flat_map(|i| (i..=6).map(move |j| (i, j))).collect(),
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

fn calculate_conquer_probability(
    attacker_armies: u16,
    defender_armies: u16,
    prob_cache: &Arc<Mutex<HashMap<(u16, u16), f64>>>,
) -> f64 {
    let start = Instant::now();
    // Call the recursive function to calculate probability
    let probability =
        calculate_attack_probability(attacker_armies as u32, defender_armies as u32, prob_cache);
    let rounded_probability = (probability * 10000.0).round() / 100.0;
    let time = start.elapsed();
    if time > Duration::new(10, 0) {
        println!("Computation time: {:?}", time);
    }
    rounded_probability
}

#[derive(Serialize, Deserialize)]
struct ProbabilityCache {
    cache: HashMap<(u16, u16), f64>,
}

fn main() {
    let start = Instant::now();

    // Create a shared, thread-safe cache
    let prob_cache = Arc::new(Mutex::new(HashMap::new()));

    // Create a parallel iterator to compute the probabilities in parallel
    let prob_cache_clone = prob_cache.clone();
    (2..=ARGS.max_attack_armies)
        .flat_map(|attacker_armies| {
            (1..=ARGS.max_defend_armies)
                .map(move |defender_armies| (attacker_armies, defender_armies))
        })
        .collect::<Vec<(u16, u16)>>()
        .par_iter()
        .for_each(|&(attacker_armies, defender_armies)| {
            println!(
                "{:?} attacker vs. {:?} defender",
                attacker_armies, defender_armies
            );
            let _ =
                calculate_conquer_probability(attacker_armies, defender_armies, &prob_cache_clone);
        });

    // After all computations are done, lock the cache and write it to a file
    let probability_cache = ProbabilityCache {
        cache: prob_cache.lock().unwrap().clone(),
    };

    // Serialize the cache to a file
    let mut file = File::create("conquer_probabilities.bin").expect("Failed to create file");
    let encoded: Vec<u8> =
        bincode::serialize(&probability_cache).expect("Failed to serialize data");
    file.write_all(&encoded).expect("Failed to write to file");

    println!("Probability cache successfully written to conquer_probabilities.bin");
    println!("Total computation time: {:?}", start.elapsed());
}
