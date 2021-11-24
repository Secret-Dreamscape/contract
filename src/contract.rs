use cosmwasm_std::{
  Api, BankMsg, Coin, CosmosMsg, Env, Extern, HandleResponse, HandleResult, InitResponse,
  InitResult, Querier, StdError, StdResult, Storage, Uint128,
};
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaChaRng;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json_wasm as serde_json;
use sha2::{Digest, Sha256};

use crate::game_state::{Card, GameBoard, Player, State};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {}

pub fn init<S: Storage, A: Api, Q: Querier>(
  deps: &mut Extern<S, A, Q>,
  _env: Env,
  _msg: InitMsg,
) -> InitResult {
  let state = State {
    player1: None,
    player2: None,
    winner: None,
    game_board: GameBoard {
      turn_ended: false,
      winner_for_turn: None,
      direction: false,
      cards: (None, None),
      pool: 0,
      turn: 0,
    },
  };

  deps
    .storage
    .set(b"state", &serde_json::to_vec(&state).unwrap());

  Ok(InitResponse::default())
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
  Join { secret: u64 },
  Deposit {},
  Bet { amount: u64 },
  PutDownCard { index: u64 },
  RequestNextTurn { direction: bool },
}

fn generate_deck(mut rng: ChaChaRng) -> Vec<Card> {
  let deck_size: u8 = 40;
  let mut deck: Vec<Card> = vec![];
  for _i in 0..deck_size {
    deck.push(Card {
      value: (rng.next_u32() % 26) as u8,
      gold: (rng.next_u32() % 2) == 0,
    })
  }
  deck
}

