use crate::board::BoardSquare;
use crate::rand::Rand;

use deepsize::DeepSizeOf;
use serde::{Deserialize, Serialize};

use std::cmp::{Ord, PartialOrd};
use std::fmt::{Display, Formatter, Result};
use std::io;

// API structs
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Default, Serialize, Deserialize, DeepSizeOf)]
pub struct Coord {
    pub x: i8,
    pub y: i8,
}

impl Coord {
    pub fn new(x: i8, y: i8) -> Self {
        Coord { x, y }
    }
}

impl Display for Coord {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "({}, {})", self.x, self.y)?;
        Ok(())
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize, DeepSizeOf)]
#[serde(rename_all = "lowercase")]
pub enum Move {
    #[default]
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

pub fn square_to_char(sqr: BoardSquare, num_stacked: i32, mv: Option<Move>) -> char {
    match (sqr, num_stacked, mv) {
        (BoardSquare::Empty, ..) => '-',
        (BoardSquare::Food, ..) => '+',
        (BoardSquare::FoodHazard, ..) => '@',
        (BoardSquare::Hazard, ..) => '*',
        (BoardSquare::SnakeHead(idx), _, None) => (idx + 48) as char,
        (BoardSquare::SnakeHeadHazard(idx), _, None) => (idx + 83) as char,
        (BoardSquare::SnakeBody(_), 0, Some(Move::Left) | None) => '<',
        (BoardSquare::SnakeBody(_), 0, Some(Move::Right)) => '>',
        (BoardSquare::SnakeBody(_), 0, Some(Move::Up)) => '^',
        (BoardSquare::SnakeBody(_), 0, Some(Move::Down)) => 'v',
        (BoardSquare::SnakeBodyHazard(_), 0, Some(Move::Left) | None) => '{',
        (BoardSquare::SnakeBodyHazard(_), 0, Some(Move::Right)) => '}',
        (BoardSquare::SnakeBodyHazard(_), 0, Some(Move::Up)) => 'u',
        (BoardSquare::SnakeBodyHazard(_), 0, Some(Move::Down)) => 'n',
        (BoardSquare::SnakeTail(_), 0, Some(Move::Left) | None) => 'a',
        (BoardSquare::SnakeTail(_), 0, Some(Move::Right)) => 'b',
        (BoardSquare::SnakeTail(_), 0, Some(Move::Up)) => 'c',
        (BoardSquare::SnakeTail(_), 0, Some(Move::Down)) => 'd',
        (BoardSquare::SnakeTail(_), _, Some(Move::Left) | None) => 'e',
        (BoardSquare::SnakeTail(_), _, Some(Move::Right)) => 'f',
        (BoardSquare::SnakeTail(_), _, Some(Move::Up)) => 'g',
        (BoardSquare::SnakeTail(_), _, Some(Move::Down)) => 'h',
        (BoardSquare::SnakeTailHazard(_), 0, Some(Move::Left) | None) => 'A',
        (BoardSquare::SnakeTailHazard(_), 0, Some(Move::Right)) => 'B',
        (BoardSquare::SnakeTailHazard(_), 0, Some(Move::Up)) => 'C',
        (BoardSquare::SnakeTailHazard(_), 0, Some(Move::Down)) => 'D',
        (BoardSquare::SnakeTailHazard(_), _, Some(Move::Left) | None) => 'E',
        (BoardSquare::SnakeTailHazard(_), _, Some(Move::Right)) => 'F',
        (BoardSquare::SnakeTailHazard(_), _, Some(Move::Up)) => 'G',
        (BoardSquare::SnakeTailHazard(_), _, Some(Move::Down)) => 'H',

        (BoardSquare::SnakeBody(_), _, None) | (BoardSquare::SnakeBodyHazard(_), _, None) => {
            panic!("Body must have move")
        }
        _ => panic!("Invalid args {:?} {:?} {:?}", sqr, num_stacked, mv),
    }
}

pub fn char_to_square(chr: char) -> (BoardSquare, i32, Option<Move>) {
    let (basic_parse_result, num_stacked, mv) = match chr {
        '-' => (Some(BoardSquare::Empty), 0, None),
        '+' => (Some(BoardSquare::Food), 0, None),
        '@' => (Some(BoardSquare::FoodHazard), 0, None),
        '*' => (Some(BoardSquare::Hazard), 0, None),
        '<' => (Some(BoardSquare::SnakeBody(0)), 0, Some(Move::Left)),
        '>' => (Some(BoardSquare::SnakeBody(0)), 0, Some(Move::Right)),
        '^' => (Some(BoardSquare::SnakeBody(0)), 0, Some(Move::Up)),
        'v' => (Some(BoardSquare::SnakeBody(0)), 0, Some(Move::Down)),
        '{' => (Some(BoardSquare::SnakeBodyHazard(0)), 0, Some(Move::Left)),
        '}' => (Some(BoardSquare::SnakeBodyHazard(0)), 0, Some(Move::Right)),
        'n' => (Some(BoardSquare::SnakeBodyHazard(0)), 0, Some(Move::Up)),
        'u' => (Some(BoardSquare::SnakeBodyHazard(0)), 0, Some(Move::Down)),
        'a' => (Some(BoardSquare::SnakeTail(0)), 0, Some(Move::Left)),
        'b' => (Some(BoardSquare::SnakeTail(0)), 0, Some(Move::Right)),
        'c' => (Some(BoardSquare::SnakeTail(0)), 0, Some(Move::Up)),
        'd' => (Some(BoardSquare::SnakeTail(0)), 0, Some(Move::Down)),
        'e' => (Some(BoardSquare::SnakeTail(0)), 1, Some(Move::Left)),
        'f' => (Some(BoardSquare::SnakeTail(0)), 1, Some(Move::Right)),
        'g' => (Some(BoardSquare::SnakeTail(0)), 1, Some(Move::Up)),
        'h' => (Some(BoardSquare::SnakeTail(0)), 1, Some(Move::Down)),
        'A' => (Some(BoardSquare::SnakeTailHazard(0)), 0, Some(Move::Left)),
        'B' => (Some(BoardSquare::SnakeTailHazard(0)), 0, Some(Move::Right)),
        'C' => (Some(BoardSquare::SnakeTailHazard(0)), 0, Some(Move::Up)),
        'D' => (Some(BoardSquare::SnakeTailHazard(0)), 0, Some(Move::Down)),
        'E' => (Some(BoardSquare::SnakeTailHazard(0)), 1, Some(Move::Left)),
        'F' => (Some(BoardSquare::SnakeTailHazard(0)), 1, Some(Move::Right)),
        'G' => (Some(BoardSquare::SnakeTailHazard(0)), 1, Some(Move::Up)),
        'H' => (Some(BoardSquare::SnakeTailHazard(0)), 1, Some(Move::Down)),
        _ => (None, 0, None),
    };

    let chr_byte = chr as u8;

    if let Some(parse_result) = basic_parse_result {
        (parse_result, num_stacked, mv)
    } else if (48..56).contains(&chr_byte) {
        (BoardSquare::SnakeHead(chr as u8 - 48), 0, None)
    } else if (83..91).contains(&chr_byte) {
        (BoardSquare::SnakeHeadHazard(chr as u8 - 83), 0, None)
    } else {
        panic!("Invalid board character {}", chr)
    }
}

pub fn rand_move(r: &mut impl Rand) -> Move {
    let x = r.range(0, 3);
    Move::from_idx(x as usize)
}

pub fn rand_move_arr(r: &mut impl Rand) -> [Move; 4] {
    let mut move_list = MOVES;
    r.shuffle(&mut move_list, 4);
    move_list
}

pub fn rand_rem_moves(r: &mut impl Rand, mv_one: Option<Move>, mv_two: Option<Move>) -> [Move; 4] {
    let mut move_list = MOVES;

    let mut swap = |mv: Option<Move>, idx| {
        let mv_idx = mv.unwrap() as usize;
        let curr_val = move_list[idx];
        move_list[idx] = mv.unwrap();
        move_list[mv_idx] = curr_val;
    };

    let rem = if mv_one.is_some() && mv_two.is_some() {
        swap(mv_one, 0);
        swap(mv_two, 1);
        2
    } else if mv_one.is_some() {
        swap(mv_one, 0);
        3
    } else if mv_two.is_some() {
        swap(mv_two, 0);
        3
    } else {
        4
    };

    r.shuffle(&mut move_list[4 - rem..], rem);
    move_list
}
