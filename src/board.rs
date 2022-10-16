use crate::api::{BattleState, BoardApi, SnakeApi};
use crate::game::{Game, Map, Rules, ARCADE_FOOD_COORDS};
use crate::util::{self, rand_move_arr};
use crate::util::{Coord, Error, Move};

use std::fmt;
use std::{cmp::max, cmp::min, fmt::Write, str};

use deepsize::DeepSizeOf;
use rand::{seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, DeepSizeOf)]
pub struct Board {
    pub width: i32,
    pub height: i32,
    pub max_width: i32,
    pub max_height: i32,
    pub turn: i32,
    pub num_food: i32,
    pub snakes: Vec<Snake>,
    board_mat: Vec<BoardSquare>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, DeepSizeOf)]
pub struct Snake {
    pub head: Coord,
    pub tail: Coord,
    pub len: i32,
    pub health: i32,
    pub eliminated: bool,
}

impl Snake {
    pub fn alive(&self) -> bool {
        self.health > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, DeepSizeOf)]
pub enum BoardSquare {
    Empty,
    SnakeHead(u8, i8),       // index of snake, number of stacked segments
    SnakeBody(u8, Move),     // index of snake, move to next body square
    SnakeTail(u8, Move, i8), // index of snake, move to next body square, number of stacked segments
    Food,
    Hazard,
}

impl fmt::Display for BoardSquare {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", util::square_to_char(*self))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MoveResult {
    pub mv: Move,
    pub loc: Coord,
}

impl MoveResult {
    pub fn new(loc: Coord, mv: Move) -> Self {
        MoveResult { mv, loc }
    }
}

impl Board {
    pub fn new(width: i32, height: i32, max_width: i32, max_height: i32, max_snakes: i32) -> Self {
        Board {
            height,
            width,
            max_width,
            max_height,
            turn: 1,
            num_food: 0,
            snakes: Vec::with_capacity(max_snakes as usize),
            board_mat: vec![BoardSquare::Empty; (max_width * max_height) as usize],
        }
    }

    pub fn from_req(req: &BattleState, max_width: i32, max_height: i32, max_snakes: i32) -> Result<Board, Error> {
        if req.board.snakes.is_empty() {
            return Err(Error::BadBoardReq("No snakes in request".to_owned()));
        }

        let mut board = Board::new(req.board.width, req.board.height, max_width, max_height, max_snakes);
        for coord in req.board.food.iter() {
            board.set_at(*coord, BoardSquare::Food);
            board.num_food += 1;
        }
        for coord in req.board.hazards.iter() {
            board.set_at(*coord, BoardSquare::Hazard);
        }

        board.turn = req.turn;
        let our_id = req.you.id.clone();
        board.add_api_snake(&req.you, req.game.ruleset.name)?;

        for snake in req.board.snakes.iter() {
            if our_id == snake.id {
                continue;
            }

            board.add_api_snake(snake, req.game.ruleset.name)?;
        }
        Ok(board)
    }

