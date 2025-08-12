use super::*;
use crate::game::{Game, Map};

use std::collections::HashMap;
use std::fmt;

pub const EMPTY: u8 = 0;
pub const FOOD: u8 = 1 << BoardBit::Food as u8;
pub const HAZARD: u8 = 1 << BoardBit::Hazard as u8;
pub const SNAKE_HEAD: u8 = 1 << BoardBit::SnakeHead as u8;
pub const SNAKE_BODY: u8 = 1 << BoardBit::SnakeBody as u8;
pub const SNAKE_TAIL: u8 = 1 << BoardBit::SnakeTail as u8;
pub const FOOD_HAZARD: u8 = FOOD + HAZARD;
pub const SNAKE_HEAD_HAZARD: u8 = SNAKE_HEAD + HAZARD;
pub const SNAKE_BODY_HAZARD: u8 = SNAKE_BODY + HAZARD;
pub const SNAKE_TAIL_HAZARD: u8 = SNAKE_TAIL + HAZARD;

impl Board {
    // Assumes snake is not just a head (first turn)
    fn set_snake_idxs(&mut self, board_chars: &[char], tail_idx: usize, rules: Rules) -> u8 {
        let mut found = false;
        let mut snake_idx = 0;
        let mut snake_len = 0;
        let tail_coord = self.coord_from_idx(tail_idx);

        let mut curr_coord = tail_coord;

        while snake_len < self.height * self.width {
            let next_mv = match util::char_to_square(board_chars[self.idx_from_coord(curr_coord)]) {
                (SNAKE_TAIL | SNAKE_TAIL_HAZARD, _, _, Some(mv)) => mv,
                (SNAKE_BODY | SNAKE_BODY_HAZARD, _, _, Some(mv)) => mv,
                (SNAKE_HEAD | SNAKE_HEAD_HAZARD, idx, _, None) => {
                    snake_idx = idx;
                    found = true;
                    break;
                }
                _ => panic!("Snake body {curr_coord} had unexpected form {self}"),
            };

            curr_coord = self.move_to_coord(curr_coord, next_mv, rules);
            snake_len += 1;
        }

        if !found {
            panic!("Could not find snake given tail_idx {tail_idx}");
        }

        // Set index in snake squares, add body segments
        curr_coord = tail_coord;
        loop {
            let (sqr, _, num_stacked, mv_opt) = util::char_to_square(board_chars[self.idx_from_coord(curr_coord)]);
            assert!(
                is_bit_set(sqr, BoardBit::SnakeHead)
                    || is_bit_set(sqr, BoardBit::SnakeBody)
                    || is_bit_set(sqr, BoardBit::SnakeTail)
            );

            self.set_at_raw(curr_coord, sqr, BoardBit::Food);
            self.set_at_raw(curr_coord, snake_idx, BoardBit::SnakeIdx);

            for _ in 0..=num_stacked {
                self.snakes[snake_idx as usize].push_back(curr_coord);
            }

            let next_mv = match mv_opt {
                Some(mv) => mv,
                None => {
                    assert!(is_bit_set(sqr, BoardBit::SnakeHead));
                    break;
                }
            };

            curr_coord = self.move_to_coord(curr_coord, next_mv, rules);
        }

        let snake_len = self.snakes[snake_idx as usize].len as usize;
        self.snakes[snake_idx as usize].body[0..snake_len].reverse();

        snake_idx
    }

    fn to_string_internal(&self) -> String {
        let mut board_str = String::new();
        // Use `self.number` to refer to each positional data point.
        write!(&mut board_str, "turn: {} ", self.turn).unwrap();

        for s in 0..self.num_snakes() {
            write!(&mut board_str, "health: {} ", self.snakes[s as usize].health).unwrap();
        }

        writeln!(&mut board_str).unwrap();

        let mut char_array = vec!['-'; self.len() as usize];

        // Fill board from board_arr
        #[allow(clippy::needless_range_loop)]
        for idx in 0..self.len() as usize {
            let square = self.at_idx_raw(idx, BoardBit::Food);
            char_array[idx] = util::square_to_char(square, 0, 0, None);
        }

        // Fill snake moves from snakes
        for s_idx in 0..self.num_snakes() as usize {
            if self.snakes[s_idx].eliminated {
                continue;
            }

            let mut prev_coord = self.snake_tail(s_idx);
            let mut num_stacked = 0;
            for body_idx in 0..self.snakes[s_idx].len - 1 {
                let coord = self.snakes[s_idx].at_tail_offset(-1 - body_idx);
                if coord == prev_coord {
                    num_stacked += 1
                } else {
                    let (mv, secondary_mv) = self.coord_to_move(prev_coord, coord, Rules::Wrapped);
                    assert!(mv.is_some());
                    assert!(secondary_mv.is_none());

                    let prev_coord_idx = self.idx_from_coord(prev_coord);
                    char_array[prev_coord_idx] =
                        util::square_to_char(self.at_raw(prev_coord, BoardBit::Food), 0, num_stacked, mv);
                    num_stacked = 0;
                }
                prev_coord = coord;
            }

            // Set head
            let head_coord = self.snake_head(s_idx);
            let head_idx = self.idx_from_coord(head_coord);
            char_array[head_idx] = util::square_to_char(self.at_raw(head_coord, BoardBit::Food), s_idx as u8, 0, None);
        }

        for y in (0..self.height).rev() {
            for x in 0..self.width {
                let square_char = char_array[(x + y * self.width) as usize];
                write!(&mut board_str, "{square_char} ").unwrap();
            }
            if y != 0 {
                writeln!(&mut board_str).unwrap();
            }
        }
        board_str
    }

