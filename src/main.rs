#[macro_use]
extern crate rocket;

use rocket::serde::json::Json;
use rocket::State;
use rocket_cors::{AllowedOrigins, CorsOptions};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
//use console_subscriber;
use rocket::response::content;

use risk_board_game_server::{
    game::{Game, GameState},
    game_config::GameConfig,
};

#[derive(Serialize, Debug)]
struct GameResponse {
    game_state: Option<GameState>,
    error: Option<String>,
}

impl GameResponse {
    fn success(game_state: GameState) -> Self {
        GameResponse {
            game_state: Some(game_state),
            error: None,
        }
    }

    fn error(game_state: GameState, error: String) -> Self {
        GameResponse {
            game_state: Some(game_state),
            error: Some(error),
        }
    }
}

#[derive(serde::Deserialize, Clone)]
struct ReinforceData {
    player_id: usize,
    territory: String,
    num_armies: u16,
}

#[derive(serde::Deserialize, Clone)]
struct BulkReinforceData {
    player_id: usize,
    reinforcements: Vec<ReinforceItem>,
}

#[derive(serde::Deserialize, Clone)]
struct ReinforceItem {
    territory: String,
    num_armies: u16,
}

#[derive(serde::Deserialize, Clone)]
struct AttackData {
    player_id: usize,
    from_territory: String,
    to_territory: String,
    num_dice: u16,
    repeat: bool,
}

#[derive(serde::Deserialize, Clone)]
struct FortifyData {
    player_id: usize,
    from_territory: String,
    to_territory: String,
    num_armies: u16,
}

#[derive(serde::Deserialize, Clone)]
struct MoveArmiesData {
    player_id: usize,
    from_territory: String,
    to_territory: String,
    num_armies: u16,
}

#[derive(serde::Deserialize, Clone)]
struct TradeCardsData {
    player_id: usize,
    card_indices: Vec<usize>,
}

#[derive(serde::Deserialize, Clone)]
struct NewGameData {
    config_file: Option<String>,
    num_players: Option<usize>,
}

#[derive(Clone)]
enum Request {
    Reinforce(ReinforceData),
    BulkReinforce(BulkReinforceData),
    Attack(AttackData),
    Fortify(FortifyData),
    MoveArmies(MoveArmiesData),
    TradeCards(TradeCardsData),
    AdvancePhase,
    NewGame(NewGameData),
    GetGameState,
}

struct RequestWithResponse {
    request: Request,
    response_sender: oneshot::Sender<GameResponse>,
}

struct SharedState {
    sender: mpsc::Sender<RequestWithResponse>,
}

#[derive(Serialize)]
struct ApiEndpoint {
    path: String,
    method: String,
    description: String,
}

#[get("/")]
fn api_documentation() -> content::RawJson<String> {
    let endpoints = vec![
        ApiEndpoint {
            path: "/".to_string(),
            method: "GET".to_string(),
            description: "Shows this API documentation".to_string(),
        },
        ApiEndpoint {
            path: "/game-state".to_string(),
            method: "GET".to_string(),
            description: "Get the current state of the game".to_string(),
        },
        ApiEndpoint {
            path: "/reinforce".to_string(),
            method: "POST".to_string(),
            description: "Reinforce a territory with armies".to_string(),
        },
        ApiEndpoint {
            path: "/bulk_reinforce".to_string(),
            method: "POST".to_string(),
            description: "Reinforce multiple territories at once".to_string(),
        },
        ApiEndpoint {
            path: "/attack".to_string(),
            method: "POST".to_string(),
            description: "Attack from one territory to another".to_string(),
        },
        ApiEndpoint {
            path: "/fortify".to_string(),
            method: "POST".to_string(),
            description: "Move armies between connected territories".to_string(),
        },
        ApiEndpoint {
            path: "/move_armies".to_string(),
            method: "POST".to_string(),
            description: "Move armies after a successful attack".to_string(),
        },
        ApiEndpoint {
            path: "/trade_cards".to_string(),
            method: "POST".to_string(),
            description: "Trade in cards for additional armies".to_string(),
        },
        ApiEndpoint {
            path: "/advance_phase".to_string(),
            method: "POST".to_string(),
            description: "Advance to the next game phase".to_string(),
        },
        ApiEndpoint {
            path: "/new-game".to_string(),
            method: "POST".to_string(),
            description: "Start a new game with optional configuration".to_string(),
        },
    ];

    content::RawJson(serde_json::to_string_pretty(&endpoints).unwrap())
}

#[post("/reinforce", data = "<data>")]
async fn reinforce(data: Json<ReinforceData>, state: &State<SharedState>) -> Json<GameResponse> {
    send_request_and_wait(state, Request::Reinforce(data.into_inner())).await
}

#[post("/bulk_reinforce", data = "<data>")]
async fn bulk_reinforce(
    data: Json<BulkReinforceData>,
    state: &State<SharedState>,
) -> Json<GameResponse> {
    send_request_and_wait(state, Request::BulkReinforce(data.into_inner())).await
}

#[post("/attack", data = "<data>")]
async fn attack(data: Json<AttackData>, state: &State<SharedState>) -> Json<GameResponse> {
    send_request_and_wait(state, Request::Attack(data.into_inner())).await
}

