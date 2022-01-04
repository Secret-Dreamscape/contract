mod utils;

#[cfg(test)]
mod test {
  use cosmwasm_std::testing::*;
  use cosmwasm_std::{from_binary, Coin, Extern, HandleResult, InitResponse, StdResult, Uint128};

  use secret_dreamscape::contract::{handle, init, HandleMsg, InitMsg};
  use secret_dreamscape::game_state::GameRound;
  use secret_dreamscape::query::{query, GameState, QueryMsg};

  use crate::utils::*;

  /// Test if betting increases the pot and the player's bet
  #[test]
  fn bet_increases_pot() {
    let (_, mut deps) = init_with_4_players();

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
    let (_, mut deps) = init_with_4_players();

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
    let (_, mut deps) = init_with_4_players();

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
    let (_, mut deps) = init_with_4_players();

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
    let (_, mut deps) = init_with_4_players();

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
}