fn get_rng(state: &State, env: &Env) -> ChaChaRng {
  let mut combined_secret: Vec<u8> = env.block.time.to_be_bytes().to_vec();
  if state.player1.is_some() {
    combined_secret.extend(&state.player1.as_ref().unwrap().secret.to_be_bytes());
  }
  if state.player2.is_some() {
    combined_secret.extend(&state.player2.as_ref().unwrap().secret.to_be_bytes());
  }
  let random_seed: [u8; 32] = Sha256::digest(&combined_secret).into();
  ChaChaRng::from_seed(random_seed)
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
  deps: &mut Extern<S, A, Q>,
  env: Env,
  msg: HandleMsg,
) -> HandleResult {
  match msg {
    HandleMsg::Join { secret } => {
      if env.message.sent_funds.len() != 1
        || env.message.sent_funds[0].amount != Uint128(1_000_000)
        || env.message.sent_funds[0].denom != *"uscrt"
      {
        return Err(StdError::generic_err(
          "Must deposit 1 SCRT to enter the game.",
        ));
      }

      let mut state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();

      if state.player1.is_none() {
        state.player1 = Some(Player {
          addr: env.message.sender.clone(),
          secret,
          deck: generate_deck(get_rng(&state, &env)),
          hp: 5,
          deposit: 1_000_000,
        });

        deps
          .storage
          .set(b"state", &serde_json::to_vec(&state).unwrap());
        Ok(HandleResponse::default())
      } else if state.player2.is_none() {
        if env.message.sender == state.player1.clone().unwrap().addr {
          return Err(StdError::generic_err(
            "You are already in the game. You can't be both player 1 and player 2.",
          ));
        }
        state.player2 = Some(Player {
          addr: env.message.sender.clone(),
          secret,
          deck: generate_deck(get_rng(&state, &env)),
          hp: 5,
          deposit: 1_000_000,
        });

        deps
          .storage
          .set(b"state", &serde_json::to_vec(&state).unwrap());
        Ok(HandleResponse::default())
      } else {
        Err(StdError::generic_err("Game is full."))
      }
    }
    HandleMsg::PutDownCard { index } => {
      if index > 4 {
        return Err(StdError::generic_err(
          "You cannot place a card that's not in your hand",
        ));
      }
      let mut state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();
      require_both_players(&mut state)?;
      let mut player1 = state.player1.clone().unwrap();
      let mut player2 = state.player2.clone().unwrap();
      let request_attempt = get_requesting_player(&deps, env);
      if request_attempt.is_none() || state.game_board.turn_ended {
        return Err(StdError::unauthorized());
      }
      let requester: Player = request_attempt.unwrap();
      let card: Card = requester.deck[index as usize].clone();
      if requester.addr == state.player1.as_ref().unwrap().addr {
        match state.game_board.cards {
          (None, None) => state.game_board.cards = (Some(card), None),
          (Some(_), None) => {
            return Err(StdError::generic_err(
              "You already put down a card for this turn. Please wait for your opponent",
            ));
          }
          (None, Some(player2_card)) => state.game_board.cards = (Some(card), Some(player2_card)),
          (Some(_), Some(_)) => {
            return Err(StdError::generic_err(
              "Both players already put down a card for this turn",
            ));
          }
        }
      } else if requester.addr == state.player2.as_ref().unwrap().addr {
        match state.game_board.cards {
          (None, None) => state.game_board.cards = (None, Some(card)),
          (None, Some(_)) => {
            return Err(StdError::generic_err(
              "You already put down a card for this turn. Please wait for your opponent",
            ));
          }
          (Some(player1_card), None) => state.game_board.cards = (Some(player1_card), Some(card)),
          (Some(_), Some(_)) => {
            return Err(StdError::generic_err(
              "Both players already put down a card for this turn",
            ));
          }
        }
      }
      if let (Some(player1_card), Some(player2_card)) = state.game_board.cards.clone() {
        if !state.game_board.turn_ended {
          let value_winner = get_winner_for_cards(
            &player1,
            &player2,
            state.game_board.direction,
            &player1_card,
            &player2_card,
          );
          let winner = match (player1_card.gold, player2_card.gold) {
            (false, false) => value_winner,
            (true, false) => Some(player1.clone()),
            (false, true) => Some(player2.clone()),
            (true, true) => value_winner,
          };

          state.game_board.turn_ended = true;

          match winner {
            None => {
              state.game_board.direction = !state.game_board.direction;
              state.game_board.cards = (None, None);
            }
            Some(winner) => {
              let mut newplayer1 = player1.clone();
              let mut newplayer2 = player2.clone();
              if winner.addr == player1.addr {
                newplayer2.hp -= 1;
                newplayer1.deposit += state.game_board.pool;
              } else {
                newplayer1.hp -= 1;
                newplayer2.deposit += state.game_board.pool;
              }
              state.game_board.pool = 0;
              player1 = newplayer1.clone();
              player2 = newplayer2.clone();
              state.player1 = Some(newplayer1);
              state.player2 = Some(newplayer2);
              state.game_board.winner_for_turn = Some(winner.addr);
            }
          }
        }
      }
      if requester.addr == state.player1.clone().unwrap().addr {
        player1.deck.remove(index as usize);
        state.player1 = Some(player1.clone());
      } else {
        player2.deck.remove(index as usize);
        state.player2 = Some(player2.clone());
      }
      deps
        .storage
        .set(b"state", &serde_json::to_vec(&state).unwrap());
      Ok(HandleResponse::default())
    }
    HandleMsg::Deposit {} => {
      let mut state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();
      if env.message.sent_funds.len() != 1
        || env.message.sent_funds[0].amount < Uint128(1_000_000)
        || env.message.sent_funds[0].denom != *"uscrt"
      {
        return Err(StdError::generic_err("Deposit at least 1 SCRT."));
      }
      let amount = env.message.sent_funds[0].amount;
      require_both_players(&mut state)?;
      let request_attempt = get_requesting_player(&deps, env);
      if request_attempt.is_none() {
        return Err(StdError::unauthorized());
      }
      let player1 = state.player1.clone().unwrap();
      let player2 = state.player2.clone().unwrap();
      let requester: Player = request_attempt.unwrap();
      if player1.addr == requester.addr {
        let mut newplayer1 = player1;
        newplayer1.deposit += amount.u128() as u64;
        state.player1 = Some(newplayer1);
      } else {
        let mut newplayer2 = player2;
        newplayer2.deposit += amount.u128() as u64;
        state.player2 = Some(newplayer2);
      }
      deps
        .storage
        .set(b"state", &serde_json::to_vec(&state).unwrap());
      Ok(HandleResponse::default())
    }
    HandleMsg::Bet { amount } => {
      let mut state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();
      require_both_players(&mut state)?;
      let request_attempt = get_requesting_player(&deps, env);
      if request_attempt.is_none() {
        return Err(StdError::unauthorized());
      }
      let player1 = state.player1.clone().unwrap();
      let player2 = state.player2.clone().unwrap();

      let requester: Player = request_attempt.unwrap();
      if requester.deposit < amount {
        return Err(StdError::generic_err(
          "Insufficient funds. Please deposit more SCRT to continue with this bet",
        ));
      }
      state.game_board.pool += amount;
      if player1.addr == requester.addr {
        let mut newplayer1 = player1;
        newplayer1.deposit -= amount;
        state.player1 = Some(newplayer1);
      } else {
        let mut newplayer2 = player2;
        newplayer2.deposit -= amount;
        state.player1 = Some(newplayer2);
      }
      deps
        .storage
        .set(b"state", &serde_json::to_vec(&state).unwrap());
      Ok(HandleResponse::default())
    }
    HandleMsg::RequestNextTurn { direction } => {
      let contract_addr = env.clone().contract.address;
      let mut state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();
      require_both_players(&mut state)?;
      let request_attempt = get_requesting_player(&deps, env);
      if request_attempt.is_none() {
        return Err(StdError::unauthorized());
      }
      let player1 = state.player1.clone().unwrap();
      let player2 = state.player2.clone().unwrap();

      match state.game_board.winner_for_turn {
        None => Err(StdError::unauthorized()),
        Some(_) => {
          // if either player ran out of hp, the game is done, so get the money
          if player1.hp == 0 || player2.hp == 0 {
            let transfers = vec![
              CosmosMsg::Bank(BankMsg::Send {
                from_address: contract_addr.clone(),
                to_address: player1.addr,
                amount: vec![Coin::new(player1.deposit as u128, "uscrt")],
              }),
              CosmosMsg::Bank(BankMsg::Send {
                from_address: contract_addr,
                to_address: player2.addr,
                amount: vec![Coin::new(player2.deposit as u128, "uscrt")],
              }),
            ];
            state.winner = state.game_board.winner_for_turn.clone();
            deps
              .storage
              .set(b"state", &serde_json::to_vec(&state).unwrap());

            return Ok(HandleResponse {
              messages: transfers,
              log: vec![],
              data: None,
            });
          }
          state.game_board.turn += 1;
          state.game_board.winner_for_turn = None;
          state.game_board.cards = (None, None);
          state.game_board.direction = direction;
          state.game_board.turn_ended = false;
          deps
            .storage
            .set(b"state", &serde_json::to_vec(&state).unwrap());
          Ok(HandleResponse::default())
        }
      }
    }
  }
}