    pub fn to_req(&self, game: &Game) -> Result<BattleState, Error> {
        let mut food = Vec::new();
        let mut hazards = Vec::new();
        let mut snakes = Vec::new();

        for i in 0..self.len() {
            let coord = self.coord_from_idx(i as i32);
            match self.at(coord) {
                BoardSquare::Food => food.push(coord),
                BoardSquare::Hazard => hazards.push(coord),
                _ => (),
            }
        }

        if self.snakes.is_empty() {
            return Err(Error::BadBoard("No snakes".to_owned()));
        }

        for (i, snake) in self.snakes.iter().enumerate() {
            let mut api_snake = SnakeApi {
                id: i.to_string(),
                name: i.to_string(),
                body: Vec::new(),
                head: snake.head,
                health: snake.health,
                latency: "0".to_owned(),
                length: snake.len,
                shout: None,
                squad: "".to_owned(),
                customizations: Default::default(),
            };

            let mut curr_coord = snake.tail;
            loop {
                let mv = match self.at(curr_coord) {
                    BoardSquare::SnakeHead(_, n) => {
                        for _ in 0..(n + 1) {
                            api_snake.body.push(curr_coord);
                        }
                        break;
                    }
                    BoardSquare::SnakeBody(_, mv) => {
                        api_snake.body.push(curr_coord);
                        mv
                    }
                    BoardSquare::SnakeTail(_, mv, n) => {
                        for _ in 0..(n + 1) {
                            api_snake.body.push(curr_coord);
                        }
                        mv
                    }
                    _ => return Err(Error::BadBoard("Snake was not contiguous".to_owned())),
                };
                curr_coord = self.move_to_coord(curr_coord, mv, game.api.ruleset.name);
            }
            api_snake.body.reverse();
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

        let board_len = (self.width * self.height) as usize;

        self.snakes.clear();
        for s in &other.snakes {
            self.snakes.push(*s);
        }
        self.board_mat[..board_len].copy_from_slice(&other.board_mat[..board_len]);
    }

    pub fn num_snakes(&self) -> i32 {
        self.snakes.len() as i32
    }

    pub fn num_alive_snakes(&self) -> i32 {
        let mut alive = 0;
        for snake in self.snakes.iter() {
            if snake.alive() {
                alive += 1;
            }
        }
        alive
    }

    pub fn max_snakes(&self) -> i32 {
        self.snakes.capacity() as i32
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

        self.max_width = max_width;
        self.max_height = max_height;
        self.board_mat.resize(mat_size, BoardSquare::Empty);
    }

    fn set_size(&mut self, w: i32, h: i32) {
        assert!(w <= self.max_width);
        assert!(h <= self.max_height);

        self.width = w;
        self.height = h;
    }

    fn add_snake(&mut self, snake: Snake) {
        assert!(self.snakes.len() < self.snakes.capacity());
        self.snakes.push(snake);
    }

    pub fn add_snakes(&mut self, new_snakes: i32, snake: Snake) {
        self.snakes.append(&mut vec![snake; new_snakes as usize]);
    }

    pub fn add_api_snake(&mut self, api_snake: &SnakeApi, rules: Rules) -> Result<(), Error> {
        let snake = Snake {
            head: Coord {
                x: api_snake.head.x,
                y: api_snake.head.y,
            },
            tail: Coord {
                x: api_snake.body.last().unwrap().x,
                y: api_snake.body.last().unwrap().y,
            },
            len: api_snake.body.len() as i32,
            health: api_snake.health,
            eliminated: false,
        };
        self.add_snake(snake);

        if api_snake.health == 0 {
            return Ok(());
        }

        let mut prev_coord = None;
        let mut prev_diff_coord = None;
        let mut num_stacked = 0;
        let snake_idx = (self.num_snakes() - 1) as u8;

        // If all coords are equal, snake is just a head
        let mut all_equal = true;
        for (i, coord) in api_snake.body[1..].iter().enumerate() {
            all_equal = all_equal && *coord == api_snake.body[i];
        }

        if all_equal {
            // Game board assumes this
            self.set_at(
                api_snake.body[0],
                BoardSquare::SnakeHead(snake_idx, api_snake.body.len() as i8),
            );
            return Ok(());
        }

        for (i, coord) in api_snake.body.iter().enumerate() {
            if prev_coord.is_some() && *coord != prev_coord.unwrap() {
                prev_diff_coord = prev_coord;
            }

            if let Some(diff_coord) = prev_diff_coord {
                if !self.next_to(*coord, diff_coord, rules) {
                    return Err(Error::BadBoard("Snake non-contiguous".to_owned()));
                }
            }

            // Move::Left should never be used here since first turn case is handled above
            let mv_ptr = match prev_diff_coord {
                Some(diff_coord) => self.coord_to_move(*coord, diff_coord, rules),
                None => Move::Left,
            };

            if i == 0 {
                self.set_at(*coord, BoardSquare::SnakeHead(snake_idx, 0));
            } else if *coord == prev_coord.unwrap() {
                num_stacked += 1;
                self.set_at(*coord, BoardSquare::SnakeTail(snake_idx, mv_ptr, num_stacked as i8));
            } else if i as i32 == snake.len - 1 {
                self.set_at(*coord, BoardSquare::SnakeTail(snake_idx, mv_ptr, 0));
            } else {
                self.set_at(*coord, BoardSquare::SnakeBody(snake_idx, mv_ptr));
            }

            prev_coord = Some(*coord);
        }
        Ok(())
    }

    pub fn our_head(&self) -> Coord {
        self.snakes[0].head
    }

    pub fn at(&self, loc: Coord) -> BoardSquare {
        self.board_mat[self.act_idx(loc)]
    }

    fn act_idx(&self, loc: Coord) -> usize {
        (loc.x + loc.y * self.width) as usize
    }

    pub fn at_idx(&self, idx: i32) -> BoardSquare {
        self.at(self.coord_from_idx(idx))
    }

    pub fn set_at(&mut self, loc: Coord, val: BoardSquare) {
        let idx = self.act_idx(loc);
        self.board_mat[idx] = val;
    }

    pub fn set_at_idx(&mut self, idx: usize, val: BoardSquare) {
        self.set_at(self.coord_from_idx(idx as i32), val)
    }

    pub fn on_board(&self, square: Coord) -> bool {
        !(square.x < 0 || square.x >= self.width || square.y < 0 || square.y >= self.height)
    }

    pub fn coord_from_idx(&self, idx: i32) -> Coord {
        Coord {
            x: idx % self.width,
            y: idx / self.width,
        }
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
            square.x = square.x.rem_euclid(self.width);
            square.y = square.y.rem_euclid(self.height);
        }

        square
    }

