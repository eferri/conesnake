use crate::board::{Board, BoardSquare};
use crate::tests::common::{solo_game, test_game, wrapped_game};
use crate::util::{Coord, Move};

#[test]
pub fn basic_str_test() {
    let game = test_game();

    let mut board = Board::new(15, 15, 15, 15, 1);
    board.add_snakes(1, Default::default());
    board.turn = 4;

    board.snakes[0].head = Coord { x: 3, y: 3 };
    board.snakes[0].tail = Coord { x: 4, y: 3 };
    board.snakes[0].health = 87;
    board.snakes[0].len = 4;
    board.snakes[0].alive = true;

    board.set_at(Coord::new(10, 7), BoardSquare::Food);
    board.num_food += 1;

    board.set_at(board.our_head(), BoardSquare::SnakeHead(0, 0));
    board.set_at(Coord::new(3, 4), BoardSquare::SnakeBody(0, Move::Down));
    board.set_at(Coord::new(4, 4), BoardSquare::SnakeBody(0, Move::Left));
    board.set_at(Coord::new(4, 3), BoardSquare::SnakeTail(0, Move::Up, 0));

    board.set_at(Coord::new(8, 10), BoardSquare::Hazard);

    let board_string = board.to_string();
    let parsed_board = Board::from_str(board_string.as_str(), &game).unwrap();

    assert_eq!(board, parsed_board);
}

#[test]
pub fn multiple_snake_str_test() {
    let game = test_game();

    let mut board = Board::new(15, 15, 15, 15, 2);
    board.add_snakes(2, Default::default());
    board.turn = 4;

    board.snakes[0].head = Coord { x: 3, y: 3 };
    board.snakes[0].tail = Coord { x: 2, y: 4 };
    board.snakes[0].health = 87;
    board.snakes[0].len = 4;
    board.snakes[0].alive = true;

    board.snakes[1].head = Coord { x: 5, y: 6 };
    board.snakes[1].tail = Coord { x: 7, y: 4 };
    board.snakes[1].health = 76;
    board.snakes[1].len = 5;
    board.snakes[1].alive = true;

    board.set_at(Coord::new(10, 7), BoardSquare::Food);
    board.num_food += 1;

    board.set_at(board.our_head(), BoardSquare::SnakeHead(0, 0));
    board.set_at(Coord::new(3, 4), BoardSquare::SnakeBody(0, Move::Down));
    board.set_at(Coord::new(2, 4), BoardSquare::SnakeTail(0, Move::Right, 1));

    board.set_at(Coord::new(5, 6), BoardSquare::SnakeHead(1, 0));
    board.set_at(Coord::new(5, 5), BoardSquare::SnakeBody(1, Move::Up));
    board.set_at(Coord::new(6, 5), BoardSquare::SnakeBody(1, Move::Left));
    board.set_at(Coord::new(7, 5), BoardSquare::SnakeBody(1, Move::Left));
    board.set_at(Coord::new(7, 4), BoardSquare::SnakeTail(1, Move::Up, 0));

    let board_string = board.to_string();
    let parsed_board = Board::from_str(board_string.as_str(), &game).unwrap();

    assert_eq!(board, parsed_board);
}

#[test]
pub fn stacked_str_test() {
    let game = test_game();

    let mut board = Board::new(15, 15, 15, 15, 1);
    board.add_snakes(1, Default::default());

    board.snakes[0].head = Coord { x: 8, y: 10 };
    board.snakes[0].tail = Coord { x: 8, y: 10 };
    board.snakes[0].health = 100;
    board.snakes[0].len = 3;
    board.snakes[0].alive = true;

    board.set_at(Coord::new(10, 7), BoardSquare::Food);
    board.num_food += 1;

    board.set_at(board.our_head(), BoardSquare::SnakeHead(0, 2));

    let board_string = board.to_string();
    let parsed_board = Board::from_str(board_string.as_str(), &game).unwrap();

    assert_eq!(board, parsed_board);
}

const BOARD_A: &str = "
    turn: 10 health: 45 health: 42
    - - - - - -
    - d - - - -
    1 v - - - -
    ^ < 0 - - -
    - - ^ v < l
    - - ^ < - -
";

const BOARD_B: &str = "
    turn: 10 health: 45 health: 42
    > v - - - -
    ^ 1 - - - -
    ^ - 0 - - -
    ^ - ^ - - -
    u - ^ < < l
    - - - - - -
";

const BOARD_TAIL: &str = "
    turn: 10 health: 45 health: 42 health: 42
    - - - - - -
    - - r > > 1
    v L 0 - - -
    v - ^ - - -
    v - ^ < < l
    2 - - - - -
