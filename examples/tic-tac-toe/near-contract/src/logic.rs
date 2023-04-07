use crate::types::{CellState, GameState, BOARD_SIZE, ROW_SIZE};

const X_WINS_SUM: i8 = ROW_SIZE as i8;
const O_WINS_SUM: i8 = -X_WINS_SUM;

pub enum MoveResult {
    Move { updated_state: GameState },
    GameOver { winner: CellState },
}

pub fn get_move(state: GameState) -> MoveResult {
    // Special case for the first move by both players (for efficiency).
    let empty_count = state
        .board
        .iter()
        .filter(|x| x == &&CellState::Empty)
        .count();
    if empty_count == BOARD_SIZE {
        // On an empty board, the best move is to play X in a corner.
        let mut board = [CellState::Empty; BOARD_SIZE];
        board[0] = CellState::X;
        return MoveResult::Move {
            updated_state: GameState { board },
        };
    } else if empty_count == BOARD_SIZE - 1 {
        // O's first move should be in the center.
        // If the center is not available then play in the corner.
        let mut board = state.board;
        let center = &mut board[ROW_SIZE + (ROW_SIZE / 2)];
        if center == &CellState::Empty {
            *center = CellState::O;
        } else {
            board[0] = CellState::O;
        }
        return MoveResult::Move {
            updated_state: GameState { board },
        };
    }

    let (total, sums) = match evaluate_position(state) {
        Evaluation::Sums { total, sums } => (total, sums),
        Evaluation::GameOver { winner } => return MoveResult::GameOver { winner },
    };

    // On an empty board the total is 0.
    // X and O take turns where X adds 1 to the total and O subtracts 1.
    // X goes first.
    // Given the above, the total is 0 iff it is X to play and it is O to play otherwise.
    let player = if total == 0 {
        CellState::X
    } else {
        CellState::O
    };

    let possible_moves: Vec<(GameState, Evaluation)> = state
        .board
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            if let CellState::Empty = s {
                let new_state = {
                    let mut tmp = state.board;
                    tmp[i] = player;
                    GameState { board: tmp }
                };
                let eval = evaluate_position(new_state);
                Some((new_state, eval))
            } else {
                None
            }
        })
        .collect();

    // If there is only one move left then our play is forced
    if possible_moves.len() == 1 {
        return MoveResult::Move {
            updated_state: possible_moves[0].0,
        };
    }

    // Can we win in 1 move? Play it.
    if let Some((updated_state, _)) = possible_moves
        .iter()
        .find(|(_, eval)| matches!(eval, Evaluation::GameOver { winner } if winner == &player))
    {
        return MoveResult::Move {
            updated_state: *updated_state,
        };
    }

    // Can our opponent win in 1 move? Block it.
    let opponent_threat = (player.opponent() as i8) * (X_WINS_SUM - 1);
    let maybe_threat =
        sums.iter().enumerate().find_map(
            |(i, s)| {
                if s == &opponent_threat {
                    Some(i)
                } else {
                    None
                }
            },
        );
    if let Some(threat) = maybe_threat {
        let updated_state = possible_moves
            .into_iter()
            .find_map(|(state, eval)| match eval {
                Evaluation::Sums { sums, .. }
                    if sums[threat] == opponent_threat + (player as i8) =>
                {
                    Some(state)
                }
                _ => None,
            })
            .expect("A blocking move must be possible");
        return MoveResult::Move { updated_state };
    }

    // Can we create a threat (ideally a fork)? Do it.
    let player_threat = -opponent_threat;
    let maybe_threat = possible_moves
        .iter()
        .filter_map(|(state, eval)| match eval {
            Evaluation::Sums { sums, .. } => {
                let threats_count = sums.iter().filter(|s| s == &&player_threat).count();
                if threats_count > 0 {
                    Some((state, threats_count))
                } else {
                    None
                }
            }
            Evaluation::GameOver { .. } => None,
        })
        .max_by_key(|(_, threats_count)| *threats_count);
    if let Some((updated_state, _)) = maybe_threat {
        return MoveResult::Move {
            updated_state: *updated_state,
        };
    }

    // Otherwise: consider moves in the following order: center, corner, side.
    let mut board = state.board;
    let center = &mut board[ROW_SIZE + (ROW_SIZE / 2)];
    if center == &CellState::Empty {
        *center = player;
        return MoveResult::Move {
            updated_state: GameState { board },
        };
    }
    let corners = [
        0,
        ROW_SIZE - 1,
        (ROW_SIZE * ROW_SIZE) - ROW_SIZE,
        (ROW_SIZE * ROW_SIZE) - 1,
    ];
    for i in corners {
        let cell = &mut board[i];
        if cell == &CellState::Empty {
            *cell = player;
            return MoveResult::Move {
                updated_state: GameState { board },
            };
        }
    }

    MoveResult::Move {
        updated_state: possible_moves[0].0,
    }
}