    pub fn coord_to_move(&self, orig: Coord, dest: Coord, rules: Rules) -> Move {
        let mut diff_x = dest.x - orig.x;
        let mut diff_y = dest.y - orig.y;

        if let Rules::Wrapped = rules {
            if diff_x == self.width - 1 {
                diff_x -= self.width;
            } else if diff_x == 1 - self.width {
                diff_x += self.width;
            }

            if diff_y == self.height - 1 {
                diff_y -= self.height;
            } else if diff_y == 1 - self.height {
                diff_y += self.height;
            }
        }

        match (diff_x, diff_y) {
            (-1, 0) => Move::Left,
            (1, 0) => Move::Right,
            (0, 1) => Move::Up,
            (0, -1) => Move::Down,
            (0, 0) => panic!("Coords are overlapping"),
            (_, _) => panic!("Coords are not next to each other"),
        }
    }

    pub fn next_to(&self, square_1: Coord, square_2: Coord, rules: Rules) -> bool {
        let mut diff_x = (square_2.x - square_1.x).abs();
        let mut diff_y = (square_2.y - square_1.y).abs();

        if let Rules::Wrapped = rules {
            diff_x = min(diff_x, self.width - diff_x);
            diff_y = min(diff_y, self.height - diff_y);
        }

        match (diff_x.abs(), diff_y.abs()) {
            (1, 0) => true,
            (0, 1) => true,
            (_, _) => false,
        }
    }

    pub fn valid_move(&self, start_square: Coord, mv: Move, rules: Rules) -> bool {
        let square = self.move_to_coord(start_square, mv, rules);
        if !self.on_board(square) {
            return false;
        }
        matches!(
            self.at(square),
            BoardSquare::Empty | BoardSquare::Food | BoardSquare::SnakeTail(_, _, 0)
        )
    }

    pub fn is_trapped(&self, snake: Coord, rules: Rules) -> bool {
        !self.valid_move(snake, Move::Left, rules)
            && !self.valid_move(snake, Move::Right, rules)
            && !self.valid_move(snake, Move::Up, rules)
            && !self.valid_move(snake, Move::Down, rules)
    }

