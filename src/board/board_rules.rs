use super::*;

use crate::game::{Game, Map};
use crate::rand::Rand;

use std::cmp::max;

impl Board {
    // generate a random valid move for a snake
    pub fn gen_move(&self, game: &Game, snake_idx: usize, rng: &mut impl Rand) -> Move {
        let mut valid_moves = [Move::Left; 4];
        let mut num_valid = 0;

        for mv_idx in 0..4 {
            let mv = Move::from_idx(mv_idx);
            let is_valid = self.valid_move(game, snake_idx, mv);
            if is_valid {
                valid_moves[num_valid as usize] = mv;
                num_valid += 1;
            }
        }

        if num_valid > 0 {
            let mv_idx = rng.range(0, num_valid - 1);
            valid_moves[mv_idx as usize]
        } else {
            Move::Left
        }
    }

    pub fn gen_strong_move(&self, game: &Game, snake_idx: usize, rng: &mut impl Rand) -> Move {
        let mut best_move = None;

        let mut valid_moves = [Move::Left; 4];

        let mut neutral_moves = [Move::Left; 4];
        let mut good_moves = [Move::Left; 4];
        let mut bad_moves = [Move::Left; 4];

        let mut num_valid = 0;

        let mut num_neutral = 0;
        let mut num_good = 0;
        let mut num_bad = 0;

        for mv_idx in 0..4 {
            let mv = Move::from_idx(mv_idx);
            let is_valid = self.valid_move(game, snake_idx, mv);
            if is_valid {
                valid_moves[num_valid as usize] = mv;
                num_valid += 1;
                match self.head_on_col(game, snake_idx, mv) {
                    HeadOnCol::PossibleElimination => {
                        good_moves[num_good as usize] = mv;
                        num_good += 1;
                    }
                    HeadOnCol::None => {
                        neutral_moves[num_neutral as usize] = mv;
                        num_neutral += 1;
                    }
                    HeadOnCol::PossibleCollision => {
                        bad_moves[num_bad as usize] = mv;
                        num_bad += 1;
                    }
                }
            }
        }

        if num_good == 1 {
            best_move = Some(good_moves[0])
        } else if num_good > 1 {
            let mv_idx = rng.range(0, num_good - 1);
            best_move = Some(good_moves[mv_idx as usize]);
        } else if num_neutral == 1 {
            best_move = Some(neutral_moves[0]);
        } else if num_neutral > 1 {
            let mv_idx = rng.range(0, num_neutral - 1);
            best_move = Some(neutral_moves[mv_idx as usize]);
        } else if num_bad == 1 {
            best_move = Some(bad_moves[0]);
        } else if num_bad > 1 {
            let mv_idx = rng.range(0, num_bad - 1);
            best_move = Some(bad_moves[mv_idx as usize]);
        }

        best_move.unwrap_or(Move::Left)
    }

    pub fn valid_move(&self, game: &Game, snake_idx: usize, mv: Move) -> bool {
        let head = self.snake_head(snake_idx);
        let square = self.move_to_coord(head, mv, game.ruleset);
        if !self.on_board(square) {
            return false;
        }
        match self.at(square) {
            BoardSquare::Empty | BoardSquare::Food | BoardSquare::FoodHazard => true,
            BoardSquare::Hazard => {
                (self.snakes[snake_idx].health - game.api.ruleset.settings.hazard_damage_per_turn) > 0
            }
            BoardSquare::SnakeHead(_)
            | BoardSquare::SnakeHeadHazard(_)
            | BoardSquare::SnakeBody(_)
            | BoardSquare::SnakeBodyHazard(_) => false,
            BoardSquare::SnakeTail(i) => {
                let idx = i as usize;
                self.snake_tail(idx) != self.snakes[idx].at_tail_offset(-1)
            }
            BoardSquare::SnakeTailHazard(i) => {
                let idx = i as usize;

                (self.snakes[snake_idx].health - game.api.ruleset.settings.hazard_damage_per_turn) > 0
                    && self.snake_tail(idx) != self.snakes[idx].at_tail_offset(-1)
            }
        }
    }

    pub fn is_trapped(&self, game: &Game, snake_idx: usize) -> bool {
        !self.valid_move(game, snake_idx, Move::Left)
            && !self.valid_move(game, snake_idx, Move::Right)
            && !self.valid_move(game, snake_idx, Move::Up)
            && !self.valid_move(game, snake_idx, Move::Down)
    }

