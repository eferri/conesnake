use super::*;
use crate::game::{Game, Map};

use std::collections::HashMap;

impl Board {
    // Assumes snake is not just a head (first turn)
    fn set_snake_idxs(&mut self, board_chars: &[char], tail_idx: usize, rules: Rules) -> u8 {
        let mut found = false;
        let mut snake_idx = 0;
        let mut snake_len = 0;
        let tail_coord = self.coord_from_idx(tail_idx);

        let mut curr_coord = tail_coord;
        let mut next_mv;

        while snake_len < self.height * self.width {
            let next_mv = match util::char_to_square(board_chars[self.idx_from_coord(curr_coord)]) {
                (BoardSquare::SnakeTail(_) | BoardSquare::SnakeTailHazard(_), _, Some(mv)) => mv,
                (BoardSquare::SnakeBody(_) | BoardSquare::SnakeBodyHazard(_), _, Some(mv)) => mv,
                (BoardSquare::SnakeHead(idx) | BoardSquare::SnakeHeadHazard(idx), _, None) => {
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
            next_mv = match util::char_to_square(board_chars[self.idx_from_coord(curr_coord)]) {
                (BoardSquare::SnakeTail(_), x, Some(mv)) => {
                    self.set_at(curr_coord, BoardSquare::SnakeTail(snake_idx));
                    for _ in 0..(x + 1) {
                        self.snakes[snake_idx as usize].body.push_back(curr_coord);
                    }
                    mv
                }
                (BoardSquare::SnakeTailHazard(_), x, Some(mv)) => {
                    self.set_at(curr_coord, BoardSquare::SnakeTailHazard(snake_idx));
                    for _ in 0..(x + 1) {
                        self.snakes[snake_idx as usize].body.push_back(curr_coord);
                    }
                    mv
                }
                (BoardSquare::SnakeBody(_), _, Some(mv)) => {
                    self.set_at(curr_coord, BoardSquare::SnakeBody(snake_idx));
                    self.snakes[snake_idx as usize].body.push_back(curr_coord);
                    mv
                }
                (BoardSquare::SnakeBodyHazard(_), _, Some(mv)) => {
                    self.set_at(curr_coord, BoardSquare::SnakeBodyHazard(snake_idx));
                    self.snakes[snake_idx as usize].body.push_back(curr_coord);
                    mv
                }
                (BoardSquare::SnakeHead(_) | BoardSquare::SnakeHeadHazard(_), _, None) => {
                    self.snakes[snake_idx as usize].body.push_back(curr_coord);
                    break;
                }
                _ => panic!("Snake body was not contiguous or had unexpected form {self}"),
            };

            curr_coord = self.move_to_coord(curr_coord, next_mv, rules);
        }

        self.snakes[snake_idx as usize].body.make_contiguous().reverse();

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
            char_array[idx] = util::square_to_char(square, 0, None);
        }

        // Fill snake moves from snakes
        for s_idx in 0..self.num_snakes() as usize {
            if self.snakes[s_idx].eliminated {
                continue;
            }

            let mut prev_coord = self.snake_tail(s_idx);
            let mut num_stacked = 0;
            for coord in self.snakes[s_idx].body.iter().rev().skip(1) {
                if *coord == prev_coord {
                    num_stacked += 1
                } else {
                    let (mv, secondary_mv) = self.coord_to_move(prev_coord, *coord, Rules::Wrapped);
                    assert!(mv.is_some());
                    assert!(secondary_mv.is_none());

                    let prev_coord_idx = self.idx_from_coord(prev_coord);
                    char_array[prev_coord_idx] = util::square_to_char(self.at(prev_coord), num_stacked, mv);
                    num_stacked = 0;
                }
                prev_coord = *coord;
            }

            // Set head
            let head_coord = self.snake_head(s_idx);
            let head_idx = self.idx_from_coord(head_coord);
            char_array[head_idx] = util::square_to_char(self.at(head_coord), 0, None);
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
                    if max_snakes == 0 {
                        board.snakes.push(Snake::new(0));
                    }
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

        if max_width == 0 && max_height == 0 && max_snakes == 0 {
            board.resize(w, h);
        }
        board.set_size(w, h);

        lines_vec.reverse();

        let chars_vec: Vec<char> = lines_vec.into_iter().flat_map(|line| line.into_iter()).collect();

        // Populate board matrix
        for (idx, char) in chars_vec.iter().enumerate() {
            let (board_square, ..) = util::char_to_square(*char);
            board.board_mat[idx] = board_square;

            match board_square {
                BoardSquare::Food | BoardSquare::FoodHazard => {
                    board.food.insert(board.coord_from_idx(idx));
                    board.num_food += 1
                }
                _ => (),
            }
        }

        let mut found_heads = HashMap::new();
        let mut found_tails = HashMap::new();

        // Populate board stats and snake indices
        for (square_idx, char) in chars_vec.iter().enumerate() {
            let (board_square, _, mv_opt) = util::char_to_square(*char);

            match (board_square, mv_opt) {
                (BoardSquare::SnakeHead(idx), None) | (BoardSquare::SnakeHeadHazard(idx), None) => {
                    found_heads.insert(idx, square_idx);
                }
                (BoardSquare::SnakeTail(_), _) | (BoardSquare::SnakeTailHazard(_), _) => {
                    let indexed_snake = board.set_snake_idxs(&chars_vec, square_idx, game.ruleset);
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
                    board.snakes[i].body.push_back(head);
                }
            }
            if !board.snakes[i].body.is_empty() {
                board.snakes[i].head = *board.snakes[i].body.front().unwrap();
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