    pub fn gen_move(&self, game: &Game, snake_idx: usize) -> Move {
        let head = self.snakes[snake_idx].head;
        let rules = game.api.ruleset.name;

        let rand_moves = rand_move_arr();
        let mut valid_moves = [false; 4];

        let mut best_move = None;

        if game.is_solo {
            for mv in rand_moves {
                if !self.valid_move(head, mv, rules) {
                    continue;
                }
                valid_moves[mv.idx()] = true;
                best_move = Some(mv)
            }

            for mv in rand_moves {
                if !valid_moves[mv.idx()] {
                    continue;
                }

                let dest = self.move_to_coord(head, mv, rules);

                if self.snakes[snake_idx].health >= 6 && self.at(dest) == BoardSquare::Food {
                    continue;
                }

                best_move = Some(mv);
                break;
            }
        } else {
            for mv in rand_moves {
                if !self.valid_move(head, mv, rules) {
                    continue;
                }
                best_move = Some(mv);
                break;
            }
        }
        best_move.unwrap_or(Move::Left)
    }

    pub fn gen_board(&mut self, moves: &[Move], game: &Game, food_buff: &mut Vec<Coord>) {
        assert_eq!(moves.len(), self.num_snakes() as usize);

        self.turn += 1;

        // ---- StageGameOver
        assert!(!game.over(self));

        self.move_tails(moves, game);

        self.apply_damage(moves, game);

        self.move_heads(moves, game);

        self.cleanup_board();

        self.spawn_food(
            game.api.map,
            game.api.ruleset.settings.food_spawn_chance,
            game.api.ruleset.settings.minimum_food,
            food_buff,
        );
    }

    fn move_tails(&mut self, moves: &[Move], game: &Game) {
        let rules = game.api.ruleset.name;

        // Move tails, Compute location of move ----- StageMovementStandard
        for (idx, mv) in moves.iter().enumerate() {
            let snake = &self.snakes[idx];
            if !snake.alive() {
                continue;
            }
            let mv = *mv;
            let old_tail = self.snakes[idx].tail;

            // Move tail first, even if eventually dead
            match self.at(old_tail) {
                BoardSquare::SnakeTail(this_idx, old_tail_mv, 0) => {
                    debug_assert_eq!(this_idx, idx as u8);

                    let new_tail = self.move_to_coord(old_tail, old_tail_mv, rules);

                    self.set_at(old_tail, BoardSquare::Empty);
                    self.snakes[idx].tail = new_tail;

                    if let BoardSquare::SnakeBody(body_idx, mv) = self.at(new_tail) {
                        debug_assert_eq!(body_idx, idx as u8);
                        self.set_at(new_tail, BoardSquare::SnakeTail(body_idx, mv, 0));
                    } else {
                        panic!(
                            "snake {} new_tail set to invalid BoardSquare: {}\n{}",
                            idx,
                            self.at(new_tail),
                            self
                        );
                    }
                }
                BoardSquare::SnakeTail(this_idx, old_tail_mv, n) => {
                    debug_assert_eq!(this_idx, idx as u8);
                    self.set_at(old_tail, BoardSquare::SnakeTail(this_idx, old_tail_mv, n - 1));
                }
                // Special case: Snake was head only. This should only happen on first move
                BoardSquare::SnakeHead(this_idx, n) => {
                    debug_assert_eq!(this_idx, idx as u8);
                    self.set_at(old_tail, BoardSquare::SnakeTail(this_idx, mv, n - 1));
                }
                _ => panic!("LOGIC ERROR: snake {} tail set to invalid BoardSquare:\n{}", idx, self),
            }
        }
    }