    // Assumption: move is valid
    pub fn head_on_col(&self, game: &Game, snake_idx: usize, mv: Move) -> HeadOnCol {
        let snake_head = self.snake_head(snake_idx);
        let dest_square = self.move_to_coord(snake_head, mv, game.ruleset);
        for mv_idx in 0..4 {
            let adj_coord = self.move_to_coord(dest_square, Move::from_idx(mv_idx), game.ruleset);

            if !self.on_board(adj_coord) {
                continue;
            }

            match self.at(adj_coord) {
                BoardSquare::SnakeHead(idx) | BoardSquare::SnakeHeadHazard(idx) => {
                    if idx as usize != snake_idx {
                        let len_us = self.snake_len(snake_idx);
                        let len_other = self.snake_len(idx as usize);
                        if len_us > len_other {
                            return HeadOnCol::PossibleElimination;
                        } else {
                            return HeadOnCol::PossibleCollision;
                        }
                    }
                }
                _ => (),
            };
        }
        HeadOnCol::None
    }

    pub fn closest_snake(&self, game: &Game, snake_idx: usize) -> Option<Coord> {
        let mut min_abs_dist = i32::MAX;
        let mut closest_snake = None;

        let snake_head = self.snake_head(snake_idx);

        for s_idx in 0..self.num_snakes() as usize {
            if s_idx == snake_idx {
                continue;
            }

            let other_head = self.snake_head(s_idx);
            let (dist_x, dist_y) = self.abs_dist(snake_head, other_head, game.ruleset);
            let abs_dist = dist_x + dist_y;

            if abs_dist < min_abs_dist {
                min_abs_dist = abs_dist;
                closest_snake = Some(other_head);
            }
        }

        closest_snake
    }

    // Battlesnake rules implementation
    //

    pub fn gen_board(&mut self, moves: u32, game: &Game, food_buff: &mut [Coord], rng: &mut impl Rand) {
        // Note: this is not done till later in rules
        self.turn += 1;

        // Rules pipeline stages:

        // StageGameOver
        assert!(!game.over(self));

        // StageMovementStandard
        self.move_snakes(moves, game);

        // StageStarvationStandard
        // StageHazardDamageStandard
        // StageFeedSnakesStandard
        // StageEliminationStandard (partial)
        //  - out of bounds
        //  - out of health
        self.update_health(game);

        // StageEliminationStandard (partial)
        //  - collision
        self.eliminate_snakes(moves, game);

        // Additional pipeline stages
        // StageModifySnakesAlwaysGrow (constrictor)
        self.post_process(game);

        // Apply map logic
        // Does not correspond to a rules stage
        self.update_board(game, food_buff, rng);
    }

