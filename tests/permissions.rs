mod utils;

#[cfg(test)]
mod test {
  use cosmwasm_std::testing::*;
  use cosmwasm_std::{Coin, Uint128};

  use secret_dreamscape::contract::{handle, HandleMsg};

  use crate::utils::*;

  /// Test that until all words have been words put down, the player can only see their word, then they can see all of them
  #[test]
  fn player_can_only_see_their_word() {
    let (_, mut deps) = init_with_4_players(false);

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
    let game_state_player0 = get_game_state(&mut deps, 0);
    let game_state_player1 = get_game_state(&mut deps, 1);
    get_game_state(&mut deps, 2);
    get_game_state(&mut deps, 3);

    let word_according_to_player0 = game_state_player0.words[0].clone();
    let word_according_to_player1 = game_state_player1.words[0].clone();

    assert!(
      word_according_to_player0.visible,
      "Player word wasn't visible to player"
    );
    let word_obj = word_according_to_player0.word.unwrap();
    assert_eq!(word_obj.cards[0].letter, 1, "Player word wasn't correct");
    assert_eq!(word_obj.cards[1].letter, 8, "Player word wasn't correct");
    assert_eq!(word_obj.cards[2].letter, 11, "Player word wasn't correct");
    assert_eq!(word_obj.cards[3].letter, 11, "Player word wasn't correct");
    assert_eq!(word_obj.cards[4].letter, 24, "Player word wasn't correct");
    assert_eq!(word_obj.cards[5].letter, 2, "Player word wasn't correct");
    assert_eq!(word_obj.cards[6].letter, 0, "Player word wasn't correct");
    assert_eq!(word_obj.cards[7].letter, 13, "Player word wasn't correct");
    assert!(
      !word_according_to_player1.visible,
      "Player word was visible to opponent"
    );
    assert!(
      word_according_to_player1.word.is_some(),
      "Player word was visible to opponent"
    );

    put_down_word(&mut deps, 1, vec![254, 253, 252, 4, 2, 1]); // bandit: 9
    put_down_word(&mut deps, 2, vec![2, 252, 3, 254, 250, 4, 0]); // inkblot: 13
    put_down_word(&mut deps, 3, vec![254, 253, 251, 250, 4, 3, 0, 252]); // bailsman: 24

    let game_state_player1 = get_game_state(&mut deps, 1);
    assert!(
      game_state_player1.words[0].visible,
      "Opponent couldn't see word after all words got played"
    );
    assert!(
      game_state_player1.words[0].word.is_some(),
      "Opponent couldn't see word after all words got played"
    );
  }

