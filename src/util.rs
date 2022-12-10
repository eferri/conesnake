use crate::board::BoardSquare;
use crate::rand::Rand;

use deepsize::DeepSizeOf;
use serde::{Deserialize, Serialize};

use std::cmp::{Ord, PartialOrd};
use std::fmt::{Display, Formatter, Result};
use std::io;

// API structs
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, Serialize, Deserialize, DeepSizeOf)]
pub struct Coord {
    pub x: i32,
    pub y: i32,
}

impl Coord {
    pub fn new(x: i32, y: i32) -> Self {
        Coord { x, y }
    }
}

impl Display for Coord {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "({}, {})", self.x, self.y)?;
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize, DeepSizeOf)]
#[serde(rename_all = "lowercase")]
pub enum Move {
    Left = 0,
    Right,
    Up,
    Down,
}
pub const MOVES: [Move; 4] = [Move::Left, Move::Right, Move::Up, Move::Down];

impl Move {
    pub fn from_idx(idx: usize) -> Self {
        MOVES[idx]
    }

    pub fn num_perm(num_snakes: i32) -> u32 {
        // Equivalent to 4^(max_snakes)
        1 << (2 * num_snakes)
    }

    // Extract the move index from snake at index snake_idx,
    pub fn extract_idx(moves: u32, snake_idx: u32) -> u32 {
        (moves & (0x3 << (2 * snake_idx))) >> (2 * snake_idx)
    }

    pub fn extract(moves: u32, snake_idx: u32) -> Self {
        Self::from_idx(Self::extract_idx(moves, snake_idx) as usize)
    }

    pub fn set_move(moves: u32, snake_idx: u32, mv: Self) -> u32 {
        ((mv as u32) << (2 * snake_idx)) | (!(0x3 << (2 * snake_idx)) & moves)
    }

    pub fn encode(moves: &[Self]) -> u32 {
        let mut encoded_moves = 0;
        for (idx, mv) in moves.iter().enumerate() {
            encoded_moves = Self::set_move(encoded_moves, idx as u32, *mv);
        }
        encoded_moves
    }

    pub fn decode(moves: u32, num_snakes: i32) -> Vec<Self> {
        let mut moves_vec = Vec::with_capacity(num_snakes as usize);
        for idx in 0..(num_snakes as u32) {
            moves_vec.push(Self::extract(moves, idx));
        }
        moves_vec
    }

    pub fn idx(&self) -> usize {
        *self as usize
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let mv_str = match self {
            Move::Left => "Left",
            Move::Right => "Right",
            Move::Up => "Up",
            Move::Down => "Down",
        };
        Formatter::pad(f, mv_str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    IoError(String),
    SerdeError(String),
    BadBoard(String),
    BadBoardReq(String),
    BadBoardStr(String),
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::SerdeError(e.to_string())
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IoError(e.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            Error::IoError(s) => write!(f, "IoError: {}", s),
            Error::SerdeError(s) => write!(f, "SerdeError: {}", s),
            Error::BadBoard(s) => write!(f, "BadBoard: {}", s),
            Error::BadBoardReq(s) => write!(f, "BadBoardReq: {}", s),
            Error::BadBoardStr(s) => write!(f, "BadBoardStr: {}", s),
        }
    }
}

// These functions assume utf-8. Not currently checked
pub fn square_to_char(sqr: BoardSquare) -> char {
    match sqr {
        BoardSquare::Empty => '-',
        BoardSquare::Food => '+',
        BoardSquare::Hazard => '*',
        BoardSquare::SnakeHead(i, _) => (48 + i) as char,
        BoardSquare::SnakeBody(_, Move::Left) => '<',
        BoardSquare::SnakeBody(_, Move::Right) => '>',
        BoardSquare::SnakeBody(_, Move::Up) => '^',
        BoardSquare::SnakeBody(_, Move::Down) => 'v',
        BoardSquare::SnakeTail(_, Move::Left, 0) => 'l',
        BoardSquare::SnakeTail(_, Move::Right, 0) => 'r',
        BoardSquare::SnakeTail(_, Move::Up, 0) => 'u',
        BoardSquare::SnakeTail(_, Move::Down, 0) => 'd',
        BoardSquare::SnakeTail(_, Move::Left, 1) => 'L',
        BoardSquare::SnakeTail(_, Move::Right, 1) => 'R',
        BoardSquare::SnakeTail(_, Move::Up, 1) => 'U',
        BoardSquare::SnakeTail(_, Move::Down, 1) => 'D',
        BoardSquare::SnakeTail(_, Move::Left, _) => 'W',
        BoardSquare::SnakeTail(_, Move::Right, _) => 'X',
        BoardSquare::SnakeTail(_, Move::Up, _) => 'Y',
        BoardSquare::SnakeTail(_, Move::Down, _) => 'Z',
    }
}

pub fn char_to_square(chr: char) -> BoardSquare {
    let basic_parse_result = match chr {
        '-' => Some(BoardSquare::Empty),
        '+' => Some(BoardSquare::Food),
        '*' => Some(BoardSquare::Hazard),
        '<' => Some(BoardSquare::SnakeBody(0, Move::Left)),
        '>' => Some(BoardSquare::SnakeBody(0, Move::Right)),
        '^' => Some(BoardSquare::SnakeBody(0, Move::Up)),
        'v' => Some(BoardSquare::SnakeBody(0, Move::Down)),
        'l' => Some(BoardSquare::SnakeTail(0, Move::Left, 0)),
        'r' => Some(BoardSquare::SnakeTail(0, Move::Right, 0)),
        'u' => Some(BoardSquare::SnakeTail(0, Move::Up, 0)),
        'd' => Some(BoardSquare::SnakeTail(0, Move::Down, 0)),
        'L' => Some(BoardSquare::SnakeTail(0, Move::Left, 1)),
        'R' => Some(BoardSquare::SnakeTail(0, Move::Right, 1)),
        'U' => Some(BoardSquare::SnakeTail(0, Move::Up, 1)),
        'D' => Some(BoardSquare::SnakeTail(0, Move::Down, 1)),
        'W' => Some(BoardSquare::SnakeTail(0, Move::Left, 2)),
        'X' => Some(BoardSquare::SnakeTail(0, Move::Right, 2)),
        'Y' => Some(BoardSquare::SnakeTail(0, Move::Up, 2)),
        'Z' => Some(BoardSquare::SnakeTail(0, Move::Down, 2)),
        _ => None,
    };

    if let Some(parse_result) = basic_parse_result {
        parse_result
    } else {
        BoardSquare::SnakeHead(chr as u8 - 48, 0)
    }
}

pub fn rand_move(r: &mut impl Rand) -> Move {
    let x = r.range(0, 3);
    Move::from_idx(x as usize)
}

pub fn rand_move_arr(r: &mut impl Rand) -> [Move; 4] {
    let mut move_list = MOVES;
    r.shuffle(&mut move_list);
    move_list
}