";

const BOARD_D: &str = "
    turn: 1 health: 42
    - - - - - -
    - - - - - -
    - - 0 + - -
    - - - - - -
    - - - - - -
    - - - - - -
";

const BOARD_X: &str = "
    turn: 45  health: 44
    - - - - - -
    - - - - - -
    - 0 < - - -
    - - ^ - - -
    - - ^ < l -
    - - - - - -
";

#[test]
pub fn move_to_coord_test() {
    let game = test_game();
    let rules = game.api.ruleset.name;
    let board_a = Board::from_str(BOARD_A, &game).unwrap();

    let test_coord = Coord { x: 4, y: 3 };

    assert_eq!(
        board_a.move_to_coord(test_coord, Move::Left, rules),
        Coord { x: 3, y: 3 }
    );
    assert_eq!(
        board_a.move_to_coord(test_coord, Move::Right, rules),
        Coord { x: 5, y: 3 }
    );
    assert_eq!(board_a.move_to_coord(test_coord, Move::Up, rules), Coord { x: 4, y: 4 });
    assert_eq!(
        board_a.move_to_coord(test_coord, Move::Down, rules),
        Coord { x: 4, y: 2 }
    );
}

#[test]
pub fn move_to_coord_wrapped_test() {
    let game = wrapped_game();
    let rules = game.api.ruleset.name;
    let board_a = Board::from_str(BOARD_A, &game).unwrap();

    let test_coord = Coord { x: 0, y: 5 };

    assert_eq!(
        board_a.move_to_coord(test_coord, Move::Left, rules),
        Coord { x: 5, y: 5 }
    );
    assert_eq!(
        board_a.move_to_coord(test_coord, Move::Right, rules),
        Coord { x: 1, y: 5 }
    );
    assert_eq!(board_a.move_to_coord(test_coord, Move::Up, rules), Coord { x: 0, y: 0 });
    assert_eq!(
        board_a.move_to_coord(test_coord, Move::Down, rules),
        Coord { x: 0, y: 4 }
    );
}

#[test]
pub fn next_to_test() {
    let game = test_game();
    let rules = game.api.ruleset.name;
    let board = Board::from_str(BOARD_A, &game).unwrap();

    let a = Coord { x: 1, y: 1 };
    let b = Coord { x: 2, y: 2 };
    let c = Coord { x: 3, y: 3 };
    let d = Coord { x: 3, y: 4 };
    let e = Coord { x: 5, y: 2 };
    let f = Coord { x: 4, y: 2 };

    assert!(board.next_to(e, f, rules));
    assert!(!board.next_to(a, b, rules));
    assert!(!board.next_to(b, a, rules));
    assert!(board.next_to(c, d, rules));
    assert!(!board.next_to(d, e, rules));
}

#[test]
pub fn next_to_wrapped_test() {
    let game = wrapped_game();
    let rules = game.api.ruleset.name;
    let board = Board::from_str(BOARD_A, &game).unwrap();

    let a = Coord { x: 0, y: 4 };
    let b = Coord { x: 5, y: 4 };
    let c = Coord { x: 3, y: 5 };
    let d = Coord { x: 3, y: 0 };
    let e = Coord { x: 0, y: 5 };
    let f = Coord { x: 5, y: 0 };

    let g = Coord { x: 1, y: 1 };
    let h = Coord { x: 2, y: 2 };
    let i = Coord { x: 3, y: 2 };

    assert!(board.next_to(a, b, rules));
    assert!(board.next_to(b, a, rules));
    assert!(!board.next_to(e, f, rules));
    assert!(board.next_to(c, d, rules));
    assert!(!board.next_to(d, e, rules));

    assert!(!board.next_to(g, h, rules));
    assert!(!board.next_to(h, g, rules));
    assert!(board.next_to(h, i, rules));
    assert!(board.next_to(i, h, rules));
}

#[test]
pub fn coord_to_move_test() {
    let game = test_game();
    let rules = game.api.ruleset.name;
    let board = Board::from_str(BOARD_A, &game).unwrap();

    let a = Coord { x: 1, y: 1 };
    let b = Coord { x: 1, y: 2 };
    let c = Coord { x: 0, y: 0 };
    let d = Coord { x: 1, y: 0 };

    assert_eq!(board.coord_to_move(a, b, rules), Move::Up);
    assert_eq!(board.coord_to_move(b, a, rules), Move::Down);
    assert_eq!(board.coord_to_move(c, d, rules), Move::Right);
    assert_eq!(board.coord_to_move(d, c, rules), Move::Left);
}

