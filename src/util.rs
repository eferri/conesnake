use crate::api::ApiCoord;
use crate::board::BoardBit;
use crate::rand::Rand;

use serde::{Deserialize, Serialize};

use std::cmp::{Ord, PartialOrd};
use std::fmt::{Debug, Display, Formatter, Result};
use std::io;

// API structs
#[derive(Copy, Clone, Eq, PartialEq, Hash, Default, Serialize, Deserialize)]
pub struct Coord {
    pub val: u8,
}

impl Coord {
    pub fn new(x: i8, y: i8) -> Self {
        let mut crd = Coord { val: 0 };
        crd.set_x(x);
        crd.set_y(y);
        crd
    }

    pub fn x(&self) -> i8 {
        (self.val & 0xf) as i8
    }

    pub fn y(&self) -> i8 {
        (self.val >> 4) as i8
    }

    pub fn set_x(&mut self, x_val: i8) {
        self.val &= 0xf0;
        self.val |= (x_val as u8) & 0xf;
    }

    pub fn set_y(&mut self, y_val: i8) {
        self.val &= 0x0f;
        self.val |= ((y_val as u8) & 0xf) << 4;
    }

    pub fn to_api(&self) -> ApiCoord {
        ApiCoord {
            x: self.x(),
            y: self.y(),
        }
    }
}

impl Display for Coord {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "({}, {})", self.x(), self.y())?;
        Ok(())
    }
}