    // Reduce Snake Health -------- StageStarvationStandard
    // Apply Hazard Damage -------- StageHazardDamageStandard
    // Feed Snakes ---------------- StageFeedSnakesStandard
    // First elimination phases: -- StageEliminationStandard
    //  - out of health or
    //  - out of bounds or
    fn apply_damage(&mut self, moves: &[Move], game: &Game) {
        let rules = game.api.ruleset.name;

        for (idx, mv) in moves.iter().enumerate() {
            if !self.snakes[idx as usize].alive() {
                continue;
            }

            let dest = self.move_to_coord(self.snakes[idx as usize].head, *mv, rules);
            let on_board = self.on_board(dest);

            if !on_board {
                self.snakes[idx as usize].health = 0;
                self.snakes[idx as usize].eliminated = true;
                continue;
            }

            let dest_square = self.at(dest);
            let mut snake = &mut self.snakes[idx as usize];

            snake.health -= 1;
            match dest_square {
                BoardSquare::Hazard => {
                    let damage = game.api.ruleset.settings.hazard_damage_per_turn;
                    snake.health = max(snake.health - damage, 0);
                }
                BoardSquare::Food => {
                    snake.health = 100;
                    snake.len += 1;
                }

                _ => (),
            }
            if snake.health == 0 {
                snake.eliminated = true;
            }
            // Adjust tail if snake ate a food
            if self.snakes[idx].health == 100 {
                match self.at(self.snakes[idx].tail) {
                    BoardSquare::SnakeTail(snake_idx, mv, n) => {
                        self.set_at(self.snakes[idx].tail, BoardSquare::SnakeTail(snake_idx, mv, n + 1));
                    }
                    _ => panic!("LOGIC ERROR: snake {} tail not set to valid BoardSquare\n{}", idx, self),
                }
            }
        }
    }

    // Move Head, Track Collisions ----- StageMovementStandard/StageEliminationStandard
    fn move_heads(&mut self, moves: &[Move], game: &Game) {
        let rules = game.api.ruleset.name;

        for (idx, mv) in moves.iter().enumerate() {
            // Dead from previous move
            let eliminated = self.snakes[idx].eliminated;

            if !self.snakes[idx].alive() && !eliminated {
                continue;
            }

            let snake_idx = idx as u8;
            let mv = *mv;

            // Move head
            let new_head = self.move_to_coord(self.snakes[idx].head, mv, rules);
            let new_square = BoardSquare::SnakeHead(snake_idx, 0);

            let old_head = self.snakes[idx].head;
            self.snakes[idx].head = new_head;

            if eliminated {
                continue;
            }

            // Update old head
            if old_head != self.snakes[idx].tail {
                self.set_at(old_head, BoardSquare::SnakeBody(snake_idx, mv));
            }

            // Track collisions by only setting head if snake is alive
            match self.at(new_head) {
                BoardSquare::Empty => {
                    self.set_at(new_head, new_square);
                }
                BoardSquare::Food => {
                    self.set_at(new_head, new_square);
                    self.num_food -= 1;
                }
                BoardSquare::SnakeHead(s, _) => {
                    if !self.snakes[s as usize].alive()
                        || (s < snake_idx && self.snakes[idx].len > self.snakes[s as usize].len)
                    {
                        self.set_at(new_head, new_square);
                    // Edge case: Equal length means we need to indicate the other snake is dead too
                    } else if s < snake_idx && self.snakes[idx].len == self.snakes[s as usize].len {
                        self.set_at(new_head, BoardSquare::Empty);
                    }
                }
                BoardSquare::SnakeTail(s, _, _) => {
                    if !self.snakes[s as usize].alive() {
                        self.set_at(new_head, new_square);
                    }
                }
                BoardSquare::SnakeBody(s, _) => {
                    if !self.snakes[s as usize].alive() {
                        self.set_at(new_head, new_square);
                    }
                }
                _ => (),
            }
        }

        // Last elimination phases: -- StageEliminationStandard
        for idx in 0..self.num_snakes() as usize {
            if !self.snakes[idx].alive() {
                continue;
            }

            let dest = self.at(self.snakes[idx].head);

            match dest {
                // If our head is not set properly, we were eliminated
                BoardSquare::SnakeHead(s, _) => {
                    if s as usize != idx {
                        self.snakes[idx as usize].health = 0;
                    }
                }
                _ => {
                    self.snakes[idx].health = 0;
                }
            }
            if self.snakes[idx].health == 0 {
                self.snakes[idx].eliminated = true;
            }
        }
    }

    fn cleanup_board(&mut self) {
        // Remove dead snakes from board
        let board_len = self.len() as usize;
        for square in &mut self.board_mat[0..board_len] {
            match square {
                BoardSquare::SnakeHead(idx, _) | BoardSquare::SnakeBody(idx, _) | BoardSquare::SnakeTail(idx, _, _) => {
                    if self.snakes[*idx as usize].eliminated {
                        *square = BoardSquare::Empty;
                    }
                }
                _ => (),
            }
        }
    }