#[test]
pub fn coord_to_move_wrapped_test() {
    let game = wrapped_game();
    let rules = game.api.ruleset.name;
    let board = Board::from_str(BOARD_A, &game).unwrap();

    let a = Coord { x: 1, y: 1 };
    let b = Coord { x: 1, y: 2 };
    let c = Coord { x: 0, y: 0 };
    let d = Coord { x: 1, y: 0 };
    let e = Coord { x: 5, y: 0 };
    let f = Coord { x: 0, y: 5 };

    assert_eq!(board.coord_to_move(a, b, rules), Move::Up);
    assert_eq!(board.coord_to_move(b, a, rules), Move::Down);
    assert_eq!(board.coord_to_move(c, d, rules), Move::Right);
    assert_eq!(board.coord_to_move(d, c, rules), Move::Left);
    assert_eq!(board.coord_to_move(c, e, rules), Move::Left);
    assert_eq!(board.coord_to_move(e, c, rules), Move::Right);
    assert_eq!(board.coord_to_move(c, f, rules), Move::Down);
    assert_eq!(board.coord_to_move(d, c, rules), Move::Left);
}

#[test]
pub fn move_test() {
    let game = test_game();
    let rules = game.api.ruleset.name;

    let board_a = Board::from_str(BOARD_A, &game).unwrap();
    let board_b = Board::from_str(BOARD_B, &game).unwrap();
    let head_a_1 = Coord::new(2, 2);
    let head_a_2 = Coord::new(0, 3);
    let head_b_1 = Coord::new(2, 3);
    let head_b_2 = Coord::new(1, 4);

    assert!(board_a.on_board(board_a.move_to_coord(head_a_2, Move::Right, rules)));
    assert!(!board_a.on_board(board_a.move_to_coord(head_a_2, Move::Left, rules)));
    assert!(board_b.on_board(board_b.move_to_coord(head_b_1, Move::Up, rules)));

    assert!(!board_a.valid_move(head_a_1, Move::Left, rules));
    assert!(board_a.valid_move(head_a_1, Move::Right, rules));
    assert!(board_a.valid_move(head_a_1, Move::Up, rules));

    assert!(!board_a.valid_move(head_a_2, Move::Left, rules));
    assert!(!board_a.valid_move(head_a_2, Move::Right, rules));
    assert!(board_a.valid_move(head_a_2, Move::Up, rules));
    assert!(!board_a.valid_move(head_a_2, Move::Down, rules));

    assert!(board_b.valid_move(head_b_1, Move::Up, rules));
    assert!(board_b.valid_move(head_b_2, Move::Right, rules));
}

#[test]
pub fn tail_test() {
    let game = test_game();
    let rules = game.api.ruleset.name;

    let board_tail = Board::from_str(BOARD_TAIL, &game).unwrap();
    let head_tail_0 = Coord::new(2, 3);

    assert!(board_tail.valid_move(head_tail_0, Move::Up, rules));

    // Stacked, shouldn't be able to move into tail
    assert!(!board_tail.valid_move(head_tail_0, Move::Left, rules));
}

#[test]
pub fn iter_test() {
    let game = test_game();

    let board = Board::from_str(BOARD_X, &game).unwrap();
    for idx in 0..board.len() {
        let coord = board.coord_from_idx(idx as i32);
        match board.at(coord) {
            BoardSquare::SnakeHead(0, 0) => assert_eq!(coord, Coord::new(1, 3)),
            BoardSquare::SnakeTail(0, Move::Left, 0) => assert_eq!(coord, Coord::new(4, 1)),
            BoardSquare::SnakeBody(0, Move::Up) => assert!(coord == Coord::new(2, 2) || coord == Coord::new(2, 1)),
            BoardSquare::SnakeBody(0, Move::Left) => assert!(coord == Coord::new(3, 1) || coord == Coord::new(2, 3)),
            BoardSquare::Empty => (),
            _ => panic!("Unexpected BoardSquare"),
        }
    }
}

#[test]
pub fn gen_board_food_test() {
    let mut game = solo_game();

    game.api.ruleset.settings.food_spawn_chance = 15;
    game.api.ruleset.settings.minimum_food = 5;

    let board_food = Board::from_str(BOARD_D, &game).unwrap();
    let mut food_buff = Vec::with_capacity((board_food.max_height * board_food.max_width) as usize);

    let gen_board_food_right = board_food.gen_board(&[Move::Right], &game, &mut food_buff);

    assert!(gen_board_food_right.num_food() > 0);
}
