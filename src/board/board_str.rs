use super::*;
use crate::game::{Game, Map};

use std::collections::HashMap;
use std::fmt;

impl Board {
    // Assumes snake is not just a head (first turn)
    fn populate_snake(&mut self, board_chars: &[char], tail_idx: usize, rules: Rules) -> u8 {
        let mut found = false;
        let mut snake_idx = 0;
        let mut snake_len = 0;
        let tail_coord = self.coord_from_idx(tail_idx);

        let mut curr_coord = tail_coord;
        let mut next_mv;

        // First iteration: find snake index
        while snake_len < self.height * self.width {
            let next_mv = match util::char_to_square(board_chars[self.idx_from_coord(curr_coord)]) {
                (BoardSquare::SnakeTail(_, mv) | BoardSquare::SnakeTailHazard(_, mv), _) => mv,
                (BoardSquare::SnakeBody(_, mv) | BoardSquare::SnakeBodyHazard(_, mv), _) => mv,
                (BoardSquare::SnakeHead(idx) | BoardSquare::SnakeHeadHazard(idx), _) => {
                    snake_idx = idx;
                    found = true;
                    snake_len += 1;
                    break;
                }
                _ => panic!("Snake body {curr_coord} had unexpected form \n{self}"),
            };

            curr_coord = self.move_to_coord(curr_coord, next_mv, rules);
            snake_len += 1;
        }

        if !found {
            panic!("Could not find snake given tail_idx {tail_idx}");
        }

        self.snakes[snake_idx as usize].head = curr_coord;
        self.snakes[snake_idx as usize].old_head = Default::default();
        self.snakes[snake_idx as usize].tail = tail_coord;

        // Set index in snake squares, add body segments
        curr_coord = tail_coord;
        loop {
            next_mv = match util::char_to_square(board_chars[self.idx_from_coord(curr_coord)]) {
                (BoardSquare::SnakeTail(_, mv), num_stacked) => {
                    self.set_at(curr_coord, BoardSquare::SnakeTail(snake_idx, mv));
                    self.snakes[snake_idx as usize].num_stacked = num_stacked;
                    mv
                }
                (BoardSquare::SnakeTailHazard(_, mv), num_stacked) => {
                    self.set_at(curr_coord, BoardSquare::SnakeTailHazard(snake_idx, mv));
                    self.snakes[snake_idx as usize].num_stacked = num_stacked;
                    mv
                }
                (BoardSquare::SnakeBody(_, mv), _) => {
                    self.set_at(curr_coord, BoardSquare::SnakeBody(snake_idx, mv));
                    mv
                }
                (BoardSquare::SnakeBodyHazard(_, mv), _) => {
                    self.set_at(curr_coord, BoardSquare::SnakeBodyHazard(snake_idx, mv));
                    mv
                }
                (BoardSquare::SnakeHead(_) | BoardSquare::SnakeHeadHazard(_), _) => {
                    break;
                }
                _ => panic!("Snake body was not contiguous or had unexpected form {self}"),
            };

            curr_coord = self.move_to_coord(curr_coord, next_mv, rules);
        }

        self.snakes[snake_idx as usize].len = snake_len + self.snakes[snake_idx as usize].num_stacked;

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

        // Fill board from board_mat
        #[allow(clippy::needless_range_loop)]
        for idx in 0..self.len() as usize {
            let square = self.board_mat[idx];
            char_array[idx] = util::square_to_char(square, 0);
        }

        // Update tail if stacked
        for s_idx in 0..self.num_snakes() as usize {
            let tail = self.snake_tail(s_idx);
            let tail_idx = self.idx_from_coord(tail);
            if self.snakes[s_idx].num_stacked > 0 {
                char_array[tail_idx] = util::square_to_char(self.at(tail), self.snakes[s_idx].num_stacked);
            }
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

    pub fn from_str(inp: &str, game: &Game) -> Result<Self, Error> {
        // Remove whitespace lines
        let lines: Vec<&str> = inp.lines().filter(|l| l.split_whitespace().next().is_some()).collect();

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

        board.set_size(w, h);

        lines_vec.reverse();

        let chars_vec: Vec<char> = lines_vec.into_iter().flat_map(|line| line.into_iter()).collect();

        // Populate board matrix
        for (idx, char) in chars_vec.iter().enumerate() {
            let (board_square, _) = util::char_to_square(*char);
            board.board_mat[idx] = board_square;

            match board_square {
                BoardSquare::Food | BoardSquare::FoodHazard => board.num_food += 1,
                _ => (),
            }
        }

        let mut found_heads = HashMap::new();
        let mut found_tails = HashMap::new();

        // Populate board stats and snake indices
        for (square_idx, char) in chars_vec.iter().enumerate() {
            let (board_square, _) = util::char_to_square(*char);

            match board_square {
                BoardSquare::SnakeHead(idx) | BoardSquare::SnakeHeadHazard(idx) => {
                    found_heads.insert(idx, square_idx);
                }
                BoardSquare::SnakeTail(_, _) | BoardSquare::SnakeTailHazard(_, _) => {
                    let indexed_snake = board.populate_snake(&chars_vec, square_idx, game.ruleset);
                    found_tails.insert(indexed_snake, square_idx);
                }
                _ => (),
            };
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
                    board.snakes[i].head = head;
                    board.snakes[i].old_head = Default::default();
                    board.snakes[i].tail = head;
                    board.snakes[i].num_stacked = 2;
                    board.snakes[i].len = 3;
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