#[post("/fortify", data = "<data>")]
async fn fortify(data: Json<FortifyData>, state: &State<SharedState>) -> Json<GameResponse> {
    send_request_and_wait(state, Request::Fortify(data.into_inner())).await
}

#[post("/move_armies", data = "<data>")]
async fn move_armies(data: Json<MoveArmiesData>, state: &State<SharedState>) -> Json<GameResponse> {
    send_request_and_wait(state, Request::MoveArmies(data.into_inner())).await
}

#[post("/trade_cards", data = "<data>")]
async fn trade_cards(data: Json<TradeCardsData>, state: &State<SharedState>) -> Json<GameResponse> {
    send_request_and_wait(state, Request::TradeCards(data.into_inner())).await
}

#[post("/advance_phase")]
async fn advance_phase(state: &State<SharedState>) -> Json<GameResponse> {
    send_request_and_wait(state, Request::AdvancePhase).await
}

#[post("/new-game", data = "<data>")]
async fn new_game(state: &State<SharedState>, data: Json<NewGameData>) -> Json<GameResponse> {
    send_request_and_wait(state, Request::NewGame(data.into_inner())).await
}

#[get("/game-state")]
async fn game_state(state: &State<SharedState>) -> Json<GameResponse> {
    send_request_and_wait(state, Request::GetGameState).await
}

async fn send_request_and_wait(state: &State<SharedState>, request: Request) -> Json<GameResponse> {
    let (response_sender, response_receiver) = oneshot::channel();
    state
        .sender
        .send(RequestWithResponse {
            request,
            response_sender,
        })
        .await
        .expect("Failed to send request");

    let response = response_receiver.await.expect("Failed to receive response");
    Json(response)
}

async fn worker_task(mut receiver: mpsc::Receiver<RequestWithResponse>, game: Arc<Mutex<Game>>) {
    while let Some(RequestWithResponse {
        request,
        response_sender,
    }) = receiver.recv().await
    {
        let mut game = game.lock().await;
        let response = match request {
            Request::Reinforce(data) => {
                match game.reinforce(data.player_id, &data.territory, data.num_armies) {
                    Ok(_) => GameResponse::success(game.get_game_state()),
                    Err(e) => GameResponse::error(game.get_game_state(), e.to_string()),
                }
            }
            Request::BulkReinforce(data) => {
                let mut error = None;
                for reinforce_item in data.reinforcements {
                    if let Err(e) = game.reinforce(
                        data.player_id,
                        &reinforce_item.territory,
                        reinforce_item.num_armies,
                    ) {
                        error = Some(e.to_string());
                        break;
                    }
                }
                match error {
                    Some(e) => GameResponse::error(game.get_game_state(), e),
                    None => GameResponse::success(game.get_game_state()),
                }
            }
            Request::Attack(data) => {
                match game.attack(
                    data.player_id,
                    &data.from_territory,
                    &data.to_territory,
                    data.num_dice,
                    data.repeat,
                ) {
                    Ok(_) => GameResponse::success(game.get_game_state()),
                    Err(e) => GameResponse::error(game.get_game_state(), e.to_string()),
                }
            }
            Request::Fortify(data) => {
                match game.fortify(
                    data.player_id,
                    &data.from_territory,
                    &data.to_territory,
                    data.num_armies,
                ) {
                    Ok(_) => GameResponse::success(game.get_game_state()),
                    Err(e) => GameResponse::error(game.get_game_state(), e.to_string()),
                }
            }
            Request::MoveArmies(data) => {
                match game.move_armies_after_attack(
                    data.player_id,
                    &data.from_territory,
                    &data.to_territory,
                    data.num_armies,
                ) {
                    Ok(_) => GameResponse::success(game.get_game_state()),
                    Err(e) => GameResponse::error(game.get_game_state(), e.to_string()),
                }
            }
            Request::TradeCards(data) => {
                match game.trade_cards(data.player_id, data.card_indices) {
                    Ok(_) => GameResponse::success(game.get_game_state()),
                    Err(e) => GameResponse::error(game.get_game_state(), e.to_string()),
                }
            }
            Request::AdvancePhase => {
                game.advance_phase();
                GameResponse::success(game.get_game_state())
            }
            Request::NewGame(data) => {
                let config = data
                    .config_file
                    .as_ref()
                    .and_then(|path| GameConfig::load_from_file(&path).ok());
                *game = Game::new(config, data.num_players);
                GameResponse::success(game.get_game_state())
            }
            Request::GetGameState => GameResponse::success(game.get_game_state()),
        };
        response_sender
            .send(response)
            .expect("Failed to send response");
    }
}

#[launch]
async fn rocket() -> _ {
    let (sender, receiver) = mpsc::channel::<RequestWithResponse>(100);
    let game = Arc::new(Mutex::new(Game::new(None, Some(6))));

    //console_subscriber::init();
    tokio::spawn(worker_task(receiver, game.clone()));

    let cors = CorsOptions::default()
        .allowed_origins(AllowedOrigins::all())
        .to_cors()
        .expect("Error creating CORS middleware");

    rocket::build()
        .manage(SharedState { sender })
        .mount(
            "/",
            routes![
                api_documentation,
                reinforce,
                bulk_reinforce,
                attack,
                fortify,
                move_armies,
                trade_cards,
                advance_phase,
                game_state,
                new_game
            ],
        )
        .attach(cors)
}
