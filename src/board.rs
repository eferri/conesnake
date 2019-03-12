use crate::api::{BattleState, BoardApi, SnakeApi};
use crate::game::{Game, Map, Rules, ARCADE_FOOD_COORDS};
use crate::util;
use crate::util::{Coord, Error, Move};

use std::fmt;
use std::{cmp::min, fmt::Write, str};

use rand::{seq::SliceRandom, Rng};

#[derive(Clone, PartialEq, Eq)]
pub struct Board {
    pub width: i32,
    pub height: i32,
    pub max_width: i32,
    pub max_height: i32,
    pub turn: i64,
    pub snakes: Vec<Snake>,
    board_mat: Vec<BoardSquare>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Snake {
    pub head: Coord,
    pub tail: Coord,
    pub len: i32,
    pub health: i32,
    pub alive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardSquare {
    Empty,
    SnakeHead(u8, i8),       // index of snake, number of stacked segments
    SnakeBody(u8, Move),     // index of snake, move to next body square
    SnakeTail(u8, Move, i8), // index of snake, move to next body square, number of stacked segments
    Food,
    Hazard,
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
            snakes: Vec::with_capacity(max_snakes as usize),
            board_mat: vec![BoardSquare::Empty; (max_width * max_height) as usize],
        }
    }