fn get_winner_for_cards(
  player1: &Player,
  player2: &Player,
  direction: bool,
  card1: &Card,
  card2: &Card,
) -> Option<Player> {
  if card1.value == card2.value {
    None
  } else if card1.value > card2.value {
    if direction {
      Some(player1.clone())
    } else {
      Some(player2.clone())
    }
  } else if direction {
    Some(player2.clone())
  } else {
    Some(player1.clone())
  }
}

fn require_both_players(state: &mut State) -> StdResult<bool> {
  if state.player1.is_none() || state.player2.is_none() {
    return Err(StdError::unauthorized());
  }
  Ok(true)
}

pub(crate) fn get_requesting_player<S: Storage, A: Api, Q: Querier>(
  deps: &&mut Extern<S, A, Q>,
  env: Env,
) -> Option<Player> {
  let state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();
  let mut requester: Option<Player> = None;
  if state.player1.is_none() && state.player2.is_none() {
    return None;
  }
  match state.player1 {
    None => {}
    Some(player1) => {
      if env.message.sender == player1.addr {
        requester = Some(player1)
      }
    }
  }
  match state.player2 {
    None => {}
    Some(player2) => {
      if env.message.sender == player2.addr {
        requester = Some(player2)
      }
    }
  }
  requester
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
struct PlayerHand {
  cards: Vec<Card>,
}
