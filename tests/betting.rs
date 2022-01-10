mod utils;

#[cfg(test)]
mod test {
  use cosmwasm_std::testing::*;
  use cosmwasm_std::{
    from_binary, BankMsg, Coin, CosmosMsg, Extern, HandleResult, HumanAddr, InitResponse,
    StdResult, Uint128,
  };

  use secret_dreamscape::contract::{handle, init, HandleMsg, InitMsg};
  use secret_dreamscape::game_state::{GameRound, PlayerAction};
  use secret_dreamscape::query::{query, GameState, QueryMsg};

  use crate::utils::*;

  /// Test if betting increases the pot and the player's bet
  #[test]
  fn bet_increases_pot() {
    let (_, mut deps) = init_with_4_players(false);

    let game = get_game_state(&mut deps, 0);

    assert_eq!(game.pool, 0, "Pool is non zero");

    send_bet(&mut deps, 0, Uint128(1_000_000));

    let game = get_game_state(&mut deps, 0);

    assert_eq!(
      game.pool, 1_000_000,
      "The pool didn't change after sending some secret"
    );

    send_bet(&mut deps, 1, Uint128(2_000_000));

    let game = get_game_state(&mut deps, 0);

    assert_eq!(
      game.pool, 3_000_000,
      "The pool didn't change after sending some secret"
    );
  }

  /// Test if after all players set a bet, the matching phase starts
  #[test]
  fn proceeding_to_matching_phase() {
    let (_, mut deps) = init_with_4_players(false);

    let game = get_game_state(&mut deps, 0);

    assert!(game.round == GameRound::Blind, "Round is not blind");

    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(2_000_000));
    send_bet(&mut deps, 2, Uint128(3_000_000));
    send_bet(&mut deps, 3, Uint128(4_000_000));

    let game = get_game_state(&mut deps, 0);