    fn spawn_food(&mut self, map: Map, chance: i32, mut min_food: i32, food_buff: &mut Vec<Coord>) {
        if let Map::Empty = map {
            return;
        } else if let Map::ArcadeMaze = map {
            min_food = 0;
        }

        let mut rng = rand::thread_rng();
        let x = rng.gen_range(0..100);

        #[allow(clippy::bool_to_int_with_if)]
        let mut num_spawn = if self.num_food() < min_food {
            min_food - self.num_food()
        } else if chance > 0 && x < chance {
            1
        } else {
            0
        } as usize;

        if num_spawn == 0 {
            return;
        }

        food_buff.clear();

        if let Map::ArcadeMaze = map {
            for coord in &ARCADE_FOOD_COORDS {
                if self.at(*coord) == BoardSquare::Empty {
                    food_buff.push(*coord);
                };
            }
        } else {
            for i in 0..self.len() {
                if self.at_idx(i) == BoardSquare::Empty {
                    food_buff.push(self.coord_from_idx(i));
                };
            }
        }

        if food_buff.is_empty() {
            return;
        }

        num_spawn = min(num_spawn, food_buff.len());
        food_buff.shuffle(&mut rng);

        self.num_food += num_spawn as i32;

        for coord in food_buff.iter().take(num_spawn) {
            self.set_at(*coord, BoardSquare::Food);
        }
    }

    // Returns a pair -> (index, length)
    // This function assumes snake is not just a head
    fn trace_snake(&mut self, tail: Coord, rules: Rules) -> (u8, i32) {
        let mut found = false;
        let mut snake_idx = 0;

        let mut next_mv = match self.at(tail) {
            BoardSquare::SnakeTail(_, mv, _) => mv,
            _ => panic!("Tail is not set to a valid square"),
        };

        let mut snake_len = match self.at(tail) {
            BoardSquare::SnakeTail(_, _, n) => n as i32 + 1,
            _ => panic!("Tail is not set to a valid square"),
        };

        let mut curr_coord = tail;

        while snake_len < self.height * self.width {
            curr_coord = self.move_to_coord(curr_coord, next_mv, rules);
            snake_len += 1;

            next_mv = match self.at(curr_coord) {
                BoardSquare::SnakeBody(_, mv) => mv,
                BoardSquare::SnakeHead(i, _) => {
                    snake_idx = i;
                    found = true;
                    break;
                }
                _ => panic!("Snake body was not contiguous {}", self),
            };
        }

        if !found {
            panic!("Could not find snake given tail {}", tail);
        }

        // Set index in snake squares
        next_mv = match self.at(tail) {
            BoardSquare::SnakeTail(_, mv, n) => {
                self.set_at(tail, BoardSquare::SnakeTail(snake_idx, mv, n));
                mv
            }
            _ => panic!("Tail is not set to a valid square"),
        };

        curr_coord = tail;
        loop {
            curr_coord = self.move_to_coord(curr_coord, next_mv, rules);

            next_mv = match self.at(curr_coord) {
                BoardSquare::SnakeBody(_, mv) => {
                    self.set_at(curr_coord, BoardSquare::SnakeBody(snake_idx, mv));
                    mv
                }
                BoardSquare::SnakeHead(_, _) => break,
                _ => panic!("Snake body was not contiguous {}", self),
            };
        }

        (snake_idx, snake_len)
    }

    fn to_string_internal(&self) -> String {
        let mut board_str = String::new();
        // Use `self.number` to refer to each positional data point.
        write!(&mut board_str, "turn: {} ", self.turn).unwrap();

        for s in 0..self.num_snakes() {
            write!(&mut board_str, "health: {} ", self.snakes[s as usize].health).unwrap();
        }

        writeln!(&mut board_str).unwrap();

        for y in (0..self.height).rev() {
            for x in 0..self.width {
                let square = self.at(Coord { x, y });
                let square_char = util::square_to_char(square);
                write!(&mut board_str, "{} ", square_char).unwrap();
            }
            if y != 0 {
                writeln!(&mut board_str).unwrap();
            }
        }
        board_str
    }

