use crate::board::BoardSquare;

use rand::{seq::SliceRandom, thread_rng, Rng};
use serde::{Deserialize, Serialize};

use std::cmp::{Ord, PartialOrd};
use std::fmt::{Display, Formatter, Result};
use std::slice::Iter;

// API structs
#[derive(Debug, Copy, Clone, Serialize, Deserialize, Eq, PartialEq, Default)]
pub struct Coord {
    pub x: i32,
    pub y: i32,
}

impl Coord {
    pub fn new(x: i32, y: i32) -> Self {
        Coord { x, y }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord)]
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

    pub fn iter() -> Iter<'static, Self> {
        MOVES.iter()
    }

    pub fn num_move_perm(num_snakes: usize) -> usize {
        1 << (num_snakes * 2)
    }

    pub fn get_perm_idx(move_idx: usize, snake_idx: usize) -> usize {
        (move_idx & (0x3 << (2 * snake_idx))) >> (2 * snake_idx)
    }

    pub fn idx(&self) -> usize {
        *self as usize
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
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

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
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

pub fn rand_move() -> Move {
    let mut rng = rand::thread_rng();
    let x = rng.gen_range(0..4);
    if x == 0 {
        Move::Left
    } else if x == 1 {
        Move::Right
    } else if x == 2 {
        Move::Up
    } else {
        Move::Down
    }
}

pub fn rand_move_arr() -> [Move; 4] {
    let mut move_list = MOVES;
    let mut rng = thread_rng();
    move_list.shuffle(&mut rng);
    move_list
}

pub fn max_children(max_snakes: i32) -> usize {
    4usize.pow((max_snakes) as u32)
}

// TODO: replace with mallinfo2, which doesn't wraparound
// Requires glibc >= 2.33
pub fn mem_usage() -> usize {
    unsafe { libc::mallinfo() }.uordblks as usize
}