    assert!(
      game.round == GameRound::Matching,
      "Game round is not matching"
    );
  }

  /// Test if when all players bet the same amount, the matching phase is skipped
  #[test]
  fn proceeding_to_flop_phase_from_blind() {
    let (_, mut deps) = init_with_4_players(false);

    let game = get_game_state(&mut deps, 0);

    assert!(game.round == GameRound::Blind, "Round is not blind");

    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));
    send_bet(&mut deps, 2, Uint128(1_000_000));
    send_bet(&mut deps, 3, Uint128(1_000_000));

    let game = get_game_state(&mut deps, 0);

    assert!(game.round == GameRound::Flop, "Game round is not flop");
  }

  /// Test if after all players match the highest bet, the flop phase starts
  #[test]
  fn proceeding_to_flot_phase_from_matching() {
    let (_, mut deps) = init_with_4_players(false);

    let game = get_game_state(&mut deps, 0);

    assert!(game.round == GameRound::Blind, "Round is not blind");

    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(2_000_000));
    send_bet(&mut deps, 2, Uint128(3_000_000));
    send_bet(&mut deps, 3, Uint128(4_000_000));

    let game = get_game_state(&mut deps, 0);

    assert!(
      game.round == GameRound::Matching,
      "Game round is not matching"
    );

    match_bet(&mut deps, 0, Uint128(3_000_000));
    match_bet(&mut deps, 1, Uint128(2_000_000));
    match_bet(&mut deps, 2, Uint128(1_000_000));

    let game = get_game_state(&mut deps, 0);
    assert!(game.round == GameRound::Flop, "Game round is not flop");
  }

  /// Test if a winner for the turn is determined when all but one player folds
  #[test]
  fn end_turn_only_one_active_player() {
    let (_, mut deps) = init_with_4_players(false);

    let game = get_game_state(&mut deps, 0);

    assert!(game.round == GameRound::Blind, "Round is not blind");

    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(2_000_000));
    send_bet(&mut deps, 2, Uint128(3_000_000));
    send_bet(&mut deps, 3, Uint128(4_000_000));

    let game = get_game_state(&mut deps, 0);

    assert!(
      game.round == GameRound::Matching,
      "Game round is not matching"
    );

    fold(&mut deps, 0);
    fold(&mut deps, 1);
    fold(&mut deps, 2);

    let game = get_game_state(&mut deps, 0);
    assert!(game.winner.is_some(), "Winner is not set");
  }

  /// Test if a player can't bet if they've folded
  #[test]
  fn player_cant_bet_if_folded() {
    let (_, mut deps) = init_with_4_players(false);

    fold(&mut deps, 0);

    let bet = send_bet(&mut deps, 0, Uint128(1_000_000));
    assert!(bet.is_err(), "Player can bet after folding");
  }

  /// Test if a player can't send a bet smaller than 1 SCRT
  #[test]
  fn cant_send_small_bet() {
    let (_, mut deps) = init_with_4_players(false);

    let bet = send_bet(&mut deps, 0, Uint128(1));
    assert!(bet.is_err(), "Player can send a bet smaller than 1 SCRT");
  }

  /// Test that betting isn't possible during any phase other than the blind phase and the flop phase
  #[test]
  fn cant_bet_when_not_in_blind_or_flop_phase() {
    let (_, mut deps) = init_with_4_players(false);

    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(2_000_000));
    send_bet(&mut deps, 2, Uint128(3_000_000));
    send_bet(&mut deps, 3, Uint128(4_000_000));

    let game = get_game_state(&mut deps, 0);
    assert!(
      game.round == GameRound::Matching,
      "Game round is not matching"
    );

    let bet = send_bet(&mut deps, 0, Uint128(1_000_000));
    assert!(
      bet.is_err(),
      "Player can bet outside of the blind or flop phase"
    );

    match_bet(&mut deps, 0, Uint128(3_000_000));
    match_bet(&mut deps, 1, Uint128(2_000_000));
    match_bet(&mut deps, 2, Uint128(1_000_000));

    let game = get_game_state(&mut deps, 0);
    assert!(game.round == GameRound::Flop, "Game round is not flop");

    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(2_000_000));
    send_bet(&mut deps, 2, Uint128(3_000_000));
    send_bet(&mut deps, 3, Uint128(4_000_000));

    let game = get_game_state(&mut deps, 0);
    assert!(
      game.round == GameRound::Matching2,
      "Game round is not matching2"
    );

    let bet = send_bet(&mut deps, 0, Uint128(1_000_000));
    assert!(
      bet.is_err(),
      "Player can bet outside of the blind or flop phase"
    );
  }

  /// Test that matching a bet only works when sending the exact amount needed
  #[test]
  fn matching_bet_only_works_with_exact_amount() {
    let (_, mut deps) = init_with_4_players(false);

    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(2_000_000));
    send_bet(&mut deps, 2, Uint128(3_000_000));
    send_bet(&mut deps, 3, Uint128(4_000_000));

    let game = get_game_state(&mut deps, 0);
    let bet = match_bet(&mut deps, 0, Uint128(2_000_000));
    assert!(
      bet.is_err(),
      "Player can match a bet with a different amount"
    );
  }

  /// Test that when requesting the next turn the pool is cleared, no word is played, the river is emptied, the winner is reset, all players' actions and other states reset, the turn increases by 1 and the phase changes to blind
  #[test]
  fn request_next_turn_works_correctly() {
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

    let new_state = get_game_state(&mut deps, 0);
    assert_eq!(new_state.pool, 0, "Pool is not empty");
    assert_eq!(new_state.words.len(), 0, "Words are not empty");
    assert!(new_state.river.is_none(), "River is not empty");
    assert!(new_state.winner.is_none(), "Winner is not reset");
    assert!(
      new_state.players[0].last_action.is_none(),
      "Player 0 action is not reset"
    );
    assert!(
      new_state.players[1].last_action.is_none(),
      "Player 1 action is not reset"
    );
    assert_eq!(new_state.turn, 1, "Turn is not incremented");
    assert!(new_state.round == GameRound::Blind, "Round is not blind");
  }

  /// Test if players can only see the river during or after the flop phase
  #[test]
  fn river_can_only_be_seen_during_or_after_flop() {
    let (_, mut deps) = init_with_2_players(false);

    let game = get_game_state(&mut deps, 0);
    assert!(game.river.is_none(), "River is not empty during bet state");

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

    let game = get_game_state(&mut deps, 0);
    assert!(
      game.river.is_none(),
      "River is not empty after turn changes"
    );
  }

  /// Test if there's a tie for first place, split the pot between all tying players
  #[test]
  fn tie_for_first_place_split_pots() {
    let (_, mut deps) = init_with_2_players(false);

    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));
    send_bet(&mut deps, 0, Uint128(1_000_000));
    send_bet(&mut deps, 1, Uint128(1_000_000));

    // p0: y, t, g, c, l
    // p1: r, t, i, a, d
    // river: l, i, n, a, b
    put_down_word(&mut deps, 0, vec![0]); // y: 0
    let final_step = (put_down_word(&mut deps, 1, vec![0])).unwrap(); // r: 0

    let first_transfer_message = final_step.clone().messages[0].clone();
    let second_transfer_message = final_step.messages[1].clone();

    match (first_transfer_message, second_transfer_message) {
      (CosmosMsg::Bank(msg1), CosmosMsg::Bank(msg2)) => match (msg1, msg2) {
        (
          BankMsg::Send {
            from_address: from_address1,
            to_address: to_address1,
            amount: amount1,
          },
          BankMsg::Send {
            from_address: from_address2,
            to_address: to_address2,
            amount: amount2,
          },
        ) => {
          assert_eq!(
            to_address1,
            HumanAddr("player0".to_string()),
            "First Winner did not collect the pot"
          );
          assert_eq!(
            amount1[0],
            Coin {
              amount: Uint128(2_000_000),
              denom: "uscrt".to_string(),
            }
          );
          assert_eq!(
            to_address2,
            HumanAddr("player1".to_string()),
            "Second Winner did not collect the pot"
          );
          assert_eq!(
            amount2[0],
            Coin {
              amount: Uint128(2_000_000),
              denom: "uscrt".to_string(),
            }
          );
        }
      },
      _ => panic!("Expected bank transfer message"),
    }
  }
}