    #[inline(always)]
    fn move_snakes(&mut self, moves: u32, game: &Game) {
        // StageMovementStandard
        // Move snakes board_mat tails only, Compute location of move
        for idx in 0..(self.num_snakes() as usize) {
            let snake = &self.snakes[idx];
            if !snake.alive() {
                continue;
            }
            let mv = Move::extract(moves, idx as u32);

            let old_head = self.snake_head(idx);
            let new_head = self.move_to_coord(old_head, mv, game.ruleset);

            let old_tail = self.snakes[idx].pop_back();
            let new_tail = self.snake_tail(idx);

            self.snakes[idx].push_front(new_head);

            match (self.at(old_tail), self.at(new_tail)) {
                (BoardSquare::SnakeTail(old_idx), BoardSquare::SnakeTail(new_idx))
                | (BoardSquare::SnakeTailHazard(old_idx), BoardSquare::SnakeTailHazard(new_idx)) => {
                    debug_assert_eq!(old_idx, idx as u8);
                    debug_assert_eq!(new_idx, idx as u8);
                }
                (BoardSquare::SnakeTail(old_idx), BoardSquare::SnakeBody(new_idx)) => {
                    debug_assert_eq!(old_idx, idx as u8);
                    debug_assert_eq!(new_idx, idx as u8);

                    self.set_at(old_tail, BoardSquare::Empty);
                    self.set_at(new_tail, BoardSquare::SnakeTail(new_idx));
                }
                (BoardSquare::SnakeTail(old_idx), BoardSquare::SnakeBodyHazard(new_idx)) => {
                    debug_assert_eq!(old_idx, idx as u8);
                    debug_assert_eq!(new_idx, idx as u8);

                    self.set_at(old_tail, BoardSquare::Empty);
                    self.set_at(new_tail, BoardSquare::SnakeTailHazard(new_idx));
                }
                (BoardSquare::SnakeTailHazard(old_idx), BoardSquare::SnakeBody(new_idx)) => {
                    debug_assert_eq!(old_idx, idx as u8);
                    debug_assert_eq!(new_idx, idx as u8);

                    self.set_at(old_tail, BoardSquare::Hazard);
                    self.set_at(new_tail, BoardSquare::SnakeTail(new_idx));
                }
                (BoardSquare::SnakeTailHazard(old_idx), BoardSquare::SnakeBodyHazard(new_idx)) => {
                    debug_assert_eq!(old_idx, idx as u8);
                    debug_assert_eq!(new_idx, idx as u8);

                    self.set_at(old_tail, BoardSquare::Hazard);
                    self.set_at(new_tail, BoardSquare::SnakeTailHazard(new_idx));
                }
                // Special cases: Snake was head only. This should only happen on first move
                (BoardSquare::SnakeHead(old_idx), _) => {
                    debug_assert_eq!(old_idx, idx as u8);

                    self.set_at(old_tail, BoardSquare::SnakeTail(old_idx));
                }
                (BoardSquare::SnakeHeadHazard(old_idx), _) => {
                    debug_assert_eq!(old_idx, idx as u8);
                    self.set_at(old_tail, BoardSquare::SnakeTailHazard(old_idx));
                }
                (old_val, new_val) => panic!(
                    "LOGIC ERROR: snake {idx} tails set to invalid BoardSquare:\nold: {old_val:?}, new: {new_val:?}\n{self}"
                ),
            }
        }
    }

    #[inline(always)]
    fn update_health(&mut self, game: &Game) {
        for idx in 0..(self.num_snakes() as usize) {
            if !self.snakes[idx].alive() {
                continue;
            }

            let dest = self.snake_head(idx);

            // StageEliminationStandard: out-of-bounds
            // This early exit is valid because out-of-bounds snakes are eliminated first
            if !self.on_board(dest) {
                self.snakes[idx].health = 0;
                self.snakes[idx].eliminated = true;
                continue;
            }

            // StageStarvationStandard
            self.snakes[idx].health -= 1;

            // StageHazardDamageStandard
            // StageFeedSnakesStandard
            //  - Except for removal of food coords, which are only tracked on board
            let dest_square = self.at(dest);
            match dest_square {
                BoardSquare::Food | BoardSquare::FoodHazard => {
                    let tail = self.snake_tail(idx);

                    self.snakes[idx].health = 100;
                    self.snakes[idx].push_back(tail);
                }
                BoardSquare::Hazard
                | BoardSquare::SnakeHeadHazard(_)
                | BoardSquare::SnakeBodyHazard(_)
                | BoardSquare::SnakeTailHazard(_) => {
                    let damage = game.api.ruleset.settings.hazard_damage_per_turn;
                    self.snakes[idx].health = max(self.snakes[idx].health - damage, 0);
                }
                _ => (),
            }

            // StageEliminationStandard: out-of-health
            if self.snakes[idx].health == 0 {
                self.snakes[idx].eliminated = true;
                continue;
            }
        }
    }