    pub fn from_str(inp: &str, game: &Game) -> Result<Self, Error> {
        Board::from_str_dims(inp, game, 0, 0, 0)
    }

    pub fn from_str_dims(
        inp: &str,
        game: &Game,
        max_width: i32,
        max_height: i32,
        max_snakes: i32,
    ) -> Result<Self, Error> {
        // Remove whitespace lines
        let lines: Vec<&str> = inp.lines().filter(|l| l.split_whitespace().next().is_some()).collect();

        let header: Vec<&str> = lines[0].split_whitespace().collect();

        assert_eq!(header.len() % 2, 0);
        assert!(header.len() >= 4);

        let mut board = Board::new(0, 0, max_width, max_height, max_snakes);

        assert_eq!(header[0], "turn:", "Invalid board str header: turn field");
        board.turn = header[1].parse::<i32>().unwrap();

        for (i, h) in header.iter().skip(2).enumerate() {
            match i % 2 {
                0 => {
                    assert_eq!(*h, "health:", "Invalid board str header: health field");
                    board.snakes.push(Default::default());
                }
                1 => {
                    let health = h.parse::<i32>().unwrap();
                    board.snakes[i / 2].health = health;
                }
                _ => return Err(Error::BadBoardStr("Invalid match: check mod".to_owned())),
            }
        }

        // Get dimensions of board
        let h = lines[1..].len() as i32;
        let mut lines_vec: Vec<Vec<char>> = Vec::new();

        let mut w_opt = None;

        for line in lines[1..].iter() {
            let line_vec: Vec<char> = line.chars().filter(|c| !c.is_whitespace()).collect();
            let w_line = line_vec.len();
            lines_vec.push(line_vec);

            if let Some(w_prev) = w_opt {
                if w_prev != w_line {
                    return Err(Error::BadBoardStr(
                        "Invalid board str, board width not consistent".to_owned(),
                    ));
                }
            }
            w_opt = Some(w_line);
        }

        let w = w_opt.unwrap() as i32;

        if max_width == 0 && max_height == 0 && max_snakes == 0 {
            board.resize(w, h);
        }
        board.set_size(w, h);

        // Populate board matrix, except for snake indices as they are not encoded
        for (line_idx, line) in lines_vec.iter().enumerate() {
            for (i, char) in line.iter().enumerate() {
                let board_square = util::char_to_square(*char);
                let board_coord = Coord {
                    x: i as i32 % board.width,
                    y: board.height - 1 - line_idx as i32,
                };

                board.set_at(board_coord, board_square);
                if let BoardSquare::Food = board_square {
                    board.num_food += 1;
                }
            }
        }

        let mut found_snakes = 0;

        // Populate board stats and snake indices
        for y in 0..board.height {
            for x in 0..board.width {
                let coord = Coord { x, y };
                let square = board.at(coord);
                match square {
                    BoardSquare::SnakeHead(i, _) => {
                        found_snakes += 1;
                        board.snakes[i as usize].head = coord;
                    }
                    BoardSquare::SnakeTail(_, _, _) => {
                        let (snake_idx, snake_len) = board.trace_snake(coord, game.api.ruleset.name);
                        board.snakes[snake_idx as usize].len = snake_len as i32;
                        board.snakes[snake_idx as usize].tail = coord;
                    }
                    _ => (),
                };
            }
        }

        if found_snakes < board.num_alive_snakes() {
            panic!("Board was missing snakes from header")
        }

        // Handle edge case: heads only. We haven't found the tail
        // For string boards we assume this is always a length of 3
        for i in 0..board.num_snakes() as usize {
            if board.snakes[i].alive() && board.snakes[i].len == 0 {
                board.snakes[i].tail = board.snakes[i].head;
                board.snakes[i].len = 3;
                board.set_at(board.snakes[i].head, BoardSquare::SnakeHead(i as u8, 2))
            }
        }
        Ok(board)
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.to_string_internal())?;
        Ok(())
    }
}

impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[cfg(test)]
mod board_test;

#[cfg(test)]
mod gen_board_test;