  /// Test if a completed game can't be joined and no action can be performed
  #[test]
  fn completed_game_cant_be_joined_and_nothing_can_be_done() {
    let (_, mut deps) = init_with_2_players(false);

    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));
    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));

    // p0: y, t, g, c, l
    // p1: r, t, i, a, d
    // river: l, i, n, a, b
    put_down_word(&mut deps, 0, vec![254, 251, 250, 4, 0, 3, 253, 252]); // billycan: 15
    put_down_word(&mut deps, 1, vec![254, 253, 252, 4, 2, 1]); // bandit: 9

    request_next_turn(&mut deps, 0);

    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));
    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));

    // p0: t, g, a, a, r
    // p1: r, a, m, s, w
    // river: t, a, i, k, o

    put_down_word(&mut deps, 0, vec![0, 4, 254, 252, 253, 251]); // troika: 10
    put_down_word(&mut deps, 1, vec![250, 251, 2, 1, 0, 252, 253]); // tamarisk: 14

    request_next_turn(&mut deps, 1);

    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));
    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));

    // p0: g, a, a, s, r
    // p1: s, w, e, e, w
    // river: e, i, t, n, p
    put_down_word(&mut deps, 0, vec![0, 4, 1, 254, 251, 250, 3, 252]); // grapiest: 11
    put_down_word(&mut deps, 1, vec![1, 2, 3, 254, 251, 250, 0, 252]); // weepiest: 13

    request_next_turn(&mut deps, 1);

    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));
    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));

    // p0: a, n, n, d, e
    // p1: w, e, e, s, r
    // river: o, t, c, u, a
    put_down_word(&mut deps, 0, vec![0, 3, 253, 1, 252, 254, 251, 4]); // aduncate: 11
    put_down_word(&mut deps, 1, vec![252, 4, 254, 0, 3]); // craws: 10

    request_next_turn(&mut deps, 0);

    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));
    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));

    // p0: n, t, gold h, f, s
    // p1: e, e, o, gold x, z
    // river: i, e, e, i, e
    put_down_word(&mut deps, 0, vec![3, 250, 4, 2, 0, 251, 1]); // fishnet: 26
    put_down_word(&mut deps, 1, vec![2, 3]); // ox: 18

    request_next_turn(&mut deps, 0);

    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));
    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));

    // p0: i, l, g, f, d
    // p1: e, e, z, m, o
    // river: g, u, o, gold d, e
    put_down_word(&mut deps, 0, vec![3, 1, 252, 250, 2, 254, 253]); // flogged: 26
    put_down_word(&mut deps, 1, vec![2, 0, 253]); // zed: 22

    request_next_turn(&mut deps, 0);

    let final_bet = send_bet(&mut deps, 0, Uint128(1_000_000));
    let mut p2 = mock_env(
      "player2".to_string(),
      &[Coin {
        denom: "uscrt".to_string(),
        amount: Uint128(1_000_000),
      }],
    );
    p2.block.time = 0;
    let join_attempt = handle(
      &mut deps,
      p2,
      HandleMsg::Join {
        nfts: vec![],
        secret: 2,
        password: "".to_string(),
      },
    );
    assert!(final_bet.is_err(), "Final bet did not fail");
    assert!(
      join_attempt.is_err(),
      "Player was able to join a completed game"
    );
  }

  /// Test if a full game can't be joined
  #[test]
  fn full_game_cant_be_joined() {
    let (_, mut deps) = init_with_4_players(false);
    let mut p4 = mock_env(
      "player4".to_string(),
      &[Coin {
        denom: "uscrt".to_string(),
        amount: Uint128(1_000_000),
      }],
    );
    p4.block.time = 0;
    let join_attempt = handle(
      &mut deps,
      p4,
      HandleMsg::Join {
        nfts: vec![],
        secret: 4,
        password: "".to_string(),
      },
    );
    assert!(join_attempt.is_err(), "Player was able to join a full game");
  }

  /// Test if the player can't see an opponent's played word
  #[test]
  fn player_cant_see_opponent_word() {
    let (_, mut deps) = init_with_2_players(false);

    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));
    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));

    // p0: y, t, g, c, l
    // p1: r, t, i, a, d
    // river: l, i, n, a, b
    put_down_word(&mut deps, 0, vec![254, 251, 250, 4, 0, 3, 253, 252]); // billycan: 15
    let p1 = get_game_state(&mut deps, 1);

    assert!(!p1.words[0].visible, "Player could see opponent's word");
    assert!(
      p1.words[0].word.is_some(),
      "Player could see opponent's word"
    );
  }

  /// Test if private rooms are reported as such
  #[test]
  fn private_rooms_are_reported_as_private() {
    let (_, mut deps) = init_with_2_players(true);

    let data = get_join_permissions(&mut deps);

    assert!(
      data.requires_password,
      "Private rooms are not reported as private"
    );
  }

  /// Test if in order to join a private room you need to enter the correct password
  #[test]
  fn correct_password_needed_to_join_private_room() {
    let (_, mut deps) = init_with_2_players(true);
    let mut p2 = mock_env(
      "player2".to_string(),
      &[Coin {
        denom: "uscrt".to_string(),
        amount: Uint128(1_000_000),
      }],
    );
    p2.block.time = 0;
    let join_attempt = handle(
      &mut deps,
      p2.clone(),
      HandleMsg::Join {
        nfts: vec![],
        secret: 4,
        password: "wrong password".to_string(),
      },
    );
    assert!(
      join_attempt.is_err(),
      "Player was able to join a private game with the wrong password"
    );
    let join_attempt = handle(
      &mut deps,
      p2.clone(),
      HandleMsg::Join {
        nfts: vec![],
        secret: 4,
        password: "correct password".to_string(),
      },
    );
    assert!(
      join_attempt.is_ok(),
      "Player was NOT able to join a private game with the correct password"
    );
  }
}
