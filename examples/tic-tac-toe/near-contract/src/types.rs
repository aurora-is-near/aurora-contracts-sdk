use std::{fmt, str::FromStr};

pub const ROW_SIZE: usize = 3;
pub const BOARD_SIZE: usize = ROW_SIZE * ROW_SIZE;

#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Empty = 0,
    X = 1,
    O = -1,
}

impl CellState {
    pub fn opponent(&self) -> Self {
        match self {
            Self::Empty => Self::Empty,
            Self::X => Self::O,
            Self::O => Self::X,
        }
    }
}

impl Default for CellState {
    fn default() -> Self {
        Self::Empty
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GameState {
    /// Row-major representation of the board
    pub board: [CellState; BOARD_SIZE],
}

impl ToString for GameState {
    fn to_string(&self) -> String {
        let mut buf = Vec::with_capacity(BOARD_SIZE);
        for cell in self.board.iter() {
            let value = match cell {
                CellState::Empty => b'.',
                CellState::X => b'X',
                CellState::O => b'O',
            };
            buf.push(value);
        }
        // This is safe because the conversion above only uses utf-8 characters.
        unsafe { String::from_utf8_unchecked(buf) }
    }
}

impl FromStr for GameState {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = s.as_bytes();
        if bytes.len() != BOARD_SIZE {
            return Err(ParseError::InvalidLength {
                expected: BOARD_SIZE,
                actual: bytes.len(),
            });
        }
        let mut board = [CellState::Empty; BOARD_SIZE];
        for (i, (byte, cell)) in bytes.iter().zip(board.iter_mut()).enumerate() {
            match byte {
                b'X' => *cell = CellState::X,
                b'O' => *cell = CellState::O,
                // nothing to do in this case because we initialize the board with empty cells
                b'.' => (),
                other => {
                    // byte does not represent a cell state
                    let value = (*other) as char;
                    return Err(ParseError::InvalidCharacter { position: i, value });
                }
            }
        }
        Ok(Self { board })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    InvalidLength { expected: usize, actual: usize },
    InvalidCharacter { position: usize, value: char },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCharacter { position, value } => {
                write!(f, "Invalid character {value} at position {position}")
            }
            Self::InvalidLength { expected, actual } => write!(
                f,
                "Invalid input length. Expected={expected} Actual={actual}"
            ),
        }
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        // Position:
        // X | O | O
        // ----------
        // O | _ | O
        // ----------
        // X | X | X
        let input = "XOOO.OXXX";
        let game: GameState = input.parse().unwrap();
        let output = game.to_string();
        assert_eq!(input, output.as_str());

        // Invalid character
        let input = "..A......";
        let game: Result<GameState, ParseError> = input.parse();
        let err = game.unwrap_err();
        let expected_err = ParseError::InvalidCharacter {
            position: 2,
            value: 'A',
        };
        assert_eq!(err, expected_err);

        // Too long
        let input = "..........";
        let game: Result<GameState, ParseError> = input.parse();
        let err = game.unwrap_err();
        let expected_err = ParseError::InvalidLength {
            expected: BOARD_SIZE,
            actual: 10,
        };
        assert_eq!(err, expected_err);

        // Too short
        let input = "........";
        let game: Result<GameState, ParseError> = input.parse();
        let err = game.unwrap_err();
        let expected_err = ParseError::InvalidLength {
            expected: BOARD_SIZE,
            actual: 8,
        };
        assert_eq!(err, expected_err);
    }
}
