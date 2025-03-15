use crate::board::{Board, BoardSquare, HeadOnCol};
use crate::config::MAX_BOARD_SIZE;
use crate::rand::{FastRand, Rand};
use crate::tests::common::{solo_game, test_game, test_snake, wrapped_game};
use crate::util::{Coord, Move};

use pretty_assertions::assert_eq;

#[test]
pub fn basic_str_test() {
    let game = test_game();

    let mut board = Board::new(7, 7);
    board.turn = 4;

    let snake = test_snake(&[Coord::new(3, 4), Coord::new(4, 4), Coord::new(4, 3)], 87);
    board.add_api_snake(&game, &snake).unwrap();

    board.set_at(Coord::new(5, 6), BoardSquare::Food);
    board.num_food += 1;

    board.set_at(Coord::new(1, 3), BoardSquare::Hazard);

    let board_string = board.to_string();
    let parsed_board = Board::from_str(board_string.as_str(), &game).unwrap();

    assert_eq!(board, parsed_board);
}

#[test]
pub fn multiple_snake_str_test() {
    let game = test_game();

    let mut board = Board::new(11, 11);
    board.turn = 4;

    board.set_at(Coord::new(10, 7), BoardSquare::Food);
    board.num_food += 1;

    let snake_0 = test_snake(&[Coord::new(3, 4), Coord::new(2, 4)], 87);
    let snake_1 = test_snake(
        &[
            Coord::new(5, 6),
            Coord::new(5, 5),
            Coord::new(6, 5),
            Coord::new(7, 5),
            Coord::new(7, 4),
        ],
        76,
    );

    board.add_api_snake(&game, &snake_0).unwrap();
    board.add_api_snake(&game, &snake_1).unwrap();

    let board_string = board.to_string();
    let parsed_board = Board::from_str(board_string.as_str(), &game).unwrap();

    assert_eq!(board, parsed_board);
}

#[test]
pub fn stacked_str_test() {
    let game = test_game();

    let mut board = Board::new(7, 7);

    let snake = test_snake(&[Coord::new(3, 5), Coord::new(3, 5), Coord::new(3, 5)], 100);
    board.add_api_snake(&game, &snake).unwrap();

    let board_string = board.to_string();
    let parsed_board = Board::from_str(board_string.as_str(), &game).unwrap();

    assert_eq!(board, parsed_board);
}