    pub fn from_req(req: BattleState, max_width: i32, max_height: i32, max_snakes: i32) -> Result<Board, Error> {
        if req.board.snakes.is_empty() {
            return Err(Error::BadBoardReq("No snakes in request".to_owned()));
        }

        let mut board = Board::new(req.board.width, req.board.height, max_width, max_height, max_snakes);
        for coord in req.board.food.into_iter() {
            board.set_at(coord, BoardSquare::Food);
        }
        for coord in req.board.hazards.into_iter() {
            board.set_at(coord, BoardSquare::Hazard);
        }

        board.turn = req.turn;
        let our_id = req.you.id.clone();
        board.add_api_snake(req.you, req.game.ruleset.name)?;

        for snake in req.board.snakes.into_iter() {
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

    pub fn num_snakes(&self) -> i32 {
        self.snakes.len() as i32
    }

    pub fn num_alive_snakes(&self) -> i32 {
        let mut alive = 0;
        for snake in self.snakes.iter() {
            if snake.alive {
                alive += 1;
            }
        }
        alive
    }

    pub fn max_snakes(&self) -> i32 {
        self.snakes.capacity() as i32
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> i32 {
        self.width * self.height
    }

    fn resize(&mut self, w: i32, h: i32, max_width: i32, max_height: i32) {
        assert!(w <= max_width);
        assert!(h <= max_height);

        let mat_size = (max_width * max_height) as usize;

        self.width = w;
        self.height = h;
        self.max_width = max_width;
        self.max_height = max_height;
        self.board_mat.resize(mat_size, BoardSquare::Empty);
    }

    pub fn add_snake(&mut self, snake: Snake) {
        assert!(self.snakes.len() < self.snakes.capacity());
        self.snakes.push(snake);
    }

    pub fn add_snakes(&mut self, new_snakes: i32, snake: Snake) {
        self.snakes.append(&mut vec![snake; new_snakes as usize]);
    }

    pub fn add_api_snake(&mut self, api_snake: SnakeApi, rules: Rules) -> Result<(), Error> {
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
            alive: api_snake.health > 0,
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

        for (i, coord) in api_snake.body.into_iter().enumerate() {
            if prev_coord.is_some() && coord != prev_coord.unwrap() {
                prev_diff_coord = prev_coord;
            }

            if let Some(diff_coord) = prev_diff_coord {
                if !self.next_to(coord, diff_coord, rules) {
                    return Err(Error::BadBoard("Snake non-contiguous".to_owned()));
                }
            }

            // Move::Left should never be used here since first turn case is handled above
            let mv_ptr = match prev_diff_coord {
                Some(diff_coord) => self.coord_to_move(coord, diff_coord, rules),
                None => Move::Left,
            };

            if i == 0 {
                self.set_at(coord, BoardSquare::SnakeHead(snake_idx, 0));
            } else if coord == prev_coord.unwrap() {
                num_stacked += 1;
                self.set_at(coord, BoardSquare::SnakeTail(snake_idx, mv_ptr, num_stacked as i8));
            } else if i as i32 == snake.len - 1 {
                self.set_at(coord, BoardSquare::SnakeTail(snake_idx, mv_ptr, 0));
            } else {
                self.set_at(coord, BoardSquare::SnakeBody(snake_idx, mv_ptr));
            }

            prev_coord = Some(coord);
        }
        Ok(())
    }

    pub fn our_head(&self) -> Coord {
        self.snakes[0].head
    }

    pub fn at(&self, loc: Coord) -> BoardSquare {
        self.board_mat[(loc.x + (self.width * loc.y)) as usize]
    }

    pub fn at_idx(&self, idx: i32) -> BoardSquare {
        self.at(self.coord_from_idx(idx))
    }

    pub fn set_at(&mut self, loc: Coord, val: BoardSquare) {
        self.board_mat[(loc.x + (self.width * loc.y)) as usize] = val;
    }

    pub fn set_at_idx(&mut self, idx: i32, val: BoardSquare) {
        self.set_at(self.coord_from_idx(idx), val)
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

    pub fn rand_valid_move(&self, start_square: Coord, rules: Rules) -> Move {
        if self.is_trapped(start_square, rules) {
            return Move::Left;
        }

        loop {
            let mv = util::rand_move();
            if self.valid_move(start_square, mv, rules) {
                return mv;
            }
        }
    }

    pub fn is_trapped(&self, snake: Coord, rules: Rules) -> bool {
        !self.valid_move(snake, Move::Left, rules)
            && !self.valid_move(snake, Move::Right, rules)
            && !self.valid_move(snake, Move::Up, rules)
            && !self.valid_move(snake, Move::Down, rules)
    }

    pub fn gen_board(&self, moves: &[Move], game: &Game, food_buff: &mut Vec<Coord>) -> Board {
        assert_eq!(moves.len(), self.num_snakes() as usize);

        let mut new_board = self.clone();
        new_board.turn += 1;

        let rules = game.api.ruleset.name;

        // ---- StageGameOver
        assert!(!game.over(&new_board));

        // Move tails, Compute location of move ----- StageMovementStandard
        for (idx, mv) in moves.iter().enumerate() {
            let snake = &new_board.snakes[idx];
            if !snake.alive {
                continue;
            }
            let mv = *mv;
            let old_tail = new_board.snakes[idx].tail;

            // Move tail first, even if eventually dead
            match new_board.at(old_tail) {
                BoardSquare::SnakeTail(this_idx, old_tail_mv, 0) => {
                    debug_assert_eq!(this_idx, idx as u8);

                    let new_tail = new_board.move_to_coord(old_tail, old_tail_mv, rules);

                    new_board.set_at(old_tail, BoardSquare::Empty);
                    new_board.snakes[idx].tail = new_tail;

                    if let BoardSquare::SnakeBody(body_idx, mv) = new_board.at(new_tail) {
                        debug_assert_eq!(body_idx, idx as u8);
                        new_board.set_at(new_tail, BoardSquare::SnakeTail(body_idx, mv, 0));
                    } else {
                        panic!(
                            "snake {} new_tail set to invalid BoardSquare: {:?}\n{}",
                            idx,
                            new_board.at(new_tail),
                            new_board
                        );
                    }
                }
                BoardSquare::SnakeTail(this_idx, old_tail_mv, n) => {
                    debug_assert_eq!(this_idx, idx as u8);
                    new_board.set_at(old_tail, BoardSquare::SnakeTail(this_idx, old_tail_mv, n - 1));
                }
                // Special case: Snake was head only. This should only happen on first move
                BoardSquare::SnakeHead(this_idx, n) => {
                    debug_assert_eq!(this_idx, idx as u8);
                    new_board.set_at(old_tail, BoardSquare::SnakeTail(this_idx, mv, n - 1));
                }
                _ => panic!(
                    "LOGIC ERROR: snake {} tail set to invalid BoardSquare:\n{}",
                    idx, new_board
                ),
            }
        }

        // Reduce Snake Health -------- StageStarvationStandard
        // Apply Hazard Damage -------- StageHazardDamageStandard
        // Feed Snakes ---------------- StageFeedSnakesStandard
        // First elimination phases: -- StageEliminationStandard
        //  - out of health or
        //  - out of bounds or
        let num_snakes = new_board.num_snakes() as usize;
        for (idx, mv) in moves.iter().enumerate() {
            if !new_board.snakes[idx as usize].alive {
                continue;
            }

            let dest = new_board.move_to_coord(new_board.snakes[idx as usize].head, *mv, rules);
            let on_board = new_board.on_board(dest);

            if !on_board {
                new_board.snakes[idx as usize].health = 0;
                new_board.snakes[idx as usize].alive = false;
                continue;
            }

            let dest_square = new_board.at(dest);
            let mut snake = &mut new_board.snakes[idx as usize];

            snake.health -= 1;
            match dest_square {
                BoardSquare::Hazard => {
                    let damage = game.api.ruleset.settings.hazard_damage_per_turn;
                    snake.health = min(snake.health - damage, 0);
                }
                BoardSquare::Food => {
                    snake.health = 100;
                    snake.len += 1;
                }

                _ => (),
            }
            if snake.health == 0 {
                snake.alive = false;
            }
            // Adjust tail if snake ate a food
            if new_board.snakes[idx].health == 100 {
                match new_board.at(new_board.snakes[idx].tail) {
                    BoardSquare::SnakeTail(snake_idx, mv, n) => {
                        new_board.set_at(new_board.snakes[idx].tail, BoardSquare::SnakeTail(snake_idx, mv, n + 1));
                    }
                    _ => panic!(
                        "LOGIC ERROR: snake {} tail not set to valid BoardSquare\n{}",
                        idx, new_board
                    ),
                }
            }
        }

        // Move Head, Track Collisions ----- StageMovementStandard/StageEliminationStandard
        for (idx, mv) in moves.iter().enumerate() {
            // Dead from previous move
            if !new_board.snakes[idx].alive {
                continue;
            }

            let snake_idx = idx as u8;
            let mv = *mv;

            // Move head
            let new_head = new_board.move_to_coord(new_board.snakes[idx].head, mv, rules);
            let new_square = BoardSquare::SnakeHead(snake_idx, 0);

            // Update old head
            if new_board.snakes[idx].head != new_board.snakes[idx].tail {
                new_board.set_at(new_board.snakes[idx].head, BoardSquare::SnakeBody(snake_idx, mv));
            }
            new_board.snakes[idx].head = new_head;

            // Track collisions by only setting head if snake is alive
            match new_board.at(new_head) {
                BoardSquare::Empty | BoardSquare::Food => {
                    new_board.set_at(new_head, new_square);
                }
                BoardSquare::SnakeHead(s, _) => {
                    if !new_board.snakes[s as usize].alive
                        || (s < snake_idx && new_board.snakes[idx].len > new_board.snakes[s as usize].len)
                    {
                        new_board.set_at(new_head, new_square);
                    // Edge case: Equal length means we need to indicate the other snake is dead too
                    } else if s < snake_idx && new_board.snakes[idx].len == new_board.snakes[s as usize].len {
                        new_board.set_at(new_head, BoardSquare::Empty);
                    }
                }
                BoardSquare::SnakeTail(s, _, _) => {
                    if !new_board.snakes[s as usize].alive {
                        new_board.set_at(new_head, new_square);
                    }
                }
                BoardSquare::SnakeBody(s, _) => {
                    if !new_board.snakes[s as usize].alive {
                        new_board.set_at(new_head, new_square);
                    }
                }
                _ => (),
            }
        }

        // Last elimination phases: -- StageEliminationStandard
        //  - collide with body
        //  - collide head-to-head
        for idx in 0..num_snakes {
            if !new_board.snakes[idx as usize].alive {
                continue;
            }

            let dest = new_board.at(new_board.snakes[idx].head);

            match dest {
                // If our head is not set properly, we were eliminated
                BoardSquare::SnakeHead(s, _) => {
                    if s as usize != idx {
                        new_board.snakes[idx as usize].health = 0;
                    }
                }
                _ => {
                    new_board.snakes[idx as usize].health = 0;
                }
            }
            if new_board.snakes[idx as usize].health == 0 {
                new_board.snakes[idx as usize].alive = false;
            }
        }

        new_board.spawn_food(
            game.api.map,
            game.api.ruleset.settings.food_spawn_chance,
            game.api.ruleset.settings.minimum_food,
            food_buff,
        );

        // Remove dead snakes from board
        for snake in &mut new_board.snakes {
            if snake.health == 0 || !snake.alive {
                *snake = Default::default();
            }
        }

        for i in 0..new_board.len() {
            let square = new_board.at_idx(i);
            match square {
                BoardSquare::SnakeHead(idx, _) | BoardSquare::SnakeBody(idx, _) | BoardSquare::SnakeTail(idx, _, _) => {
                    if !new_board.snakes[idx as usize].alive {
                        new_board.set_at_idx(i, BoardSquare::Empty);
                    }
                }
                _ => (),
            }
        }

        new_board
    }

    pub fn num_food(&self) -> i32 {
        let mut food_count = 0;
        for i in 0..self.len() {
            let square = self.at_idx(i);
            if square == BoardSquare::Food {
                food_count += 1
            };
        }
        food_count
    }

    fn spawn_food(&mut self, map: Map, chance: i32, mut min_food: i32, food_buff: &mut Vec<Coord>) {
        if let Map::Empty = map {
            return;
        } else if let Map::ArcadeMaze = map {
            min_food = 0;
        }

        let num_food = self.num_food();

        let mut rng = rand::thread_rng();
        let x = rng.gen_range(0..100);

        let mut num_spawn = if num_food < min_food {
            min_food - num_food
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
            panic!("Could not find snake given tail {:?}", tail);
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
        // Remove whitespace lines
        let lines: Vec<&str> = inp.lines().filter(|l| l.split_whitespace().next().is_some()).collect();

        let header: Vec<&str> = lines[0].split_whitespace().collect();

        assert_eq!(header.len() % 2, 0);
        assert!(header.len() >= 4);

        let num_snakes = (header.len() - 2) / 2;
        let mut board = Board::new(0, 0, 0, 0, num_snakes as i32);

        assert_eq!(header[0], "turn:", "Invalid board str header: turn field");
        board.turn = header[1].parse::<i64>().unwrap();

        for (i, h) in header.iter().skip(2).enumerate() {
            match i % 2 {
                0 => {
                    assert_eq!(*h, "health:", "Invalid board str header: health field");
                    board.snakes.push(Default::default());
                }
                1 => {
                    let health = h.parse::<i32>().unwrap();
                    board.snakes[i / 2].health = health;
                    if health > 0 {
                        board.snakes[i / 2].alive = true;
                    }
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

        board.resize(w, h, w, h);

        // Populate board matrix, except for snake indices as they are not encoded
        for (line_idx, line) in lines_vec.iter().enumerate() {
            for (i, char) in line.iter().enumerate() {
                let board_square = util::char_to_square(*char);
                let board_coord = Coord {
                    x: i as i32 % board.width,
                    y: board.height - 1 - line_idx as i32,
                };

                board.set_at(board_coord, board_square);
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
            if board.snakes[i].alive && board.snakes[i].len == 0 {
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
