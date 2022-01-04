mod utils;

#[cfg(test)]
mod test {
  use cosmwasm_std::testing::*;
  use cosmwasm_std::{
    from_binary, Coin, Extern, HandleResult, HumanAddr, InitResponse, StdResult, Uint128,
  };

  use secret_dreamscape::contract::{handle, init, HandleMsg, InitMsg};
  use secret_dreamscape::game_state::{Card, GameRound, Word};
  use secret_dreamscape::query::{query, GameState, QueryMsg};
  use secret_dreamscape::utils::cards::get_score_for_word;

  use crate::utils::*;

  /// Test if an invalid word has 0 points
  #[test]
  fn invalid_word_has_no_points() {
    let word = vec![
      Card {
        letter: 0,
        gold: false,
      },
      Card {
        letter: 1,
        gold: false,
      },
      Card {
        letter: 2,
        gold: false,
      },
      Card {
        letter: 3,
        gold: false,
      },
      Card {
        letter: 4,
        gold: false,
      },
    ];
    let score = get_score_for_word(word.as_slice());
    assert_eq!(score, 0, "invalid word should have 0 points");
  }

  /// Test if score doubles for each gold card
  #[test]
  fn gold_doubles_score() {
    // test
    let mut word = vec![
      Card {
        letter: 19,
        gold: false,
      },
      Card {
        letter: 4,
        gold: false,
      },
      Card {
        letter: 18,
        gold: false,
      },
      Card {
        letter: 19,
        gold: false,
      },
    ];
    let score = get_score_for_word(word.as_slice());
    assert_eq!(
      score, 4,
      "a score with no gold should be the sum of the points for each card"
    );
    word[0].gold = true;
    let score = get_score_for_word(word.as_slice());
    assert_eq!(
      score, 8,
      "a word with one gold card should double the score"
    );
    word[1].gold = true;
    let score = get_score_for_word(word.as_slice());
    assert_eq!(
      score, 16,
      "a word with two gold cards should double the score twice"
    );
    word[2].gold = true;
    let score = get_score_for_word(word.as_slice());
    assert_eq!(
      score, 32,
      "a word with two gold cards should double the score three times"
    );
    word[3].gold = true;
    let score = get_score_for_word(word.as_slice());
    assert_eq!(
      score, 64,
      "a word with two gold cards should double the score four times"
    );
  }

  /// Test if the highest scoring word wins
  #[test]
  fn highest_score_wins() {
    let (_, mut deps) = init_with_4_players();

    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));
    send_bet(&mut deps, 2, Uint128(1_000_000));
    send_bet(&mut deps, 3, Uint128(1_000_000));
    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));
    send_bet(&mut deps, 2, Uint128(1_000_000));
    send_bet(&mut deps, 3, Uint128(1_000_000));

    // letters on player1's hands are: y, t, g, c, l
    // letters on player2's hands are: r, t, i, a, d
    // letters on player3's hands are: t, a, i, k, o
    // letters on player4's hands are: a, a, r, m gold, s
    // letters on river are: l, i, n, a, b
    put_down_word(&mut deps, 0, vec![254, 251, 250, 4, 0, 3, 253, 252]); // billycan: 15
    put_down_word(&mut deps, 1, vec![254, 253, 252, 4, 2, 1]); // bandit: 9
    put_down_word(&mut deps, 2, vec![2, 252, 3, 254, 250, 4, 0]); // inkblot: 13
    put_down_word(&mut deps, 3, vec![254, 253, 251, 250, 4, 3, 0, 252]); // bailsman: 24

    let game = get_game_state(&mut deps, 0);

    assert_eq!(
      game.winner,
      Some(HumanAddr("player3".to_string())),
      "Winner is not the highest word"
    );
  }

  /// Test if putting down the same card twice results in an error
  #[test]
  fn putting_down_card_twice_results_in_err() {
    let (_, mut deps) = init_with_4_players();

    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));
    send_bet(&mut deps, 2, Uint128(1_000_000));
    send_bet(&mut deps, 3, Uint128(1_000_000));
    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));
    send_bet(&mut deps, 2, Uint128(1_000_000));
    send_bet(&mut deps, 3, Uint128(1_000_000));

    let word = put_down_word(&mut deps, 0, vec![0, 0]);
    assert!(
      word.is_err(),
      "putting down the same card twice should result in an error",
    );
  }

  /// Test if putting down an invalid word results in an error
  #[test]
  fn putting_down_invalid_word_results_in_err() {
    let (_, mut deps) = init_with_4_players();

    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));
    send_bet(&mut deps, 2, Uint128(1_000_000));
    send_bet(&mut deps, 3, Uint128(1_000_000));
    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));
    send_bet(&mut deps, 2, Uint128(1_000_000));
    send_bet(&mut deps, 3, Uint128(1_000_000));

    let word = put_down_word(&mut deps, 0, vec![6]);
    assert!(
      word.is_err(),
      "putting down the same card twice should result in an error",
    );

    let word = put_down_word(&mut deps, 0, vec![123]);
    assert!(
      word.is_err(),
      "putting down the same card twice should result in an error",
    );
  }
}