#[test]
pub fn hazard_str_test() {
    let game = test_game();

    let mut board = Board::new(11, 11);

    let snake = test_snake(
        &[Coord::new(3, 4), Coord::new(4, 4), Coord::new(4, 3), Coord::new(4, 2)],
        100,
    );

    board.set_at(Coord::new(3, 4), BoardSquare::Hazard);
    board.set_at(Coord::new(4, 4), BoardSquare::Hazard);
    board.set_at(Coord::new(4, 2), BoardSquare::Hazard);

    board.add_api_snake(&game, &snake).unwrap();

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
    - - ^ v < a
    - - ^ < - -
";

const BOARD_B: &str = "
    turn: 10 health: 45 health: 42
    > v - - - -
    ^ 1 - - - +
    ^ - 0 - - -
    ^ - ^ - - -
    c - ^ < < a
    - - - - - -
";

const BOARD_TAIL: &str = "
    turn: 10 health: 45 health: 42 health: 42
    - - - - - -
    - - b > > 1
    v e 0 - - -
    v - ^ - - -
    v - ^ < < a
    2 - - - - -
";

const BOARD_TAIL_HAZARD: &str = "
    turn: 10 health: 45 health: 42 health: 42
    * * * - - -
    * * B > > 1
    u E S - - -
    u * n - - -
    u * n < < a
    U * * - - -
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
    turn: 10 health: 45
    + - - - - -
    - - 0 < - -
    - - - ^ - +
    - - - ^ - -
    - - - ^ < a
    - - - - - -
";

const BOARD_Y: &str = "
    turn: 10 health: 45 health: 42
    - - - - - -
    - d - > > 0
    1 v - ^ - -
    ^ < - ^ - -
    - - - ^ < a
    - - - - - -
";

#[test]
pub fn move_to_coord_test() {
    let game = test_game();
    let board_a = Board::from_str(BOARD_A, &game).unwrap();

    let test_coord = Coord { x: 4, y: 3 };

    assert_eq!(
        board_a.move_to_coord(test_coord, Move::Left, game.ruleset),
        Coord { x: 3, y: 3 }
    );
    assert_eq!(
        board_a.move_to_coord(test_coord, Move::Right, game.ruleset),
        Coord { x: 5, y: 3 }
    );
    assert_eq!(
        board_a.move_to_coord(test_coord, Move::Up, game.ruleset),
        Coord { x: 4, y: 4 }
    );
    assert_eq!(
        board_a.move_to_coord(test_coord, Move::Down, game.ruleset),
        Coord { x: 4, y: 2 }
    );
}

#[test]
pub fn move_to_coord_wrapped_test() {
    let game = wrapped_game();
    let board_a = Board::from_str(BOARD_A, &game).unwrap();

    let test_coord = Coord { x: 0, y: 5 };

    assert_eq!(
        board_a.move_to_coord(test_coord, Move::Left, game.ruleset),
        Coord { x: 5, y: 5 }
    );
    assert_eq!(
        board_a.move_to_coord(test_coord, Move::Right, game.ruleset),
        Coord { x: 1, y: 5 }
    );
    assert_eq!(
        board_a.move_to_coord(test_coord, Move::Up, game.ruleset),
        Coord { x: 0, y: 0 }
    );
    assert_eq!(
        board_a.move_to_coord(test_coord, Move::Down, game.ruleset),
        Coord { x: 0, y: 4 }
    );
}

#[test]
pub fn next_to_test() {
    let game = test_game();
    let board = Board::from_str(BOARD_A, &game).unwrap();

    let a = Coord { x: 1, y: 1 };
    let b = Coord { x: 2, y: 2 };
    let c = Coord { x: 3, y: 3 };
    let d = Coord { x: 3, y: 4 };
    let e = Coord { x: 5, y: 2 };
    let f = Coord { x: 4, y: 2 };

    assert!(board.next_to(e, f, game.ruleset));
    assert!(!board.next_to(a, b, game.ruleset));
    assert!(!board.next_to(b, a, game.ruleset));
    assert!(board.next_to(c, d, game.ruleset));
    assert!(!board.next_to(d, e, game.ruleset));
}

#[test]
pub fn next_to_wrapped_test() {
    let game = wrapped_game();
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

    assert!(board.next_to(a, b, game.ruleset));
    assert!(board.next_to(b, a, game.ruleset));
    assert!(!board.next_to(e, f, game.ruleset));
    assert!(board.next_to(c, d, game.ruleset));
    assert!(!board.next_to(d, e, game.ruleset));

    assert!(!board.next_to(g, h, game.ruleset));
    assert!(!board.next_to(h, g, game.ruleset));
    assert!(board.next_to(h, i, game.ruleset));
    assert!(board.next_to(i, h, game.ruleset));
}

#[test]
pub fn coord_to_move_test() {
    let game = test_game();
    let board = Board::from_str(BOARD_A, &game).unwrap();

    let a = Coord { x: 1, y: 1 };
    let b = Coord { x: 1, y: 2 };
    let c = Coord { x: 0, y: 0 };
    let d = Coord { x: 1, y: 0 };

    assert_eq!(board.coord_to_move(a, b, game.ruleset), (Some(Move::Up), None));
    assert_eq!(board.coord_to_move(b, a, game.ruleset), (Some(Move::Down), None));
    assert_eq!(board.coord_to_move(c, d, game.ruleset), (Some(Move::Right), None));
    assert_eq!(board.coord_to_move(d, c, game.ruleset), (Some(Move::Left), None));

    // More than one coord away
    assert_eq!(
        board.coord_to_move(Coord::new(0, 3), Coord::new(5, 3), game.ruleset),
        (Some(Move::Right), None)
    );
    assert_eq!(
        board.coord_to_move(Coord::new(0, 3), Coord::new(1, 4), game.ruleset),
        (Some(Move::Right), Some(Move::Up))
    );
    assert_eq!(
        board.coord_to_move(Coord::new(0, 3), Coord::new(1, 5), game.ruleset),
        (Some(Move::Up), Some(Move::Right))
    );

    // Equal
    assert_eq!(
        board.coord_to_move(Coord::new(0, 3), Coord::new(0, 3), game.ruleset),
        (None, None)
    );
}

#[test]
pub fn coord_to_move_wrapped_test() {
    let game = wrapped_game();
    let board = Board::from_str(BOARD_A, &game).unwrap();

    // Immediately next to
    let a = Coord { x: 1, y: 1 };
    let b = Coord { x: 1, y: 2 };
    let c = Coord { x: 0, y: 0 };
    let d = Coord { x: 1, y: 0 };
    let e = Coord { x: 5, y: 0 };
    let f = Coord { x: 0, y: 5 };

    assert_eq!(board.coord_to_move(a, b, game.ruleset), (Some(Move::Up), None));
    assert_eq!(board.coord_to_move(b, a, game.ruleset), (Some(Move::Down), None));
    assert_eq!(board.coord_to_move(c, d, game.ruleset), (Some(Move::Right), None));
    assert_eq!(board.coord_to_move(d, c, game.ruleset), (Some(Move::Left), None));
    assert_eq!(board.coord_to_move(c, e, game.ruleset), (Some(Move::Left), None));
    assert_eq!(board.coord_to_move(e, c, game.ruleset), (Some(Move::Right), None));
    assert_eq!(board.coord_to_move(c, f, game.ruleset), (Some(Move::Down), None));
    assert_eq!(board.coord_to_move(d, c, game.ruleset), (Some(Move::Left), None));

    // More than one coord away
    assert_eq!(
        board.coord_to_move(Coord::new(0, 3), Coord::new(5, 3), game.ruleset),
        (Some(Move::Left), None)
    );
    assert_eq!(
        board.coord_to_move(Coord::new(0, 3), Coord::new(5, 4), game.ruleset),
        (Some(Move::Left), Some(Move::Up))
    );
    assert_eq!(
        board.coord_to_move(Coord::new(0, 3), Coord::new(5, 5), game.ruleset),
        (Some(Move::Up), Some(Move::Left))
    );

    // Equal
    assert_eq!(
        board.coord_to_move(Coord::new(0, 3), Coord::new(0, 3), game.ruleset),
        (None, None)
    );
}

#[test]
pub fn move_test() {
    let game = test_game();

    let board_a = Board::from_str(BOARD_A, &game).unwrap();
    let board_b = Board::from_str(BOARD_B, &game).unwrap();

    let head_a_1 = Coord::new(0, 3);
    let head_b_0 = Coord::new(2, 3);

    assert!(board_a.on_board(board_a.move_to_coord(head_a_1, Move::Right, game.ruleset)));
    assert!(!board_a.on_board(board_a.move_to_coord(head_a_1, Move::Left, game.ruleset)));
    assert!(board_b.on_board(board_b.move_to_coord(head_b_0, Move::Up, game.ruleset)));

    assert!(!board_a.valid_move(&game, 0, Move::Left));
    assert!(board_a.valid_move(&game, 0, Move::Right));
    assert!(board_a.valid_move(&game, 0, Move::Up));

    assert!(!board_a.valid_move(&game, 1, Move::Left));
    assert!(!board_a.valid_move(&game, 1, Move::Right));
    assert!(board_a.valid_move(&game, 1, Move::Up));
    assert!(!board_a.valid_move(&game, 1, Move::Down));

    assert!(board_b.valid_move(&game, 0, Move::Up));
    assert!(board_b.valid_move(&game, 1, Move::Right));
}

#[test]
pub fn tail_test() {
    let mut game = test_game();

    let board_tail = Board::from_str(BOARD_TAIL, &game).unwrap();

    game.api.ruleset.settings.hazard_damage_per_turn = 10;
    let mut board_hazard = Board::from_str(BOARD_TAIL_HAZARD, &game).unwrap();

    assert!(board_tail.valid_move(&game, 0, Move::Up));

    // Stacked, shouldn't be able to move into tail
    assert!(!board_tail.valid_move(&game, 0, Move::Left));

    assert!(board_hazard.valid_move(&game, 0, Move::Up));
    assert!(!board_hazard.valid_move(&game, 0, Move::Left));

    game.api.ruleset.settings.hazard_damage_per_turn = 80;
    board_hazard = Board::from_str(BOARD_TAIL_HAZARD, &game).unwrap();
    assert!(!board_hazard.valid_move(&game, 0, Move::Up));
    assert!(!board_hazard.valid_move(&game, 0, Move::Left));
}

#[test]
pub fn head_on_col_test() {
    let game = test_game();

    let board_b = Board::from_str(BOARD_B, &game).unwrap();

    assert_eq!(board_b.head_on_col(&game, 0, Move::Left), HeadOnCol::PossibleCollision);
    assert_eq!(board_b.head_on_col(&game, 0, Move::Right), HeadOnCol::None);
    assert_eq!(board_b.head_on_col(&game, 0, Move::Up), HeadOnCol::PossibleCollision);
    assert_eq!(board_b.head_on_col(&game, 0, Move::Down), HeadOnCol::None);

    assert_eq!(board_b.head_on_col(&game, 1, Move::Left), HeadOnCol::None);
    assert_eq!(
        board_b.head_on_col(&game, 1, Move::Right),
        HeadOnCol::PossibleElimination
    );
    assert_eq!(board_b.head_on_col(&game, 1, Move::Up), HeadOnCol::None);
    assert_eq!(
        board_b.head_on_col(&game, 1, Move::Down),
        HeadOnCol::PossibleElimination
    );
}

#[test]
pub fn head_on_col_wrapped_test() {
    let game = wrapped_game();

    let board_y = Board::from_str(BOARD_Y, &game).unwrap();

    assert_eq!(board_y.head_on_col(&game, 0, Move::Left), HeadOnCol::None);
    assert_eq!(
        board_y.head_on_col(&game, 0, Move::Right),
        HeadOnCol::PossibleElimination
    );
    assert_eq!(board_y.head_on_col(&game, 0, Move::Up), HeadOnCol::None);
    assert_eq!(
        board_y.head_on_col(&game, 0, Move::Down),
        HeadOnCol::PossibleElimination
    );

    assert_eq!(board_y.head_on_col(&game, 1, Move::Left), HeadOnCol::PossibleCollision);
    assert_eq!(board_y.head_on_col(&game, 1, Move::Right), HeadOnCol::None);
    assert_eq!(board_y.head_on_col(&game, 1, Move::Up), HeadOnCol::PossibleCollision);
    assert_eq!(board_y.head_on_col(&game, 1, Move::Down), HeadOnCol::None);
}

#[test]
pub fn closest_snake_test() {
    let game = test_game();

    let board_tail = Board::from_str(BOARD_TAIL, &game).unwrap();
    let board_x = Board::from_str(BOARD_X, &game).unwrap();

    assert_eq!(board_tail.closest_snake(&game, 0), Some(Coord::new(5, 4)));
    assert_eq!(board_x.closest_snake(&game, 0), None);
}

#[test]
pub fn gen_board_food_test() {
    let mut game = solo_game();
    let mut rng = FastRand::new();

    game.api.ruleset.settings.food_spawn_chance = 15;
    game.api.ruleset.settings.minimum_food = 5;

    let mut board_food = Board::from_str(BOARD_D, &game).unwrap();
    let mut food_buff = [Default::default(); MAX_BOARD_SIZE];

    board_food.gen_board(Move::Right as u32, &game, &mut food_buff, &mut rng);

    assert!(board_food.num_food() > 0);
}
