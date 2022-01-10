use cosmwasm_std::testing::{mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{from_binary, Coin, Extern, HandleResult, InitResponse, StdResult, Uint128};

use secret_dreamscape::contract::{handle, init, HandleMsg, InitMsg};
use secret_dreamscape::game_state::Word;
use secret_dreamscape::query::{query, CanJoinResponse, GameState, QueryMsg};

fn init_without_password() -> (
  StdResult<InitResponse>,
  Extern<MockStorage, MockApi, MockQuerier>,
) {
  let mut deps = mock_dependencies(20, &[]);
  let env = mock_env("player0", &[]);

  let init_msg = InitMsg {
    bg: 0,
    password: None,
  };

  (init(&mut deps, env, init_msg), deps)
}

fn init_with_password() -> (
  StdResult<InitResponse>,
  Extern<MockStorage, MockApi, MockQuerier>,
) {
  let mut deps = mock_dependencies(20, &[]);
  let env = mock_env("player0", &[]);

  let init_msg = InitMsg {
    bg: 0,
    password: Some("correct password".to_string()),
  };

  (init(&mut deps, env, init_msg), deps)
}

pub fn init_with_2_players(
  private: bool,
) -> (
  StdResult<InitResponse>,
  Extern<MockStorage, MockApi, MockQuerier>,
) {
  init_with_n_players(2, private)
}

pub fn init_with_4_players(
  private: bool,
) -> (
  StdResult<InitResponse>,
  Extern<MockStorage, MockApi, MockQuerier>,
) {
  init_with_n_players(4, private)
}

pub fn init_with_n_players(
  n: u64,
  private: bool,
) -> (
  StdResult<InitResponse>,
  Extern<MockStorage, MockApi, MockQuerier>,
) {
  let (init_result, mut deps) = if private {
    init_with_password()
  } else {
    init_without_password()
  };
  for i in 0..n {
    let mut player_env = mock_env(
      format!("player{}", i),
      &[Coin {
        denom: "uscrt".to_string(),
        amount: Uint128(1_000_000),
      }],
    );
    player_env.block.time = 0;
    handle(
      &mut deps,
      player_env,
      HandleMsg::Join {
        secret: i,
        password: if private { "correct password" } else { "" }.to_string(),
      },
    );
  }

  (init_result, deps)
}

pub fn get_game_state(
  deps: &mut Extern<MockStorage, MockApi, MockQuerier>,
  player: u64,
) -> GameState {
  let query_data = query(deps, QueryMsg::GetGameState { secret: player });
  from_binary(&query_data.unwrap()).unwrap()
}

pub fn get_join_permissions(
  deps: &mut Extern<MockStorage, MockApi, MockQuerier>,
) -> CanJoinResponse {
  let query_data = query(deps, QueryMsg::CanJoin {});
  from_binary(&query_data.unwrap()).unwrap()
}

fn transaction_with_money(
  deps: &mut Extern<MockStorage, MockApi, MockQuerier>,
  player: usize,
  amount: Uint128,
  msg: HandleMsg,
) -> HandleResult {
  handle(
    deps,
    mock_env(
      format!("player{}", player),
      &[Coin {
        denom: "uscrt".to_string(),
        amount,
      }],
    ),
    msg,
  )
}

pub fn send_bet(
  deps: &mut Extern<MockStorage, MockApi, MockQuerier>,
  player: usize,
  amount: Uint128,
) -> HandleResult {
  transaction_with_money(deps, player, amount, HandleMsg::Bet {})
}

pub fn match_bet(
  deps: &mut Extern<MockStorage, MockApi, MockQuerier>,
  player: usize,
  amount: Uint128,
) -> HandleResult {
  transaction_with_money(deps, player, amount, HandleMsg::Match {})
}

pub fn put_down_word(
  deps: &mut Extern<MockStorage, MockApi, MockQuerier>,
  player: usize,
  word: Vec<u8>,
) -> HandleResult {
  handle(
    deps,
    mock_env(format!("player{}", player), &[]),
    HandleMsg::PutDownCard {
      indexes: word,
      opened_dictionary: false,
    },
  )
}

pub fn request_next_turn(
  deps: &mut Extern<MockStorage, MockApi, MockQuerier>,
  player: usize,
) -> HandleResult {
  handle(
    deps,
    mock_env(format!("player{}", player), &[]),
    HandleMsg::RequestNextTurn {},
  )
}

pub fn fold(deps: &mut Extern<MockStorage, MockApi, MockQuerier>, player: usize) -> HandleResult {
  handle(
    deps,
    mock_env(format!("player{}", player), &[]),
    HandleMsg::Fold {},
  )
}