    pub fn from_str(board_str: &str, game: &Game) -> Result<Self, Error> {
        // Remove whitespace lines
        let lines: Vec<&str> = board_str
            .lines()
            .filter(|l| l.split_whitespace().next().is_some())
            .collect();

        let header: Vec<&str> = lines[0].split_whitespace().collect();

        assert_eq!(header.len() % 2, 0);
        assert!(header.len() >= 4);

        let mut board = Board::new(0, 0);

        assert_eq!(header[0], "turn:", "Invalid board str header: turn field");
        board.turn = header[1].parse::<i32>().unwrap();

        for (i, h) in header.iter().skip(2).enumerate() {
            match i % 2 {
                0 => {
                    assert_eq!(*h, "health:", "Invalid board str header: health field");
                    board.add_snake(&[], 0);
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

            if let Some(w_prev) = w_opt
                && w_prev != w_line
            {
                return Err(Error::BadBoardStr(
                    "Invalid board str, board width not consistent".to_owned(),
                ));
            }
            w_opt = Some(w_line);
        }

        let w = w_opt.unwrap() as i32;

        board.set_size(w, h);

        lines_vec.reverse();

        let chars_vec: Vec<char> = lines_vec.into_iter().flat_map(|line| line.into_iter()).collect();

        // Populate board matrix
        for (idx, char) in chars_vec.iter().enumerate() {
            let (board_square, ..) = util::char_to_square(*char);
            board.set_at_idx_raw(idx, board_square, BoardBit::Food);

            if is_bit_set(board_square, BoardBit::Food) {
                board.num_food += 1;
            }
        }

        let mut found_heads = HashMap::new();
        let mut found_tails = HashMap::new();

        // Populate board stats and snake indices
        for (square_idx, char) in chars_vec.iter().enumerate() {
            let (board_square, idx, _, mv_opt) = util::char_to_square(*char);

            if is_bit_set(board_square, BoardBit::SnakeHead) {
                assert!(mv_opt.is_none());
                board.set_at_idx_raw(square_idx, idx, BoardBit::SnakeIdx);
                found_heads.insert(idx, square_idx);
            } else if is_bit_set(board_square, BoardBit::SnakeTail) {
                let indexed_snake = board.set_snake_idxs(&chars_vec, square_idx, game.ruleset);
                found_tails.insert(indexed_snake, square_idx);
            }
        }

        if found_heads.len() < board.num_alive_snakes() as usize {
            panic!("Board was missing snakes from header")
        }

        // Handle edge case: heads only. we haven't found the tail for any remaining snakes
        // For string boards we assume this is always a length of 3
        for i in 0..board.num_snakes() as usize {
            if board.snakes[i].alive() && found_heads.contains_key(&(i as u8)) && !found_tails.contains_key(&(i as u8))
            {
                for _ in 0..3 {
                    let head = board.coord_from_idx(*found_heads.get(&(i as u8)).unwrap());
                    board.snakes[i].push_back(head);
                }
            }
        }

        // Set royale min/max markers
        if let Map::Royale = game.api.map {
            board.set_royale();
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
        writeln!(f, "width: {}", self.width)?;
        writeln!(f, "height: {}", self.height)?;
        writeln!(f, "turn: {}", self.turn)?;
        writeln!(f, "num_snakes: {}", self.num_snakes)?;
        writeln!(f, "num_food: {}", self.num_food)?;
        writeln!(f, "royale_min_x: {}", self.royale_min_x)?;
        writeln!(f, "royale_max_x: {}", self.royale_max_x)?;
        writeln!(f, "royale_min_y: {}", self.royale_min_y)?;
        writeln!(f, "royale_max_y: {}", self.royale_max_y)?;
        for i in 0..self.num_snakes {
            let snake = &self.snakes[i as usize];
            writeln!(
                f,
                "snake {i} health: {}, eliminated: {}, len: {}, tail_ptr: {}, head_ptr: {}",
                snake.health, snake.eliminated, snake.len, snake.tail_ptr, snake.head_ptr
            )?;
            writeln!(f, "snake {i} body: {:?}", &snake.body[..(snake.len as usize)])?;
        }
        writeln!(f, "snake_arr:").unwrap();
        for y in (0..self.height).rev() {
            for x in 0..self.width {
                write!(
                    f,
                    "{} ",
                    self.at_idx_raw((x + y * self.width) as usize, BoardBit::SnakeIdx)
                )
                .unwrap();
            }
            if y != 0 {
                writeln!(f).unwrap();
            }
        }
        writeln!(f).unwrap();
        fmt::Display::fmt(&self, f)
    }
}
