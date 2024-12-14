# Risk Board Game Server

## Overview

Risk Board Game Server is a hobby project that implements the complete rule set of the classic board game Risk. Written in Rust, it provides a RESTful API. I made a [UI to showcase the game in a seperate repository](https://github.com/Krilecy/risk-ui).

Key features:
- Complete implementation of Risk game mechanics
- RESTful API for game state management and moves
- Pre-computed battle probabilities for quick calculations
- Parallelized with rayon and async/await with tokio for good performance
- Configurable game setup via JSON

## Prerequisites

- Rust
- Cargo (Rust package manager)
- Git

## Installation

1. Clone the repository:

```bash
git clone https://github.com/Krilecy/risk-board_game-server.git
cd risk-board-game-engine
```

2. Install dependencies:

```bash
cargo build
```

3. (Optional) Precompute battle probabilities for large armies:

```bash
cargo run --bin precompute_conquest_probabilities
```

4. Run the game:

```bash
cargo run
```

The server will start on `http://127.0.0.1:8000`.

## Battle Probability Calculator

The repository includes pre-computed battle probabilities for scenarios up to 100 attacking armies vs 100 defending armies for performance reasons. More can be computed by running the following command:

```bash
cargo run --bin precompute_conquest_probabilities <max_attacker_armies> <max_defender_armies>
```

Example (takes ca. 26s on an M1 Mac):
```bash
cargo run --bin precompute_conquest_probabilities 1000 1000
```

## API Endpoints

### Game State
- `GET /game-state`: Retrieve current game state

### Actions
- `POST /reinforce`: Add armies to a territory
- `POST /attack`: Execute an attack between territories
- `POST /fortify`: Move armies between connected territories
- `POST /trade_cards`: Trade in cards for additional armies
- `POST /advance_phase`: Progress to the next phase of the turn
- `POST /new-game`: Start a new game (a new game is automatically created when the server starts)

Detailed API documentation and request/response formats can be found in the [API Documentation](docs/api.md).

## Game Features

- Territory management
- Continent bonuses
- Card collection and trading
- Multi-phase turns (Reinforce, Attack, Fortify)
- Battle simulation with accurate probability calculations
- Support for 2-6 players
- Persistent game state

## Contributing

I consider this project to be feature complete. It was a fun project to implement and I learned a lot about Rust, tokio and dynamic programming. If you want to do something with it, feel free but don't expect me to add anything to it.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
