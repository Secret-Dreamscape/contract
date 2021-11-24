use cosmwasm_std::{Api, Binary, Env, Extern, HumanAddr, Querier, QueryResult, StdError, Storage};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json_wasm as serde_json;

use crate::contract::get_requesting_player;
use crate::game_state::{Card, Player, State};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
  GetResult {},
  GetBets {},
  GetBoard { secret: u64 },
  GetHand { secret: u64 },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
struct Result {
  winner: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
struct BetsResult {
  addr: HumanAddr,
  hp: u8,
  deposit: u64,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
struct HandResult {
  hand: Vec<Card>,
}

pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
  match msg {
    QueryMsg::GetResult {} => {
      let state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();

      if state.winner.is_none() {
        return Err(StdError::generic_err("Still waiting for players."));
      }

      return Ok(Binary(serde_json::to_vec(&vec![false]).unwrap()));
    }
    QueryMsg::GetBets {} => {
      let state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();
      let player1 = state.player1.clone().unwrap();
      let player2 = state.player2.clone().unwrap();

      return Ok(Binary(
        serde_json::to_vec(&vec![
          BetsResult {
            addr: player1.addr,
            hp: player1.hp,
            deposit: player1.deposit,
          },
          BetsResult {
            addr: player2.addr,
            hp: player2.hp,
            deposit: player2.deposit,
          },
        ])
        .unwrap(),
      ));
    }
    QueryMsg::GetHand { secret } => {
      let state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();
      if state.player1.is_none() || state.player2.is_none() {
        return Err(StdError::unauthorized());
      }

      let player1 = state.player1.clone().unwrap();
      let player2 = state.player2.clone().unwrap();

      let account_with_secret = match (player1.secret == secret, player2.secret == secret) {
        (false, false) => None,
        (true, false) => Some(player1),
        (false, true) => Some(player2),
        _ => None,
      };

      if account_with_secret.is_none() {
        return Err(StdError::unauthorized());
      }

      let hand: Vec<Card> = (&account_with_secret.unwrap().deck[0..4]).to_vec();

      return Ok(Binary(serde_json::to_vec(&HandResult { hand }).unwrap()));
    }
    QueryMsg::GetBoard { secret } => {
      let state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();

      if state.player1.is_none() || state.player2.is_none() {
        return Err(StdError::unauthorized());
      }

      let player1 = state.player1.clone().unwrap();
      let player2 = state.player2.clone().unwrap();

      let request_attempt = match (player1.secret == secret, player2.secret == secret) {
        (false, false) => None,
        (true, false) => Some(player1.clone()),
        (false, true) => Some(player2.clone()),
        _ => None,
      };
      if request_attempt.is_none() {
        return Err(StdError::unauthorized());
      }
      let requester: Player = request_attempt.unwrap();
      let direction: bool = state.game_board.direction;
      return match state.game_board.cards {
        (None, None) => show_game_board((None, None), direction, None),
        (Some(card), None) => {
          if requester.addr != player1.clone().addr {
            return show_game_board(
              (
                Some(CardView {
                  card: None,
                  visible: false,
                }),
                None,
              ),
              direction,
              None,
            );
          }
          show_game_board(
            (
              Some(CardView {
                card: Some(card),
                visible: true,
              }),
              None,
            ),
            direction,
            None,
          )
        }
        (None, Some(card)) => {
          if requester.addr != player2.clone().addr {
            return show_game_board(
              (
                None,
                Some(CardView {
                  card: None,
                  visible: false,
                }),
              ),
              direction,
              None,
            );
          }
          show_game_board(
            (
              None,
              Some(CardView {
                card: Some(card),
                visible: true,
              }),
            ),
            direction,
            None,
          )
        }
        (Some(ref card1), Some(ref card2)) => show_game_board(
          (
            Some(CardView {
              card: Some(card1.clone()),
              visible: true,
            }),
            Some(CardView {
              card: Some(card2.clone()),
              visible: true,
            }),
          ),
          direction,
          state.game_board.winner_for_turn,
        ),
      };
    }
  }
}

fn show_game_board(
  cards: (Option<CardView>, Option<CardView>),
  direction: bool,
  winner: Option<HumanAddr>,
) -> QueryResult {
  return Ok(Binary(
    serde_json::to_vec(&GameBoardView {
      cards,
      direction,
      winner,
    })
    .unwrap(),
  ));
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
struct GameBoardView {
  cards: (Option<CardView>, Option<CardView>),
  direction: bool,
  winner: Option<HumanAddr>,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
struct CardView {
  card: Option<Card>,
  visible: bool,
}