    // StageEliminationStandard
    // Move Head, Track Collisions
    #[inline(always)]
    fn eliminate_snakes(&mut self, moves: u32, game: &Game) {
        for idx in 0..(self.num_snakes() as usize) {
            if !self.snakes[idx].alive() {
                continue;
            }

            let idx_byte = idx as u8;
            let mv = Move::extract(moves, idx as u32);

            // Update old head, even for eliminated snakes
            let old_head = self.snakes[idx].at_head_offset(1);
            if old_head != self.snake_tail(idx) {
                match self.at(old_head) {
                    BoardSquare::SnakeHead(snake_idx) => {
                        debug_assert_eq!(snake_idx, idx_byte);

                        self.set_at(old_head, BoardSquare::SnakeBody(snake_idx));
                    }
                    BoardSquare::SnakeHeadHazard(snake_idx) => {
                        debug_assert_eq!(snake_idx, idx_byte);

                        self.set_at(old_head, BoardSquare::SnakeBodyHazard(snake_idx));
                    }
                    _ => panic!("LOGIC ERROR: snake {idx} head set to invalid BoardSquare:\n{self}"),
                }
            }

            let new_head = self.move_to_coord(old_head, mv, game.ruleset);

            // Track collisions by only setting new head if snake is alive
            match self.at(new_head) {
                BoardSquare::Empty => {
                    self.set_at(new_head, BoardSquare::SnakeHead(idx_byte));
                }
                BoardSquare::Food => {
                    self.set_at(new_head, BoardSquare::SnakeHead(idx_byte));
                    self.num_food -= 1;
                }
                BoardSquare::Hazard => {
                    self.set_at(new_head, BoardSquare::SnakeHeadHazard(idx_byte));
                }
                BoardSquare::FoodHazard => {
                    self.set_at(new_head, BoardSquare::SnakeHeadHazard(idx_byte));
                    self.num_food -= 1;
                }
                BoardSquare::SnakeHead(s) => {
                    if self.snakes[s as usize].eliminated
                        || (s < idx_byte && self.snake_len(idx) > self.snake_len(s as usize))
                    {
                        self.set_at(new_head, BoardSquare::SnakeHead(idx_byte));
                    // Edge case: Equal length means we need to indicate the other snake is dead too
                    } else if s < idx_byte && self.snake_len(idx) == self.snake_len(s as usize) {
                        self.set_at(new_head, BoardSquare::Empty);
                    }
                }
                BoardSquare::SnakeHeadHazard(s) => {
                    if self.snakes[s as usize].eliminated
                        || (s < idx_byte && self.snake_len(idx) > self.snake_len(s as usize))
                    {
                        self.set_at(new_head, BoardSquare::SnakeHeadHazard(idx_byte));
                    // Edge case: Equal length means we need to indicate the other snake is dead too
                    } else if s < idx_byte && self.snake_len(idx) == self.snake_len(s as usize) {
                        self.set_at(new_head, BoardSquare::Empty);
                    }
                }
                BoardSquare::SnakeTail(s) => {
                    if self.snakes[s as usize].eliminated {
                        self.set_at(new_head, BoardSquare::SnakeHead(idx_byte));
                    }
                }
                BoardSquare::SnakeTailHazard(s) => {
                    if self.snakes[s as usize].eliminated {
                        self.set_at(new_head, BoardSquare::SnakeHeadHazard(idx_byte));
                    }
                }
                BoardSquare::SnakeBody(s) => {
                    if self.snakes[s as usize].eliminated {
                        self.set_at(new_head, BoardSquare::SnakeHead(idx_byte));
                    }
                }
                BoardSquare::SnakeBodyHazard(s) => {
                    if self.snakes[s as usize].eliminated {
                        self.set_at(new_head, BoardSquare::SnakeHeadHazard(idx_byte));
                    }
                }
            }
        }

        // Remove eliminated snakes
        for idx in 0..self.num_snakes() as usize {
            if !self.snakes[idx].alive() && !self.snakes[idx].eliminated {
                continue;
            }

            if !self.snakes[idx].eliminated {
                let head = self.at(self.snake_head(idx));
                match head {
                    // If snake head is not set properly and snake has not yet been eliminated for other reasons
                    // then snake was eliminated by a collision
                    BoardSquare::SnakeHead(s) | BoardSquare::SnakeHeadHazard(s) => {
                        if s as usize != idx {
                            self.snakes[idx].health = 0;
                            self.snakes[idx].eliminated = true;
                        }
                    }
                    _ => {
                        self.snakes[idx].health = 0;
                        self.snakes[idx].eliminated = true;
                    }
                }
            }

            if !self.snakes[idx].eliminated {
                continue;
            }

            for body_idx in 0..self.snakes[idx].len {
                let coord = self.snakes[idx].at_head_offset(body_idx);

                if !self.on_board(coord) {
                    continue;
                }

                match self.at(coord) {
                    BoardSquare::SnakeHead(s) | BoardSquare::SnakeBody(s) | BoardSquare::SnakeTail(s) => {
                        if s as usize == idx {
                            self.set_at(coord, BoardSquare::Empty)
                        }
                    }
                    BoardSquare::SnakeHeadHazard(s)
                    | BoardSquare::SnakeBodyHazard(s)
                    | BoardSquare::SnakeTailHazard(s) => {
                        if s as usize == idx {
                            self.set_at(coord, BoardSquare::Hazard)
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    #[inline(always)]
    fn update_board(&mut self, game: &Game, food_buff: &mut [Coord], rng: &mut impl Rand) {
        let map = game.api.map;
        let rules = game.ruleset;

        if let Map::Empty = map {
            return;
        }

        let min_food = game.api.ruleset.settings.minimum_food;
        let chance = game.api.ruleset.settings.food_spawn_chance;

        let mut num_spawn = 0;

        let rand_val = 100 - rng.int_n(100);
        if self.num_food() < min_food {
            num_spawn = (min_food - self.num_food()) as usize;
        } else if rand_val < chance {
            num_spawn = 1;
        }

        let mut num_unnocupied = 0;

        if num_spawn > 0 {
            for x in 0..self.width {
                'coord_loop: for y in 0..self.height {
                    let coord = Coord::new(x as i8, y as i8);

                    let square = self.at(coord);

                    if square != BoardSquare::Empty && square != BoardSquare::Hazard {
                        continue;
                    };

                    // Potential Bug: GetUnoccupiedPoints excludes squares that may be moved to
                    for idx in 0..self.num_snakes() as usize {
                        if !self.snakes[idx].alive() {
                            continue;
                        }

                        // Bug: maps don't consider possibility of wrapping.
                        // Use standard ruleset here to match this behavior
                        if self.next_to(self.snake_head(idx), coord, Rules::Standard) {
                            continue 'coord_loop;
                        }
                    }

                    food_buff[num_unnocupied] = coord;
                    num_unnocupied += 1;
                }
            }

            if num_unnocupied != 0 {
                num_spawn = min(num_spawn, num_unnocupied);
                rng.shuffle(&mut food_buff[0..num_unnocupied], num_spawn);

                self.num_food += num_spawn as i32;

                for coord in food_buff.iter().take(num_spawn) {
                    match self.at(*coord) {
                        BoardSquare::Empty => self.set_at(*coord, BoardSquare::Food),
                        BoardSquare::Hazard => self.set_at(*coord, BoardSquare::FoodHazard),
                        _ => panic!("Invalid square in food_buff"),
                    }
                }
            }
        }

        // royale logic, possibly shrink board
        match (map, rules) {
            (Map::Royale, _) | (_, Rules::Royale) => (),
            _ => return,
        }

        let shrink_every = game.api.ruleset.settings.royale.shrink_every_n_turns;

        let do_shrink = (self.turn % shrink_every) == 0;

        if !do_shrink || self.royale_min_x > self.royale_max_x || self.royale_min_y > self.royale_max_y {
            return;
        }

        let (dim, index) = match rng.int_n(4) {
            0 => {
                let index = self.royale_min_x;
                self.royale_min_x += 1;
                (self.height, index)
            }
            1 => {
                let index = self.royale_max_x;
                self.royale_max_x -= 1;
                (self.height, index)
            }
            2 => {
                let index = self.royale_min_y;
                self.royale_min_y += 1;
                (self.width, index)
            }
            3 => {
                let index = self.royale_max_y;
                self.royale_max_y -= 1;
                (self.width, index)
            }
            _ => panic!("Invalid random number"),
        };

        for z in 0..dim {
            let curr_coord = if dim == self.width {
                Coord::new(z as i8, index as i8)
            } else {
                Coord::new(index as i8, z as i8)
            };

            let curr_val = self.at(curr_coord);
            let set_val = match curr_val {
                BoardSquare::Empty => BoardSquare::Hazard,
                BoardSquare::Food => BoardSquare::FoodHazard,
                BoardSquare::SnakeHead(x) => BoardSquare::SnakeHeadHazard(x),
                BoardSquare::SnakeBody(x) => BoardSquare::SnakeBodyHazard(x),
                BoardSquare::SnakeTail(x) => BoardSquare::SnakeTailHazard(x),
                _ => curr_val,
            };

            self.set_at(curr_coord, set_val)
        }
    }

    #[inline(always)]
    fn post_process(&mut self, game: &Game) {
        match game.ruleset {
            Rules::Constrictor => (),
            _ => return,
        }

        for s_idx in 0..self.num_snakes() as usize {
            if self.snakes[s_idx].alive() {
                self.snakes[s_idx].health = 100;
            }

            let tail = self.snake_tail(s_idx);
            let sub_tail = self.snakes[s_idx].at_tail_offset(-1);

            if tail != sub_tail {
                self.snakes[s_idx].push_back(tail);
            }
        }
    }
}
