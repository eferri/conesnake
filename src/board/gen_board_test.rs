use crate::board::Board;
use crate::game::{Map, Rules};
use crate::rand::{MaxRand, Rand};
use crate::tests::{common::test_game, ref_move::RefMove};
use crate::util::Move;

use pretty_assertions::assert_eq;

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
            - - ^ < < a
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
            - 0 e - - -
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
            - - ^ < a -
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
            - ^ a - - -",
            Rules::Solo,
            Map::Standard,
        ),
        (
            // Head
            "turn: 12 health: 93 health: 53 health: 29
            - - - - - -
            - - b > > 1
            2 - 0 - - -
            ^ - ^ - - -
            ^ - ^ < < a
            c - - - - -",
            Rules::Standard,
            Map::Standard,
        ),
        (
            // Head 2
            "turn: 12 health: 93 health: 53 health: 29
            - - - - - -
            - - b > > 1
            0 - 2 - - -
            ^ - ^ - - -
            ^ - ^ < < a
            c - - - - -",
            Rules::Standard,
            Map::Standard,
        ),
        (
            // Head 3
            "turn: 12 health: 93 health: 53 health: 29
            - - - - - -
            - - b > > 1
            0 - 2 - - -
            ^ - ^ - - -
            ^ - ^ < a -
            ^ a - - - -",
            Rules::Standard,
            Map::Standard,
        ),
        (
            // Food
            "turn: 13 health: 72 health: 53 health: 9
            - - - - - -
            - b > > > 1
            2 - - - - +
            ^ - b > v 0
            ^ - - - > ^
            c - - - - -",
            Rules::Standard,
            Map::Standard,
        ),
        (
            // Food 2
            "turn: 13 health: 72 health: 53 health: 9
            - - - - - -
            - b > > > 1
            2 - - - - +
            ^ - - b v 0
            ^ - - - > ^
            c - - - - -",
            Rules::Standard,
            Map::Standard,
        ),
        (
            // Tail
            "turn: 10 health: 45 health: 42 health: 42
            - - - - - -
            - - b > > 1
            v E 0 - - -
            v - ^ - - -
            v - ^ < < a
            2 - - - - -",
            Rules::Standard,
            Map::Standard,
        ),
        (
            // Challenge
            "turn: 1 health: 100 health: 100 health: 100
            - - - - - - -
            - 1 b v - - -
            - ^ - v - - -
            - ^ < < - - -
            - - - - 2 < <
            b > 0 - d - ^
            - - - - > > ^",
            Rules::Standard,
            Map::Standard,
        ),
        (
            // Simultaneous body
            "turn: 1 health: 45 health: 43
            b > v - - - -
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
            - - - - - - - v < a 4
            - > > 1 b v - v - - ^
            - ^ - - - v - v - - ^
            - ^ - - - v - v - - ^
            - ^ < < < < - > > > ^
            - - - - v e - - - - -
            3 < < < v > > > > v -
            d - - ^ 0 ^ - - - v -
            v - - ^ - ^ - - - v -
            v - - ^ - ^ < < a 2 -
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
            * 0 < a - - - - - - - - - - - - - - *
            * - * * - * - * * * * * - * - * * - *
            * - - - - * - - - * - - - * - - - - *
            * - - * - * * * - * - * * * - * - - *
            * - - * - * - - - - - - - * - * - - *
            * * * * - * - * - * - * - * - * * * *
            - - - - v a - * - - - * - - - - - - -
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
            b ^ - - - 0 < - - - -
            - - - - - - ^ - - - -
            - - - - - - c - - - d
            - - - - - - - - - - v
            - - - - - - - - - 3 < ",
            Rules::Standard,
            Map::Standard,
        ),
        (
            "turn: 232 health: 81 health: 45 health: 58 health: 42
            - - + > > > > v - - -
            - - - ^ < < a 3 - b v
            - d - + - - - - - - v
            - v - - - - - 0 < < v
            - > v - - v < < < ^ <
            - - > v v < - b ^ - -
            - - 1 v v - 2 - - - -
            - - ^ v > > ^ - - - -
            - - ^ < - - - - - - -
            - - - - - - - - - - -
            - - - - - - - - - - -",
            Rules::Standard,
            Map::Standard,
        ),
        (
            "turn: 99 health: 95 health: 84 health: 89 health: 95
            * * * * * * * * * * *
            * * * * * * * * * * @
            * * S { u E * * * * *
            * * * n u * * * T * *
            - - - ^ < - - - ^ - -
            - - 2 < < < < - ^ - -
            - + b > > > ^ - ^ - -
            - - - - - - - - c - -
            > > > > 3 - - - - - b
            - - - - - - - - - - -",
            Rules::Wrapped,
            Map::Royale,
        ),
        (
            "turn: 93 health: 95 health: 84
            * * * * * * * * * * *
            * * * u { * * * * * *
            * @ S { n A * * * * *
            * * * * * * * * * * *
            - - - - - - - - - - -
            - - - - - - - - + - -
            - + - - - - - - - - -
            - - - - - - - - - - -
            > > > > 1 - - - - - f
            - - - - - - - - - - -",
            Rules::Wrapped,
            Map::Royale,
        ),
        (
            "turn: 10 health: 90 health: 90
            > > 0 - - - - - - - -
            ^ - - - - - - - - - -
            ^ - - - - - - 1 < - -
            ^ - - - - - - - ^ - -
            ^ < - - - - - - ^ - -
            b ^ - - - - > > ^ - -
            - - - - - - ^ < a - -
            - - - - - - - - - - -
            - - - - - - - - - - -
            - - - - - - - - - - -
            - - - - - - - - - - - ",
            Rules::Constrictor,
            Map::Standard,
        ),
    ];

    for (board_str, rules, map) in test_boards {
        let mut game = test_game();
        game.ruleset = rules;
        game.api.map = map;

        let mut rng = MaxRand::new();

        // Remove extra quotes from serde output
        let mut rules_str = serde_json::to_string(&rules).unwrap();
        rules_str = rules_str[1..rules_str.len() - 1].to_owned();
        game.api.ruleset.name = rules_str;

        let board = Board::from_str(board_str, &game).unwrap();
        let mut food_buff = Vec::with_capacity((board.width * board.height) as usize);

        if let Map::Royale = game.api.map {
            game.api.ruleset.settings.hazard_damage_per_turn = 16;
        }

        // Create permutations of all possible moves
        let mut moves_arr = Vec::new();
        let num_alive_snake_moves = Move::num_perm(board.num_alive_snakes());

        for mv_idx in 0..num_alive_snake_moves {
            let mut arr = Vec::new();
            let mut num_alive = 0;
            for j in 0..board.num_snakes() {
                if !board.snakes[j as usize].alive() {
                    arr.push(Move::Left);
                    continue;
                }
                arr.push(Move::extract(mv_idx, num_alive));

                num_alive += 1;
            }

            moves_arr.push(arr);
        }

        for moves in moves_arr {
            let mut gen_board = board.clone();
            gen_board.gen_board(Move::encode(&moves), &game, &mut food_buff, &mut rng);
            let ref_board = ref_gen.gen_ref_board(&game, &board, &moves);

            if gen_board != ref_board {
                println!("\nmoves: {moves:?}\ninput board:\n{board}");
                println!("gen_board:\n{gen_board}\nref_board:\n{ref_board}\n-----");
            }
            assert_eq!(gen_board, ref_board);
        }
    }
}
