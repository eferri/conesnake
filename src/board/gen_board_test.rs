use crate::board::Board;
use crate::game::{Map, Rules};
use crate::tests::{common::test_game, ref_move::RefMove};
use crate::util::Move;

#[test]
fn gen_board_ref_test() {
    let mut ref_gen = RefMove::new();

    let test_boards = vec![
        (
            "turn: 10 health: 45
            - - - - - -
            - - - - - -
            - - 0 - - -
            - - ^ - - -
            - - ^ < < l
            - - - - - -",
            Rules::Solo,
            Map::Standard,
        ),
        (
            "turn: 1 health: 42
            - - - - - -
            - - - - - -
            - - 0 + - -
            - - - - - -
            - - - - - -
            - - - - - -",
            Rules::Solo,
            Map::Standard,
        ),
        (
            "turn: 2 health: 64
            - - - - - -
            - - - - - -
            - 0 L - - -
            - - - - - -
            - - - - - -
            - - - - - -",
            Rules::Solo,
            Map::Standard,
        ),
        (
            "turn: 1 health: 100
            - - - - - -
            - - - - - -
            - - - - - -
            - 0 - - - -
            - - - - - -
            - - - - - -",
            Rules::Solo,
            Map::Standard,
        ),
        (
            "turn: 45 health: 44
            - - - - - -
            - - - - - -
            - 0 < - - -
            - - ^ - - -
            - - ^ < l -
            - - - - - -",
            Rules::Solo,
            Map::Standard,
        ),
        (
            "turn: 255 health: 1
            - - - - - -
            - - - - - -
            - > 0 + - -
            - ^ - - - -
            - ^ - - - -
            - ^ l - - -",
            Rules::Solo,
            Map::Standard,
        ),
        (
            // Head
            "turn: 12 health: 93 health: 53 health: 29
            - - - - - -
            - - r > > 1
            2 - 0 - - -
            ^ - ^ - - -
            ^ - ^ < < l
            u - - - - -",
            Rules::Standard,
            Map::Standard,
        ),
        (
            // Head 2
            "turn: 12 health: 93 health: 53 health: 29
            - - - - - -
            - - r > > 1
            0 - 2 - - -
            ^ - ^ - - -
            ^ - ^ < < l
            u - - - - -",
            Rules::Standard,
            Map::Standard,
        ),
        (
            // Head 3
            "turn: 12 health: 93 health: 53 health: 29
            - - - - - -
            - - r > > 1
            0 - 2 - - -
            ^ - ^ - - -
            ^ - ^ < l -
            ^ l - - - -",
            Rules::Standard,
            Map::Standard,
        ),
        (
            // Food
            "turn: 13 health: 72 health: 53 health: 9
            - - - - - -
            - r > > > 1
            2 - - - - +
            ^ - r > v 0
            ^ - - - > ^
            u - - - - -",
            Rules::Standard,
            Map::Standard,
        ),
        (
            // Food 2
            "turn: 13 health: 72 health: 53 health: 9
            - - - - - -
            - r > > > 1
            2 - - - - +
            ^ - - r v 0
            ^ - - - > ^
            u - - - - -",
            Rules::Standard,
            Map::Standard,
        ),
        (
            // Tail
            "turn: 10 health: 45 health: 42 health: 42
            - - - - - -
            - - r > > 1
            v L 0 - - -
            v - ^ - - -
            v - ^ < < l
            2 - - - - -",
            Rules::Standard,
            Map::Standard,
        ),
        (
            // Challenge
            "turn: 1 health: 100 health: 100 health: 100
            - - - - - - -
            - 1 r v - - -
            - ^ - v - - -
            - ^ < < - - -
            - - - - 2 < <
            r > 0 - d - ^
            - - - - > > ^",
            Rules::Standard,
            Map::Standard,
        ),
        (
            // Simultaneous body
            "turn: 1 health: 45 health: 43
            r > v - - - -
            - 1 v - - - -
            - ^ 0 d - - -
            - ^ < < - - -
            - - - - - - -
            - - - - - - -
            - - - - - - -",
            Rules::Standard,
            Map::Standard,
        ),
        (
            "turn: 3 health: 97 health: 97 health: 97 health: 97 health: 97
            - - - - - - - v < l 4
            - > > 1 r v - v - - ^
            - ^ - - - v - v - - ^
            - ^ - - - v - v - - ^
            - ^ < < < < - > > > ^
            - - - - v L - - - - -
            3 < < < v > > > > v -
            d - - ^ 0 ^ - - - v -
            v - - ^ - ^ - - - v -
            v - - ^ - ^ < < l 2 -
            > > > ^ - - - - - - - ",
            Rules::Standard,
            Map::Standard,
        ),
        (
            // Arcade Maze
            "turn: 3 health: 97 health: 97 health: 97
            * - * * * * * * * * * * * * * * * v *
            * - - - - - - - - * - - - - - - - 1 *
            * - * * - * * * - * - * * * - * * - *
            * 0 < l - - - - - - - - - - - - - - *
            * - * * - * - * * * * * - * - * * - *
            * - - - - * - - - * - - - * - - - - *
            * - - * - * * * - * - * * * - * - - *
            * - - * - * - - - - - - - * - * - - *
            * * * * - * - * - * - * - * - * * * *
            - - - - v l - * - - - * - - - - - - -
            * * * * 2 * - * - * - * - * - * * * *
            * - - * - * - - - - - - - * - * - - *
            * - - * - * - * * * * * - * - * - - *
            * - - - - - - - - * - - - - - - - - *
            * - * * - * * * - * - * * * - * * - *
            * - - * - - - - - - - - - - - * - - *
            * * - * - * - * * * * * - * - * - * *
            * - - - - * - - - * - - - * - - - - *
            * - * * * * * * - * - * * * * * * - *
            * - - - - - - - - - - - - - - - - - *
            * - * * * * * * * * * * * * * * * d *",
            Rules::Wrapped,
            Map::ArcadeMaze,
        ),
        (
            // Head-to-head on food same length
            "turn: 17 health: 93 health: 97 health: 52 health: 89
            - - - - - - - - - - -
            - - - - - d - - - - -
            - - - - - v - - - - -
            - - - - - v - - - - -
            - - - - - 2 - - - - -
            1 < - - - + - - - - -
            r ^ - - - 0 < - - - -
            - - - - - - ^ - - - -
            - - - - - - u - - - d
            - - - - - - - - - - v
            - - - - - - - - - 3 < ",
            Rules::Standard,
            Map::Standard,
        ),
    ];

    for (board_str, rules, map) in test_boards {
        let mut game = test_game();
        game.api.ruleset.name = rules;
        game.api.map = map;

        let board = Board::from_str(board_str, &game).unwrap();
        let mut food_buff = Vec::with_capacity((board.max_height * board.max_width) as usize);

        // Create permutations of all possible moves
        let mut moves_arr = Vec::new();
        let num_alive_snake_moves = Move::num_move_perm(board.num_alive_snakes() as usize);

        for mv_idx in 0..num_alive_snake_moves {
            let mut arr = Vec::new();
            let mut num_alive = 0;
            for j in 0..board.num_snakes() {
                if !board.snakes[j as usize].alive() {
                    arr.push(Move::Left);
                    continue;
                }
                let snake_mv_idx = Move::get_perm_idx(mv_idx, num_alive);
                arr.push(Move::from_idx(snake_mv_idx));

                num_alive += 1;
            }

            moves_arr.push(arr);
        }

        for moves in moves_arr {
            let mut gen_board = board.clone();
            gen_board.gen_board(&moves, &game, &mut food_buff);

            let ref_board = ref_gen.gen_ref_board(&board, &moves, &game);

            let mut msg = format!(
                "gen_board != ref_board\ngen_board:\n{:?}\nref_board:\n{:?}\n",
                gen_board, ref_board
            );
            msg = format!("{}moves: {:?}\ninput board:\n{}\n", msg, moves, board);
            assert!(gen_board == ref_board, "{}", msg);
        }
    }
}