fn evaluate_position(state: GameState) -> Evaluation {
    // Sums of cells for all the rows, columns and diagonals.
    // The first `ROW_SIZE` elements are the row sums, the next
    // `ROW_SIZE` elements are the column sums (it's a square board),
    // and the last two sums are the diagonals.
    let mut sums = [0_i8; ROW_SIZE + ROW_SIZE + 2];
    let mut total = 0_i8;
    for (x, cell) in state.board.into_iter().enumerate() {
        let i = x / ROW_SIZE;
        let j = x % ROW_SIZE;
        // Cast is safe since `CellState` has `repr(i8)`
        let n = cell as i8;
        total += n;
        sums[i] += n;
        sums[ROW_SIZE + j] += n;
        if i == j {
            // diagonal from top left to bottom right
            sums[ROW_SIZE + ROW_SIZE] += n;
        }
        if i + j == (ROW_SIZE - 1) {
            // diagonal from top right to bottom left
            sums[ROW_SIZE + ROW_SIZE + 1] += n;
        }
    }

    if sums.iter().any(|s| s == &X_WINS_SUM) {
        // 3 in a row for X
        return Evaluation::GameOver {
            winner: CellState::X,
        };
    } else if sums.iter().any(|s| s == &O_WINS_SUM) {
        // 3 in a row for O
        return Evaluation::GameOver {
            winner: CellState::O,
        };
    } else if state.board.iter().all(|s| s != &CellState::Empty) {
        // board is full without 3 in a row, so is a draw
        return Evaluation::GameOver {
            winner: CellState::Empty,
        };
    }

    Evaluation::Sums { sums, total }
}

enum Evaluation {
    Sums {
        sums: [i8; ROW_SIZE + ROW_SIZE + 2],
        total: i8,
    },
    GameOver {
        winner: CellState,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_game() {
        // The bot should always draw when playing itself.
        let mut state = GameState::default();
        let winner = loop {
            match get_move(state) {
                MoveResult::Move { updated_state } => state = updated_state,
                MoveResult::GameOver { winner } => break winner,
            }
        };
        assert_eq!(winner, CellState::Empty);
    }

    #[test]
    fn test_x_win() {
        // The bot should win as X against a bad player.
        let mut state = GameState::default();

        // First move: X in the corner
        state = unwrap_move(get_move(state));
        assert_eq!(state, "X........".parse().unwrap());

        // Second move: O in opposite corner
        state = "X.......O".parse().unwrap();

        // Third move: X creates a threat
        state = unwrap_move(get_move(state));
        assert_eq!(state, "X.....X.O".parse().unwrap());

        // Fourth move: O blocks
        state = "X..O..X.O".parse().unwrap();

        // Fifth move: X creates a fork
        state = unwrap_move(get_move(state));
        assert_eq!(state, "X.XO..X.O".parse().unwrap());

        // Sixth move: O blocks one branch
        state = "X.XOO.X.O".parse().unwrap();

        // Seventh move: X plays the win
        state = unwrap_move(get_move(state));
        assert_eq!(state, "XXXOO.X.O".parse().unwrap());

        // Game over.
        let winner = unwrap_winner(get_move(state));
        assert_eq!(winner, CellState::X);
    }

    #[test]
    fn test_o_win() {
        // The bot should win as O against a bad player.

        // First move: X in the corner
        let mut state = "X........".parse().unwrap();

        // Second move: O in the center
        state = unwrap_move(get_move(state));
        assert_eq!(state, "X...O....".parse().unwrap());

        // Third move: X in the lower corner
        state = "X...O...X".parse().unwrap();

        // Fourth move: O creates a threat
        state = unwrap_move(get_move(state));
        assert_eq!(state, "X...O..OX".parse().unwrap());

        // Fifth move: X creates a fork but misses that O is about to win
        state = "X.X.O..OX".parse().unwrap();

        // Sixth move: O make 3 in a row
        state = unwrap_move(get_move(state));
        assert_eq!(state, "XOX.O..OX".parse().unwrap());

        // Game over.
        let winner = unwrap_winner(get_move(state));
        assert_eq!(winner, CellState::O);
    }

    fn unwrap_move(result: MoveResult) -> GameState {
        match result {
            MoveResult::Move { updated_state } => updated_state,
            MoveResult::GameOver { .. } => panic!("Unexpected win!"),
        }
    }

    fn unwrap_winner(result: MoveResult) -> CellState {
        match result {
            MoveResult::GameOver { winner } => winner,
            MoveResult::Move { .. } => panic!("Unexpected move!"),
        }
    }
}
