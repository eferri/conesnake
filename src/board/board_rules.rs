use super::*;

use crate::game::{Game, Map, ARCADE_FOOD_COORDS};
use crate::rand::Rand;

use std::cmp::max;

impl Board {
    // Move heuristics applied to each move in random playout

    pub fn gen_move(&self, game: &Game, snake_idx: usize, rng: &mut impl Rand) -> Move {
        let mut valid_moves = [Move::Left; 4];
        let mut best_move = None;
        let mut num_valid = 0;

        for mv_idx in 0..4 {
            let mv = Move::from_idx(mv_idx);
            let is_valid = self.valid_move(game, snake_idx, mv);
            if is_valid {
                valid_moves[num_valid as usize] = mv;
                num_valid += 1;
            }
        }

        #[allow(clippy::comparison_chain)]
        if num_valid == 1 {
            best_move = Some(valid_moves[0]);
        } else if num_valid > 1 {
            let mv_idx = rng.range(0, num_valid - 1);
            best_move = Some(valid_moves[mv_idx as usize]);
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
                self.snake_tail(idx) != self.snakes[idx].body[self.snakes[idx].body.len() - 2]
            }
            BoardSquare::SnakeTailHazard(i) => {
                let idx = i as usize;

                (self.snakes[snake_idx].health - game.api.ruleset.settings.hazard_damage_per_turn) > 0
                    && self.snake_tail(idx) != self.snakes[idx].body[self.snakes[idx].body.len() - 2]
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

    pub fn closest_food(&self, game: &Game, snake_idx: usize) -> Option<Coord> {
        let mut min_abs_dist = i32::MAX;
        let mut closest_food = None;

        let snake_head = self.snake_head(snake_idx);

        for food in &self.food {
            let (dist_x, dist_y) = self.abs_dist(snake_head, *food, game.ruleset);
            let abs_dist = dist_x + dist_y;

            if abs_dist < min_abs_dist {
                min_abs_dist = abs_dist;
                closest_food = Some(*food);
            }
        }

        closest_food
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

    pub fn gen_board(&mut self, moves: u32, game: &Game, food_buff: &mut Vec<Coord>, rng: &mut impl Rand) {
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

            let old_tail = self.snakes[idx].body.pop_back().unwrap();
            let new_tail = self.snake_tail(idx);

            self.snakes[idx].body.push_front(new_head);
            self.snakes[idx].head = new_head;

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
                    self.snakes[idx].body.push_back(tail);
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
    fn eliminate_snakes(&mut self, moves: u32, game: &Game) {
        for idx in 0..(self.num_snakes() as usize) {
            if !self.snakes[idx].alive() {
                continue;
            }

            let idx_byte = idx as u8;
            let mv = Move::extract(moves, idx as u32);

            // Update old head, even for eliminated snakes
            let old_head = self.snakes[idx].body[1];
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
                    self.food.remove(&new_head);
                }
                BoardSquare::Hazard => {
                    self.set_at(new_head, BoardSquare::SnakeHeadHazard(idx_byte));
                }
                BoardSquare::FoodHazard => {
                    self.set_at(new_head, BoardSquare::SnakeHeadHazard(idx_byte));
                    self.num_food -= 1;
                    self.food.remove(&new_head);
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

            for body_idx in 0..self.snakes[idx].body.len() {
                let coord = self.snakes[idx].body[body_idx];

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

    fn update_board(&mut self, game: &Game, food_buff: &mut Vec<Coord>, rng: &mut impl Rand) {
        let map = game.api.map;
        let rules = game.ruleset;

        if let Map::Empty = map {
            return;
        }

        let min_food = game.api.ruleset.settings.minimum_food;
        let chance = game.api.ruleset.settings.food_spawn_chance;

        let mut num_spawn = 0;

        if let Map::ArcadeMaze = map {
            let rand_val = rng.int_n(100);
            if rand_val <= chance {
                num_spawn = 1;
            }
        } else {
            let rand_val = 100 - rng.int_n(100);
            if self.num_food() < min_food {
                num_spawn = (min_food - self.num_food()) as usize;
            } else if rand_val < chance {
                num_spawn = 1;
            }
        };

        if num_spawn > 0 {
            food_buff.clear();

            if let Map::ArcadeMaze = map {
                for coord in &ARCADE_FOOD_COORDS {
                    if self.at(*coord) == BoardSquare::Empty {
                        food_buff.push(*coord);
                    };
                }
            } else {
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

                        food_buff.push(coord);
                    }
                }
            }

            if !food_buff.is_empty() {
                num_spawn = min(num_spawn, food_buff.len());
                rng.shuffle(food_buff, num_spawn);

                self.num_food += num_spawn as i32;

                for coord in food_buff.iter().take(num_spawn) {
                    self.food.insert(*coord);
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

    fn post_process(&mut self, game: &Game) {
        match game.ruleset {
            Rules::Constrictor => (),
            _ => return,
        }

        for s_idx in 0..self.num_snakes() as usize {
            if self.snakes[s_idx].alive() {
                self.snakes[s_idx].health = 100;
            }

            let snake_len = self.snakes[s_idx].body.len();

            let tail = self.snakes[s_idx].body[snake_len - 1];
            let sub_tail = self.snakes[s_idx].body[snake_len - 2];

            if tail != sub_tail {
                self.snakes[s_idx].body.push_back(tail);
            }
        }
    }
}