impl Debug for Coord {
    fn fmt(&self, f: &mut Formatter) -> Result {
        Display::fmt(self, f)
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Move {
    #[default]
    Left = 0,
    Right,
    Up,
    Down,
}
pub const MOVES: [Move; 4] = [Move::Left, Move::Right, Move::Up, Move::Down];
pub const MOVE_INCR: [u16; 4] = [0xff, 0x1, 0x100, 0xff00];

impl Move {
    pub fn from_idx(idx: usize) -> Self {
        MOVES[idx]
    }

    pub const fn num_perm(num_snakes: i32) -> u16 {
        // Equivalent to 4^(max_snakes)
        1 << (2 * num_snakes as u16)
    }

    // Extract the move index from snake at index snake_idx,
    pub fn extract_idx(moves: u16, snake_idx: u32) -> u16 {
        (moves & (0x3 << (2 * snake_idx))) >> (2 * snake_idx)
    }

    pub fn extract(moves: u16, snake_idx: u32) -> Self {
        Self::from_idx(Self::extract_idx(moves, snake_idx) as usize)
    }

    pub fn set_move(moves: u16, snake_idx: u32, mv: Self) -> u16 {
        ((mv as u16) << (2 * snake_idx)) | (!(0x3 << (2 * snake_idx)) & moves)
    }

    // Encode a list of snake-moves in a u16
    pub fn encode(moves: &[Self]) -> u16 {
        let mut encoded_moves = 0;
        for (idx, mv) in moves.iter().enumerate() {
            encoded_moves = Self::set_move(encoded_moves, idx as u32, *mv);
        }
        encoded_moves
    }

    pub fn decode(moves: u16, num_snakes: i32) -> Vec<Self> {
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
    RequestError(String),
    BadBoard(String),
    BadBoardReq(String),
    BadBoardStr(String),
    LockHeld(String),
    WorkerError(String),
    ResourceError(String),
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

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::RequestError(e.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            Error::IoError(s) => write!(f, "IoError: {s}"),
            Error::SerdeError(s) => write!(f, "SerdeError: {s}"),
            Error::RequestError(s) => write!(f, "RequestError: {s}"),
            Error::BadBoard(s) => write!(f, "BadBoard: {s}"),
            Error::BadBoardReq(s) => write!(f, "BadBoardReq: {s}"),
            Error::BadBoardStr(s) => write!(f, "BadBoardStr: {s}"),
            Error::LockHeld(s) => write!(f, "LockHeld: {s}"),
            Error::WorkerError(s) => write!(f, "WorkerError: {s}"),
            Error::ResourceError(s) => write!(f, "ResourceError: {s}"),
        }
    }
}

pub fn square_to_char(sqr: u8, idx: u8, num_stacked: i32, mv: Option<Move>) -> char {
    let sqr_enum = BoardBit::from_repr(sqr & !(BoardBit::SnakeIdx as u8)).unwrap();

    match (sqr_enum, num_stacked, mv) {
        (BoardBit::Empty, ..) => '-',
        (BoardBit::Food, ..) => '+',
        (BoardBit::FoodHazard, ..) => '@',
        (BoardBit::Hazard, ..) => '*',
        (BoardBit::SnakeHead, _, None) => (idx + 48) as char,
        (BoardBit::SnakeHeadHazard, _, None) => (idx + 83) as char,
        (BoardBit::SnakeBody, 0, Some(Move::Left) | None) => '<',
        (BoardBit::SnakeBody, 0, Some(Move::Right)) => '>',
        (BoardBit::SnakeBody, 0, Some(Move::Up)) => '^',
        (BoardBit::SnakeBody, 0, Some(Move::Down)) => 'v',
        (BoardBit::SnakeBodyHazard, 0, Some(Move::Left) | None) => '{',
        (BoardBit::SnakeBodyHazard, 0, Some(Move::Right)) => '}',
        (BoardBit::SnakeBodyHazard, 0, Some(Move::Up)) => 'u',
        (BoardBit::SnakeBodyHazard, 0, Some(Move::Down)) => 'n',
        (BoardBit::SnakeTail, 0, Some(Move::Left) | None) => 'a',
        (BoardBit::SnakeTail, 0, Some(Move::Right)) => 'b',
        (BoardBit::SnakeTail, 0, Some(Move::Up)) => 'c',
        (BoardBit::SnakeTail, 0, Some(Move::Down)) => 'd',
        (BoardBit::SnakeTail, _, Some(Move::Left) | None) => 'e',
        (BoardBit::SnakeTail, _, Some(Move::Right)) => 'f',
        (BoardBit::SnakeTail, _, Some(Move::Up)) => 'g',
        (BoardBit::SnakeTail, _, Some(Move::Down)) => 'h',
        (BoardBit::SnakeTailHazard, 0, Some(Move::Left) | None) => 'A',
        (BoardBit::SnakeTailHazard, 0, Some(Move::Right)) => 'B',
        (BoardBit::SnakeTailHazard, 0, Some(Move::Up)) => 'C',
        (BoardBit::SnakeTailHazard, 0, Some(Move::Down)) => 'D',
        (BoardBit::SnakeTailHazard, _, Some(Move::Left) | None) => 'E',
        (BoardBit::SnakeTailHazard, _, Some(Move::Right)) => 'F',
        (BoardBit::SnakeTailHazard, _, Some(Move::Up)) => 'G',
        (BoardBit::SnakeTailHazard, _, Some(Move::Down)) => 'H',

        (BoardBit::SnakeBody, _, None) | (BoardBit::SnakeBodyHazard, _, None) => {
            println!("ERROR: Body must have move");
            '!'
        }
        _ => {
            println!("ERROR: Invalid args sqr: {sqr:?} num_stacked: {num_stacked} mv: {mv:?}");
            '!'
        }
    }
}

pub fn char_to_square(chr: char) -> (BoardBit, u8, i32, Option<Move>) {
    let (basic_parse_result, num_stacked, mv) = match chr {
        '-' => (Some(BoardBit::Empty), 0, None),
        '+' => (Some(BoardBit::Food), 0, None),
        '@' => (Some(BoardBit::FoodHazard), 0, None),
        '*' => (Some(BoardBit::Hazard), 0, None),
        '<' => (Some(BoardBit::SnakeBody), 0, Some(Move::Left)),
        '>' => (Some(BoardBit::SnakeBody), 0, Some(Move::Right)),
        '^' => (Some(BoardBit::SnakeBody), 0, Some(Move::Up)),
        'v' => (Some(BoardBit::SnakeBody), 0, Some(Move::Down)),
        '{' => (Some(BoardBit::SnakeBodyHazard), 0, Some(Move::Left)),
        '}' => (Some(BoardBit::SnakeBodyHazard), 0, Some(Move::Right)),
        'n' => (Some(BoardBit::SnakeBodyHazard), 0, Some(Move::Up)),
        'u' => (Some(BoardBit::SnakeBodyHazard), 0, Some(Move::Down)),
        'a' => (Some(BoardBit::SnakeTail), 0, Some(Move::Left)),
        'b' => (Some(BoardBit::SnakeTail), 0, Some(Move::Right)),
        'c' => (Some(BoardBit::SnakeTail), 0, Some(Move::Up)),
        'd' => (Some(BoardBit::SnakeTail), 0, Some(Move::Down)),
        'e' => (Some(BoardBit::SnakeTail), 1, Some(Move::Left)),
        'f' => (Some(BoardBit::SnakeTail), 1, Some(Move::Right)),
        'g' => (Some(BoardBit::SnakeTail), 1, Some(Move::Up)),
        'h' => (Some(BoardBit::SnakeTail), 1, Some(Move::Down)),
        'A' => (Some(BoardBit::SnakeTailHazard), 0, Some(Move::Left)),
        'B' => (Some(BoardBit::SnakeTailHazard), 0, Some(Move::Right)),
        'C' => (Some(BoardBit::SnakeTailHazard), 0, Some(Move::Up)),
        'D' => (Some(BoardBit::SnakeTailHazard), 0, Some(Move::Down)),
        'E' => (Some(BoardBit::SnakeTailHazard), 1, Some(Move::Left)),
        'F' => (Some(BoardBit::SnakeTailHazard), 1, Some(Move::Right)),
        'G' => (Some(BoardBit::SnakeTailHazard), 1, Some(Move::Up)),
        'H' => (Some(BoardBit::SnakeTailHazard), 1, Some(Move::Down)),
        _ => (None, 0, None),
    };

    let chr_byte = chr as u8;

    if let Some(parse_result) = basic_parse_result {
        (parse_result, 0, num_stacked, mv)
    } else if (48..56).contains(&chr_byte) {
        (BoardBit::SnakeHead, chr as u8 - 48, 0, None)
    } else if (83..91).contains(&chr_byte) {
        (BoardBit::SnakeHeadHazard, chr as u8 - 83, 0, None)
    } else {
        panic!("Invalid board character {chr}")
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
