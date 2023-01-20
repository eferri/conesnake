use crate::api::{BattleState, BoardApi, SnakeApi};
use crate::game::{Game, Map, Rules};
use crate::util::{self};
use crate::util::{Coord, Error, Move};

use std::cmp::Ordering;
use std::{
    cmp::min,
    cmp::min_by,
    collections::{HashSet, VecDeque},
    fmt,
    fmt::Write,
    str,
};

use deepsize::DeepSizeOf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Eq, DeepSizeOf)]
pub struct Snake {
    pub health: i32,
    pub eliminated: bool,
    pub head: Coord,
    pub body: VecDeque<Coord>,
}

impl Snake {
    pub fn new(max_board_size: usize) -> Self {
        Self {
            health: 0,
            eliminated: false,
            head: Default::default(),
            body: VecDeque::with_capacity(max_board_size + 2),
        }
    }

    pub fn alive(&self) -> bool {
        self.health > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, DeepSizeOf)]
#[repr(u8)]
pub enum BoardSquare {
    Empty,
    SnakeHead(u8),       // index of snake
    SnakeHeadHazard(u8), // index of snake
    SnakeBody(u8),       // index of snake
    SnakeBodyHazard(u8), // index of snake
    SnakeTail(u8),       // index of snake
    SnakeTailHazard(u8), // index of snake
    Food,
    FoodHazard,
    Hazard,
}

impl BoardSquare {
    pub fn tag_val(&self) -> u16 {
        // https://rust-lang.github.io/unsafe-code-guidelines/layout/enums.html#explicit-repr-annotation-without-c-compatibility
        let sqr_ptr = self as *const BoardSquare as *const u8;
        unsafe { *sqr_ptr as u16 }
    }

    pub fn idx_val(&self) -> u16 {
        let sqr_ptr = self as *const BoardSquare as *const u8;
        unsafe { *(sqr_ptr.offset(1)) as u16 }
    }

    pub fn from_raw(tag: u16, idx: u16) -> Self {
        let mut sqr = BoardSquare::Empty;
        let sqr_ptr = &mut sqr as *mut BoardSquare as *mut u8;
        unsafe {
            *sqr_ptr = tag as u8;
            *sqr_ptr.offset(1) = idx as u8;
        }
        sqr
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, DeepSizeOf)]
pub enum HeadOnCol {
    None,
    PossibleCollision,
    PossibleElimination,
}

#[derive(Debug, Clone, PartialEq, Eq, DeepSizeOf)]
pub struct Board {
    pub width: i32,
    pub height: i32,
    pub turn: i32,
    pub num_snakes: i32,
    pub num_food: i32,

    pub royale_min_x: i32,
    pub royale_max_x: i32,
    pub royale_min_y: i32,
    pub royale_max_y: i32,

    pub food: HashSet<Coord>,
    pub snakes: Vec<Snake>,

    board_mat: Vec<BoardSquare>,
}

impl Board {
    pub fn new(width: i32, height: i32, max_width: i32, max_height: i32, max_snakes: i32) -> Self {
        let max_board_size = (max_width * max_height) as usize;
        let max_snakes = max_snakes as usize;
        let mut board = Self {
            width,
            height,
            turn: 1,
            num_snakes: 0,
            num_food: 0,
            royale_min_x: 0,
            royale_max_x: 0,
            royale_min_y: 0,
            royale_max_y: 0,
            food: HashSet::with_capacity(max_board_size),
            snakes: Vec::with_capacity(max_snakes),
            board_mat: vec![BoardSquare::Empty; max_board_size],
        };

        // Don't use vec![] here so snake body capacity is reserved
        board.snakes.resize_with(max_snakes, || Snake::new(max_board_size));

        board
    }

    pub fn from_req(
        game: &Game,
        req: &BattleState,
        max_width: i32,
        max_height: i32,
        max_snakes: i32,
    ) -> Result<Board, Error> {
        if req.board.snakes.is_empty() {
            return Err(Error::BadBoardReq("No snakes in request".to_owned()));
        }

        let mut board = Board::new(req.board.width, req.board.height, max_width, max_height, max_snakes);
        for coord in req.board.food.iter() {
            board.food.insert(*coord);
            board.set_at(*coord, BoardSquare::Food);
            board.num_food += 1;
        }

        for coord in req.board.hazards.iter() {
            match board.at(*coord) {
                BoardSquare::Empty => board.set_at(*coord, BoardSquare::Hazard),
                BoardSquare::Food => board.set_at(*coord, BoardSquare::FoodHazard),
                _ => (),
            };
        }

        board.turn = req.turn;
        let our_id = req.you.id.clone();
        board.add_api_snake(game, &req.you)?;

        for snake in req.board.snakes.iter() {
            if our_id == snake.id {
                continue;
            }

            board.add_api_snake(game, snake)?;
        }

        if let Map::Royale = req.game.map {
            board.set_royale();
        }

        Ok(board)
    }

