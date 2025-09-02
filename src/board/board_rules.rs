use super::*;

use crate::game::{Game, Map};
use crate::rand::{FastRand, Rand};
use crate::util::MOVES;

use std::cmp::max;

impl Board {
    // generate a random valid move for a snake
    pub fn gen_move(&self, game: &Game, snake_idx: usize, rng: &mut impl Rand) -> Move {
        let mut valid_moves = [0; 4];
        let mut num_valid = 0;

        for mv_idx in 0..4 {
            let is_valid = self.valid_move(game, snake_idx, mv_idx);
            if is_valid {
                valid_moves[num_valid as usize] = mv_idx;
                num_valid += 1;
            }
        }

        if num_valid > 0 {
            let mv_idx = rng.range(0, num_valid - 1);
            MOVES[valid_moves[mv_idx as usize]]
        } else {
            Move::Left
        }
    }

    pub fn gen_strong_move(&self, game: &Game, snake_idx: usize, rng: &mut impl Rand) -> Move {
        let mut best_move = None;

        let mut valid_moves = [0; 4];

        let mut neutral_moves = [0; 4];
        let mut good_moves = [0; 4];
        let mut bad_moves = [0; 4];

        let mut num_valid = 0;

        let mut num_neutral = 0;
        let mut num_good = 0;
        let mut num_bad = 0;

        for mv_idx in 0..4 {
            let is_valid = self.valid_move(game, snake_idx, mv_idx);
            if is_valid {
                valid_moves[num_valid as usize] = mv_idx;
                num_valid += 1;
                match self.head_on_col(game, snake_idx, Move::from_idx(mv_idx)) {
                    HeadOnCol::PossibleElimination => {
                        good_moves[num_good as usize] = mv_idx;
                        num_good += 1;
                    }
                    HeadOnCol::None => {
                        neutral_moves[num_neutral as usize] = mv_idx;
                        num_neutral += 1;
                    }
                    HeadOnCol::PossibleCollision => {
                        bad_moves[num_bad as usize] = mv_idx;
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

        Move::from_idx(best_move.unwrap_or(0))
    }

    #[inline(always)]
    pub fn valid_move(&self, game: &Game, snake_idx: usize, mv_idx: usize) -> bool {
        let head = self.snake_head(snake_idx);
        let coord = self.move_to_coord(head, mv_idx, game.ruleset);
        if !self.on_board(coord) {
            return false;
        }

        let sqr = self.at(coord);

        if any_bits_set(sqr, BoardBit::SnakeHead as u8 | BoardBit::SnakeBody as u8) {
            false
        } else if is_bit_set(sqr, BoardBit::SnakeTail) {
            let idx = self.snake_num(coord);
            let stacked = self.snake_tail(idx as usize) == self.snakes[idx as usize].at_tail_offset(-1);
            if !is_bit_set(sqr, BoardBit::Hazard) {
                !stacked
            } else {
                (self.snakes[snake_idx].health - game.api.ruleset.settings.hazard_damage_per_turn) > 0 && !stacked
            }
        } else {
            true
        }
    }

    #[inline(always)]
    pub fn is_trapped(&self, game: &Game, snake_idx: usize) -> bool {
        !self.valid_move(game, snake_idx, Move::Left.idx())
            && !self.valid_move(game, snake_idx, Move::Right.idx())
            && !self.valid_move(game, snake_idx, Move::Up.idx())
            && !self.valid_move(game, snake_idx, Move::Down.idx())
    }

    // Assumption: move is valid
    #[inline(always)]
    pub fn head_on_col(&self, game: &Game, snake_idx: usize, mv: Move) -> HeadOnCol {
        let snake_head = self.snake_head(snake_idx);
        let dest_square = self.move_to_coord(snake_head, mv.idx(), game.ruleset);
        for mv_idx in 0..4 {
            let adj_coord = self.move_to_coord(dest_square, mv_idx, game.ruleset);

            if !self.on_board(adj_coord) {
                continue;
            }

            if is_bit_set(self.at(adj_coord), BoardBit::SnakeHead) {
                let idx = self.snake_num(adj_coord);
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
    #[inline(always)]
    pub fn gen_board(&mut self, moves: u16, game: &Game, food_buff: &mut [usize], rng: &mut impl Rand) {
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

    #[inline(never)]
    pub fn move_snakes_asm(&mut self, moves: u16, game: &Game) {
        self.move_snakes(moves, game);
    }

    #[inline(always)]
    fn move_snakes(&mut self, moves: u16, game: &Game) {
        // StageMovementStandard
        // Move snakes board_arr tails only, Compute location of move
        for idx in 0..(self.num_snakes() as usize) {
            let snake = &self.snakes[idx];
            if !snake.alive() {
                continue;
            }
            let mv = Move::extract_idx(moves, idx as u32);

            let old_head = self.snake_head(idx);
            let new_head = self.move_to_coord(old_head, mv, game.ruleset);

            let old_tail = self.snakes[idx].pop_back();
            let new_tail = self.snake_tail(idx);

            self.snakes[idx].push_front(new_head);

            let old_sqr = self.at(old_tail);
            let new_sqr = self.at(new_tail);

            self.clear_snake_head_adj(old_head);

            if is_bit_set(old_sqr, BoardBit::SnakeTail) && is_bit_set(new_sqr, BoardBit::SnakeBody) {
                self.clear_bits_at(old_tail, BoardBit::SnakeTail as u8 | BoardBit::SnakeIdx as u8);
                self.set_at(new_tail, BoardBit::SnakeTail);
                self.clear_at(new_tail, BoardBit::SnakeBody);
            } else if is_bit_set(old_sqr, BoardBit::SnakeHead) {
                // Special cases: Snake was head only. This should only happen on first move
                self.set_at(old_tail, BoardBit::SnakeTail);
                self.clear_at(old_tail, BoardBit::SnakeHead);
            } else if is_bit_set(old_sqr, BoardBit::SnakeTail) && is_bit_set(new_sqr, BoardBit::SnakeTail) {
                // Special case: Stacked tails. no-op
            } else {
                panic!("snake {idx} tails set with invalid bits! old: {old_sqr}, new: {new_sqr}\n{self}");
            }
        }
    }

    #[inline(never)]
    pub fn update_health_asm(&mut self, game: &Game) {
        self.update_health(game);
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
                self.snakes[idx].clear_head = false;
                continue;
            }

            // StageStarvationStandard
            self.snakes[idx].health -= 1;

            // StageHazardDamageStandard
            // StageFeedSnakesStandard
            //  - Except for removal of food coords, which are only tracked on board
            let dest_square = self.at(dest);
            if is_bit_set(dest_square, BoardBit::Food) {
                let tail = self.snake_tail(idx);

                self.snakes[idx].health = 100;
                self.snakes[idx].push_back(tail);
            } else if is_bit_set(dest_square, BoardBit::Hazard) {
                let damage = game.api.ruleset.settings.hazard_damage_per_turn;
                self.snakes[idx].health = max(self.snakes[idx].health - damage, 0);
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
    fn eliminate_snakes(&mut self, moves: u16, game: &Game) {
        for idx in 0..(self.num_snakes() as usize) {
            if !self.snakes[idx].alive() {
                continue;
            }

            let mv = Move::extract_idx(moves, idx as u32);

            // Update old head, even for eliminated snakes
            let old_head = self.snakes[idx].at_head_offset(1);
            if old_head != self.snake_tail(idx) {
                self.clear_at(old_head, BoardBit::SnakeHead);
                self.set_at(old_head, BoardBit::SnakeBody);
            }

            let new_head = self.move_to_coord(old_head, mv, game.ruleset);

            // Track collisions by only setting new head if snake is alive
            let new_sqr = self.at(new_head);
            let new_snake_idx = self.snake_num(new_head) as usize;

            if is_bit_set(new_sqr, BoardBit::SnakeHead) {
                if self.snakes[new_snake_idx].eliminated
                    || (new_snake_idx < idx && self.snake_len(idx) > self.snake_len(new_snake_idx))
                {
                    self.set_snake_num(new_head, idx as u8);
                // Edge case: Equal length means we need to indicate the other snake is dead too
                } else if new_snake_idx < idx && self.snake_len(idx) == self.snake_len(new_snake_idx) {
                    self.clear_bits_at(new_head, BoardBit::SnakeHead as u8 | BoardBit::SnakeIdx as u8);
                }
            } else if any_bits_set(new_sqr, BoardBit::SnakeBody as u8 | BoardBit::SnakeTail as u8) {
                if self.snakes[new_snake_idx].eliminated {
                    self.set_at(new_head, BoardBit::SnakeHead);
                    self.clear_bits_at(new_head, BoardBit::SnakeBody as u8 | BoardBit::SnakeTail as u8);
                    self.set_snake_num(new_head, idx as u8);
                }
            } else {
                if is_bit_set(new_sqr, BoardBit::Food) {
                    self.num_food -= 1;
                    self.clear_at(new_head, BoardBit::Food);
                }

                self.set_at(new_head, BoardBit::SnakeHead);
                self.set_snake_num(new_head, idx as u8);
            }
        }

        // Remove eliminated snakes
        for idx in 0..self.num_snakes() as usize {
            if !self.snakes[idx].alive() && !self.snakes[idx].eliminated {
                continue;
            }

            if !self.snakes[idx].eliminated {
                // If snake head is not set properly and snake has not yet been eliminated for other reasons
                // then snake was eliminated by a collision
                if is_bit_set(self.at(self.snake_head(idx)), BoardBit::SnakeHead) {
                    let other_idx = self.snake_num(self.snake_head(idx));
                    if other_idx as usize != idx {
                        self.snakes[idx].health = 0;
                        self.snakes[idx].eliminated = true;
                        self.snakes[idx].clear_head = false;
                    } else {
                        continue;
                    }
                } else {
                    self.snakes[idx].health = 0;
                    self.snakes[idx].eliminated = true;
                    self.snakes[idx].clear_head = false;
                }
            }

            let start_iter = if self.snakes[idx].clear_head { 0 } else { 1 };
            for body_idx in start_iter..self.snakes[idx].len {
                let coord = self.snakes[idx].at_head_offset(body_idx);

                let snake_idx = self.snake_num(coord) as usize;
                if snake_idx == idx {
                    self.clear_bits_at(
                        coord,
                        BoardBit::SnakeHead as u8
                            | BoardBit::SnakeBody as u8
                            | BoardBit::SnakeTail as u8
                            | BoardBit::SnakeIdx as u8,
                    );
                }
            }
        }
    }

    #[inline(never)]
    pub fn update_board_asm(&mut self, game: &Game, food_buff: &mut [usize], rng: &mut FastRand) {
        self.update_board(game, food_buff, rng);
    }

    #[inline(always)]
    fn update_board(&mut self, game: &Game, food_buff: &mut [usize], rng: &mut impl Rand) {
        let map = game.api.map;
        let rules = game.ruleset;

        if let Map::Empty = map {
            return;
        }

        let min_food = game.api.ruleset.settings.minimum_food;
        let chance = game.api.ruleset.settings.food_spawn_chance;

        let mut num_spawn = 0;

        let rand_val = 100 - rng.int_n(100);
        if self.num_food < min_food {
            num_spawn = (min_food - self.num_food) as usize;
        } else if rand_val < chance {
            num_spawn = 1;
        }

        let mut num_unnocupied = 0;

        for idx in 0..self.num_snakes() as usize {
            if self.snakes[idx].alive() {
                let head = self.snake_head(idx);
                self.set_snake_head_adj(head);
            }
        }

        if num_spawn > 0 {
            // Iterate over height dimension to match rules
            let mut coord_idx = 0;

            let board_len = self.len() as usize;

            #[allow(clippy::explicit_counter_loop)]
            for _ in 0..board_len {
                let square = self.at_idx(coord_idx);

                if !any_bits_set(
                    square,
                    BoardBit::SnakeHead as u8
                        | BoardBit::SnakeBody as u8
                        | BoardBit::SnakeTail as u8
                        | BoardBit::SnakeHeadAdj as u8
                        | BoardBit::Food as u8,
                ) {
                    food_buff[num_unnocupied] = coord_idx;
                    num_unnocupied += 1;
                };

                // When testing iterate over height dimension first
                // to match rules behavior
                #[cfg(test)]
                {
                    coord_idx += self.width as usize;
                    if coord_idx >= board_len {
                        coord_idx = coord_idx % self.height as usize + 1;
                    }
                }
                #[cfg(not(test))]
                {
                    coord_idx += 1;
                }
            }

            if num_unnocupied != 0 {
                num_spawn = min(num_spawn, num_unnocupied);
                rng.shuffle(&mut food_buff[0..num_unnocupied], num_spawn);

                self.num_food += num_spawn as i32;

                for idx in food_buff.iter().take(num_spawn) {
                    self.set_at_idx(*idx, BoardBit::Food);
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
                Coord::new(z as i8, index as i8, self.width)
            } else {
                Coord::new(index as i8, z as i8, self.width)
            };

            self.set_at(curr_coord, BoardBit::Hazard);
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
