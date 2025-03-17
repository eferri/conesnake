use crate::api::{BattleState, BoardApi, SnakeApi};
use crate::config::{MAX_BOARD_SIZE, MAX_SNAKES};
use crate::game::{Game, Map, Rules};
use crate::util::{self};
use crate::util::{Coord, Error, Move};

use std::cmp::{min, min_by, Ordering};
use std::{fmt::Write, str};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Snake {
    pub health: i32,
    pub eliminated: bool,
    pub len: i32,
    pub tail: Coord,
    pub num_stacked: i32,
    pub head: Coord,
    pub old_head: Coord,
}

impl Default for Snake {
    fn default() -> Self {
        Self::new()
    }
}

impl Snake {
    pub fn new() -> Self {
        Self {
            health: 0,
            eliminated: false,
            len: 0,
            head: Coord::new(0, 0),
            tail: Coord::new(0, 0),
            num_stacked: 0,
            old_head: Coord::new(0, 0),
        }
    }

    pub fn alive(&self) -> bool {
        self.health > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum BoardSquare {
    Empty,
    SnakeHead(u8),             // index of snake
    SnakeHeadHazard(u8),       // index of snake
    SnakeBody(u8, Move),       // index of snake
    SnakeBodyHazard(u8, Move), // index of snake
    SnakeTail(u8, Move),       // index of snake
    SnakeTailHazard(u8, Move), // index of snake
    Food,
    FoodHazard,
    Hazard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadOnCol {
    None,
    PossibleCollision,
    PossibleElimination,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

    pub snakes: [Snake; MAX_SNAKES],

    board_mat: [BoardSquare; MAX_BOARD_SIZE],
}

impl Board {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            turn: 1,
            num_snakes: 0,
            num_food: 0,
            royale_min_x: 0,
            royale_max_x: 0,
            royale_min_y: 0,
            royale_max_y: 0,
            snakes: [Snake::new(); MAX_SNAKES],
            board_mat: [BoardSquare::Empty; MAX_BOARD_SIZE],
        }
    }

    pub fn from_req(game: &Game, req: &BattleState) -> Result<Board, Error> {
        if req.board.snakes.is_empty() {
            return Err(Error::BadBoardReq("No snakes in request".to_owned()));
        }

        let mut board = Board::new(req.board.width, req.board.height);
        for coord in req.board.food.iter() {
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
                | BoardSquare::SnakeBodyHazard(_, _)
                | BoardSquare::SnakeTailHazard(_, _) => hazards.push(coord),
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
            let snake = &self.snakes[idx];

            let mut api_snake_body = Vec::with_capacity(snake.len as usize);

            let mut curr_coord = snake.tail;
            loop {
                let mv = match self.at(curr_coord) {
                    BoardSquare::SnakeHead(_) | BoardSquare::SnakeHeadHazard(_) => {
                        // All heads case
                        let n = if snake.head == snake.tail { 3 } else { 1 };
                        for _ in 0..n {
                            api_snake_body.push(curr_coord);
                        }
                        break;
                    }
                    BoardSquare::SnakeBody(_, mv) | BoardSquare::SnakeBodyHazard(_, mv) => {
                        api_snake_body.push(curr_coord);
                        mv
                    }
                    BoardSquare::SnakeTail(_, mv) | BoardSquare::SnakeTailHazard(_, mv) => {
                        for _ in 0..(snake.num_stacked + 1) {
                            api_snake_body.push(curr_coord);
                        }
                        mv
                    }
                    _ => return Err(Error::BadBoard("Snake was not contiguous".to_owned())),
                };
                curr_coord = self.move_to_coord(curr_coord, mv, game.ruleset);
            }

            api_snake_body.reverse();

            let api_snake = SnakeApi {
                id: idx.to_string(),
                name: idx.to_string(),
                body: api_snake_body,
                head: snake.head,
                health: snake.health,
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

    pub fn set_from(&mut self, other: &Board) {
        if std::ptr::eq(self, other) {
            panic!("Cannot set from self");
        }

        self.width = other.width;
        self.height = other.height;
        self.turn = other.turn;
        self.num_food = other.num_food;
        self.num_snakes = other.num_snakes;

        self.royale_max_x = other.royale_max_x;
        self.royale_min_x = other.royale_min_x;
        self.royale_max_y = other.royale_max_y;
        self.royale_min_y = other.royale_min_y;

        let board_len = (self.width * self.height) as usize;

        for s_idx in 0..other.num_snakes {
            let snake = &mut self.snakes[s_idx as usize];
            let other_snake = &other.snakes[s_idx as usize];

            snake.health = other_snake.health;
            snake.eliminated = other_snake.eliminated;

            snake.len = other_snake.len;
            snake.head = other_snake.head;
            snake.tail = other_snake.tail;
            snake.num_stacked = other_snake.num_stacked;
            snake.old_head = other_snake.old_head;
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
                        | BoardSquare::SnakeBody(_, _)
                        | BoardSquare::SnakeTail(_, _) => {
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

    fn set_size(&mut self, w: i32, h: i32) {
        assert!(w * h <= self.board_mat.len() as i32);

        self.width = w;
        self.height = h;
    }

    fn add_snake(&mut self, body: &[Coord], health: i32) {
        self.snakes[self.num_snakes as usize].len = body.len() as i32;

        assert!(body.len() != 1 && body.len() != 2);

        if body.len() > 2 {
            self.snakes[self.num_snakes as usize].head = body[0];
            self.snakes[self.num_snakes as usize].old_head = Default::default();
            self.snakes[self.num_snakes as usize].tail = body[body.len() - 1];

            for i in (1..body.len()).rev() {
                if body[i] == body[i - 1] {
                    self.snakes[self.num_snakes as usize].num_stacked += 1;
                } else {
                    break;
                }
            }
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

        let mut next_coord = Some(api_snake.body[api_snake.body.len() - 2]);

        for (i, coord) in api_snake.body.iter().rev().copied().enumerate() {
            if next_coord.is_some()
                && coord != next_coord.unwrap()
                && !self.next_to(coord, next_coord.unwrap(), game.ruleset)
            {
                return Err(Error::BadBoard("Snake was not contiguous".to_owned()));
            }

            let mv = match next_coord {
                Some(next) => self.coord_to_move(coord, next, game.ruleset),
                None => None,
            };

            match (self.at(coord), i) {
                (BoardSquare::Empty, 0) => {
                    if self.snakes[snake_idx as usize].head == self.snakes[snake_idx as usize].tail {
                        // All-heads case
                        self.set_at(coord, BoardSquare::SnakeHead(snake_idx));
                    } else if self.snakes[snake_idx as usize].num_stacked > 0 {
                        // First block of stacked tail case
                        self.set_at(coord, BoardSquare::SnakeTail(snake_idx, Move::Left));
                    } else {
                        self.set_at(coord, BoardSquare::SnakeTail(snake_idx, mv.unwrap()));
                    }
                }
                (BoardSquare::Hazard, 0) => {
                    if self.snakes[snake_idx as usize].head == self.snakes[snake_idx as usize].tail {
                        self.set_at(coord, BoardSquare::SnakeHeadHazard(snake_idx));
                    } else if self.snakes[snake_idx as usize].num_stacked > 0 {
                        self.set_at(coord, BoardSquare::SnakeTailHazard(snake_idx, Move::Left));
                    } else {
                        self.set_at(coord, BoardSquare::SnakeTailHazard(snake_idx, mv.unwrap()));
                    }
                }
                (BoardSquare::Empty, x) => {
                    if x < api_snake.body.len() - 1 {
                        self.set_at(coord, BoardSquare::SnakeBody(snake_idx, mv.unwrap()));
                    } else {
                        self.set_at(coord, BoardSquare::SnakeHead(snake_idx));
                    }
                }
                (BoardSquare::Hazard, x) => {
                    if x < api_snake.body.len() - 1 {
                        self.set_at(coord, BoardSquare::SnakeBodyHazard(snake_idx, mv.unwrap()));
                    } else {
                        self.set_at(coord, BoardSquare::SnakeHeadHazard(snake_idx));
                    }
                }
                (BoardSquare::SnakeTail(idx, _), _) => {
                    if idx != snake_idx {
                        return Err(Error::BadBoard(
                            "Snake square conflicts with other SnakeTail".to_owned(),
                        ));
                    }
                    // last block of stacked tail case
                    if let Some(next_mv) = mv {
                        self.set_at(coord, BoardSquare::SnakeTail(snake_idx, next_mv));
                    }
                }
                (BoardSquare::SnakeTailHazard(idx, _), _) => {
                    if idx != snake_idx {
                        return Err(Error::BadBoard(
                            "Snake square conflicts with other SnakeTailHazard".to_owned(),
                        ));
                    }
                    if let Some(next_mv) = mv {
                        self.set_at(coord, BoardSquare::SnakeTailHazard(snake_idx, next_mv));
                    }
                }
                (BoardSquare::SnakeHead(idx), _) | (BoardSquare::SnakeHeadHazard(idx), _) => {
                    if idx != snake_idx {
                        return Err(Error::BadBoard(
                            "Snake square conflicts with other SnakeHead/SnakeTail".to_owned(),
                        ));
                    }
                }
                (BoardSquare::Food, _) | (BoardSquare::FoodHazard, _) => {
                    return Err(Error::BadBoard("Snake square conflicts with Food".to_owned()))
                }
                (BoardSquare::SnakeBody(_, _), _) | (BoardSquare::SnakeBodyHazard(_, _), _) => {
                    return Err(Error::BadBoard(
                        "Snake square conflicts with other SnakeBody".to_owned(),
                    ));
                }
            }

            if i < api_snake.body.len() - 2 {
                next_coord = Some(api_snake.body[api_snake.body.len() - i - 3]);
            } else {
                next_coord = None;
            }
        }
        Ok(())
    }

    pub fn snake_head(&self, snake_idx: usize) -> Coord {
        self.snakes[snake_idx].head
    }

    pub fn snake_tail(&self, snake_idx: usize) -> Coord {
        self.snakes[snake_idx].tail
    }

    pub fn snake_len(&self, snake_idx: usize) -> i32 {
        self.snakes[snake_idx].len
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

    pub fn coord_to_move(&self, orig: Coord, dest: Coord, rules: Rules) -> Option<Move> {
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

        assert!(diff_x == 0 || diff_y == 0);

        if diff_y == 0 {
            match diff_x.cmp(&0) {
                Ordering::Greater => Some(Move::Right),
                Ordering::Less => Some(Move::Left),
                Ordering::Equal => None,
            }
        } else {
            match diff_y.cmp(&0) {
                Ordering::Greater => Some(Move::Up),
                Ordering::Less => Some(Move::Down),
                Ordering::Equal => None,
            }
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