    pub fn to_req(&self, game: &Game) -> Result<BattleState, Error> {
        let mut food = Vec::new();
        let mut hazards = Vec::new();
        let mut snakes = Vec::new();

        for i in 0..self.len() {
            let coord = self.coord_from_idx(i as usize);
            match self.at(coord) {
                BoardSquare::Food => food.push(coord),
                BoardSquare::Hazard
                | BoardSquare::SnakeHeadHazard(_)
                | BoardSquare::SnakeBodyHazard(_)
                | BoardSquare::SnakeTailHazard(_) => hazards.push(coord),
                BoardSquare::FoodHazard => {
                    food.push(coord);
                    hazards.push(coord);
                }
                _ => (),
            }
        }

        if self.snakes.is_empty() {
            return Err(Error::BadBoard("No snakes".to_owned()));
        }

        for idx in 0..self.num_snakes() as usize {
            let api_snake = SnakeApi {
                id: idx.to_string(),
                name: idx.to_string(),
                body: Vec::from(self.snakes[idx].body.clone()),
                head: self.snakes[idx].body[0],
                health: self.snakes[idx].health,
                latency: "0".to_owned(),
                length: self.snake_len(idx),
                shout: None,
                squad: "".to_owned(),
                customizations: Default::default(),
            };

            snakes.push(api_snake);
        }

        Ok(BattleState {
            game: game.api.clone(),
            turn: self.turn,
            you: snakes[0].clone(),
            board: BoardApi {
                height: self.height,
                width: self.width,
                food,
                hazards,
                snakes,
            },
        })
    }

    pub fn set(&mut self, other: &Board) {
        self.width = other.width;
        self.height = other.height;
        self.turn = other.turn;
        self.num_food = other.num_food;
        self.num_snakes = other.num_snakes;

        let board_len = (self.width * self.height) as usize;

        self.food.clear();
        self.food.extend(&other.food);

        for s_idx in 0..other.num_snakes {
            let snake = &mut self.snakes[s_idx as usize];
            let other = &other.snakes[s_idx as usize];

            snake.health = other.health;
            snake.eliminated = other.eliminated;

            snake.head = other.head;

            snake.body.clear();
            snake.body.extend(&other.body);
        }

        self.board_mat[..board_len].copy_from_slice(&other.board_mat[..board_len]);
    }

