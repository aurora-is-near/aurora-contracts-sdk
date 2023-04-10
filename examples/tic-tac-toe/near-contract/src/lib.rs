pub mod logic;
pub mod types;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct TicTacToe;

#[near_bindgen]
impl TicTacToe {
    pub fn get_move(&self, state: String) -> GetMoveResponse {
        let parsed_state: types::GameState = state
            .parse()
            .unwrap_or_else(|_| env::panic_str("Invalid state string"));
        match logic::get_move(parsed_state) {
            logic::MoveResult::Move { updated_state } => {
                let serialized_state = updated_state.to_string();
                let winner = match logic::get_move(updated_state) {
                    logic::MoveResult::GameOver { winner } => Some(format!("{winner:?}")),
                    logic::MoveResult::Move { .. } => None,
                };
                GetMoveResponse {
                    updated_state: serialized_state,
                    winner,
                }
            }
            logic::MoveResult::GameOver { winner } => GetMoveResponse {
                updated_state: state,
                winner: Some(format!("{winner:?}")),
            },
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct GetMoveResponse {
    updated_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    winner: Option<String>,
}