    pub fn set_royale(&mut self) {
        self.royale_min_x = 0;
        self.royale_max_x = self.width - 1;

        self.royale_min_y = 0;
        self.royale_max_y = self.height - 1;

        'side_loop: for (side, iter_dim) in [
            (Move::Left, self.height),
            (Move::Right, self.height),
            (Move::Up, self.width),
            (Move::Down, self.width),
        ] {
            loop {
                let side_val = match side {
                    Move::Left => self.royale_min_x,
                    Move::Right => self.royale_max_x,
                    Move::Up => self.royale_min_y,
                    Move::Down => self.royale_max_y,
                };

                for z in 0..iter_dim {
                    let coord = if iter_dim == self.height {
                        Coord::new(side_val as i8, z as i8)
                    } else {
                        Coord::new(z as i8, side_val as i8)
                    };

                    match self.at(coord) {
                        BoardSquare::Empty
                        | BoardSquare::Food
                        | BoardSquare::SnakeHead(_)
                        | BoardSquare::SnakeBody(_)
                        | BoardSquare::SnakeTail(_) => {
                            continue 'side_loop;
                        }
                        _ => (),
                    }
                }

                match side {
                    Move::Left => self.royale_min_x += 1,
                    Move::Right => self.royale_max_x -= 1,
                    Move::Up => self.royale_min_y += 1,
                    Move::Down => self.royale_max_y -= 1,
                }
            }
        }
    }

    pub fn num_snakes(&self) -> i32 {
        self.num_snakes
    }

    pub fn num_alive_snakes(&self) -> i32 {
        let mut alive = 0;
        for s_idx in 0..self.num_snakes() as usize {
            if self.snakes[s_idx].alive() {
                alive += 1;
            }
        }
        alive
    }

    pub fn max_snakes(&self) -> i32 {
        self.snakes.len() as i32
    }

    pub fn num_food(&self) -> i32 {
        self.num_food
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> i32 {
        self.width * self.height
    }

    fn resize(&mut self, max_width: i32, max_height: i32) {
        let mat_size = (max_width * max_height) as usize;
        self.board_mat.resize(mat_size, BoardSquare::Empty);
    }

    fn set_size(&mut self, w: i32, h: i32) {
        assert!(w * h <= self.board_mat.len() as i32);

        self.width = w;
        self.height = h;
    }

    fn add_snake(&mut self, body: &[Coord], health: i32) {
        self.snakes[self.num_snakes as usize].body.clear();

        for coord in body.iter() {
            self.snakes[self.num_snakes as usize].body.push_back(*coord);
        }

        if !body.is_empty() {
            self.snakes[self.num_snakes as usize].head = body[0];
        }

        self.snakes[self.num_snakes as usize].health = health;
        self.snakes[self.num_snakes as usize].eliminated = false;

        self.num_snakes += 1;
    }

    pub fn add_api_snake(&mut self, game: &Game, api_snake: &SnakeApi) -> Result<(), Error> {
        let snake_idx = self.num_snakes() as u8;

        self.add_snake(&api_snake.body, api_snake.health);

        if !self.snakes[snake_idx as usize].alive() {
            return Ok(());
        }

        let mut prev_coord: Option<Coord> = None;

        for (i, coord) in api_snake.body.iter().enumerate() {
            if let Some(p) = prev_coord {
                if *coord != p && !self.next_to(*coord, p, game.ruleset) {
                    return Err(Error::BadBoard("Snake was not contiguous".to_owned()));
                }
            }

            let coord_idx = self.idx_from_coord(*coord);

            match (self.board_mat[coord_idx], i) {
                (BoardSquare::Empty, 0) => {
                    self.board_mat[coord_idx] = BoardSquare::SnakeHead(snake_idx);
                }
                (BoardSquare::Hazard, 0) => {
                    self.board_mat[coord_idx] = BoardSquare::SnakeHeadHazard(snake_idx);
                }
                (BoardSquare::Empty, x) => {
                    if x < api_snake.body.len() - 1 {
                        self.board_mat[coord_idx] = BoardSquare::SnakeBody(snake_idx);
                    } else {
                        self.board_mat[coord_idx] = BoardSquare::SnakeTail(snake_idx);
                    }
                }
                (BoardSquare::Hazard, x) => {
                    if x < api_snake.body.len() - 1 {
                        self.board_mat[coord_idx] = BoardSquare::SnakeBodyHazard(snake_idx);
                    } else {
                        self.board_mat[coord_idx] = BoardSquare::SnakeTailHazard(snake_idx);
                    }
                }
                (BoardSquare::Food, _) | (BoardSquare::FoodHazard, _) => {
                    return Err(Error::BadBoard("Snake square conflicts with Food".to_owned()))
                }
                (BoardSquare::SnakeBody(idx), _) => {
                    if idx != snake_idx {
                        return Err(Error::BadBoard(
                            "Snake square conflicts with other SnakeBody".to_owned(),
                        ));
                    }
                    self.board_mat[coord_idx] = BoardSquare::SnakeTail(snake_idx);
                }
                (BoardSquare::SnakeBodyHazard(idx), _) => {
                    if idx != snake_idx {
                        return Err(Error::BadBoard(
                            "Snake square conflicts with other SnakeBodyHazard".to_owned(),
                        ));
                    }
                    self.board_mat[coord_idx] = BoardSquare::SnakeTailHazard(snake_idx);
                }
                (BoardSquare::SnakeHead(idx), _)
                | (BoardSquare::SnakeHeadHazard(idx), _)
                | (BoardSquare::SnakeTail(idx), _)
                | (BoardSquare::SnakeTailHazard(idx), _) => {
                    if idx != snake_idx {
                        return Err(Error::BadBoard(
                            "Snake square conflicts with other SnakeHead/SnakeTail".to_owned(),
                        ));
                    }
                }
            }
            prev_coord = Some(*coord);
        }
        Ok(())
    }

    pub fn snake_head(&self, snake_idx: usize) -> Coord {
        self.snakes[snake_idx].head
    }

    pub fn snake_tail(&self, snake_idx: usize) -> Coord {
        *self.snakes[snake_idx].body.back().unwrap()
    }

    pub fn snake_len(&self, snake_idx: usize) -> i32 {
        self.snakes[snake_idx].body.len() as i32
    }

    pub fn at(&self, loc: Coord) -> BoardSquare {
        self.board_mat[self.idx_from_coord(loc)]
    }

    fn idx_from_coord(&self, loc: Coord) -> usize {
        (loc.x as i32 + (loc.y as i32) * self.width) as usize
    }

    pub fn coord_from_idx(&self, idx: usize) -> Coord {
        Coord {
            x: (idx as i32 % self.width) as i8,
            y: (idx as i32 / self.width) as i8,
        }
    }

    pub fn set_at(&mut self, loc: Coord, val: BoardSquare) {
        let idx = self.idx_from_coord(loc);
        self.board_mat[idx] = val;
    }

    pub fn on_board(&self, square: Coord) -> bool {
        !(square.x < 0 || square.x as i32 >= self.width || square.y < 0 || square.y as i32 >= self.height)
    }

    pub fn move_to_coord(&self, head: Coord, mv: Move, rules: Rules) -> Coord {
        let mut square = Coord { x: head.x, y: head.y };
        match mv {
            Move::Left => square.x = head.x - 1,
            Move::Right => square.x = head.x + 1,
            Move::Up => square.y = head.y + 1,
            Move::Down => square.y = head.y - 1,
        };

        if let Rules::Wrapped = rules {
            square.x = square.x.rem_euclid(self.width as i8);
            square.y = square.y.rem_euclid(self.height as i8);
        }

        square
    }

    pub fn coord_to_move(&self, orig: Coord, dest: Coord, rules: Rules) -> (Option<Move>, Option<Move>) {
        let mut diff_x = dest.x as i32 - orig.x as i32;
        let mut diff_y = dest.y as i32 - orig.y as i32;

        if let Rules::Wrapped = rules {
            let diff_x_wrapped = if diff_x < 0 {
                self.width - orig.x as i32 + dest.x as i32
            } else {
                -(orig.x as i32 + self.width - dest.x as i32)
            };

            diff_x = min_by(diff_x, diff_x_wrapped, |a, b| a.abs().cmp(&b.abs()));

            let diff_y_wrapped = if diff_y < 0 {
                self.height - orig.y as i32 + dest.y as i32
            } else {
                -(orig.y as i32 + self.height - dest.y as i32)
            };
            diff_y = min_by(diff_y, diff_y_wrapped, |a, b| a.abs().cmp(&b.abs()));
        }

        let mv_x = match diff_x.cmp(&0) {
            Ordering::Greater => Some(Move::Right),
            Ordering::Less => Some(Move::Left),
            Ordering::Equal => None,
        };

        let mv_y = match diff_y.cmp(&0) {
            Ordering::Greater => Some(Move::Up),
            Ordering::Less => Some(Move::Down),
            Ordering::Equal => None,
        };

        if diff_y.abs() > diff_x.abs() {
            (mv_y, mv_x)
        } else {
            (mv_x, mv_y)
        }
    }

    pub fn next_to(&self, square_1: Coord, square_2: Coord, rules: Rules) -> bool {
        let (diff_x, diff_y) = self.abs_dist(square_1, square_2, rules);

        match (diff_x, diff_y) {
            (1, 0) => true,
            (0, 1) => true,
            (_, _) => false,
        }
    }

    pub fn abs_dist(&self, square_1: Coord, square_2: Coord, rules: Rules) -> (i32, i32) {
        let mut diff_x = (square_2.x as i32 - square_1.x as i32).abs();
        let mut diff_y = (square_2.y as i32 - square_1.y as i32).abs();

        if let Rules::Wrapped = rules {
            diff_x = min(diff_x, self.width - diff_x);
            diff_y = min(diff_y, self.height - diff_y);
        }

        (diff_x, diff_y)
    }
}

pub mod board_rules;
pub mod board_str;

#[cfg(test)]
mod board_test;

#[cfg(test)]
mod gen_board_test;
