use std::cmp::Ordering;

use cosmwasm_std::{
  Api, BankMsg, Coin, CosmosMsg, Env, Extern, HandleResponse, HandleResult, HumanAddr,
  InitResponse, InitResult, Querier, StdError, StdResult, Storage, Uint128,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json_wasm as serde_json;

use crate::constants::{
  ALREADY_IN_GAME, ALREADY_PUT_DOWN, AT_LEAST_1SCRT, CANT_BET_IF_FOLDED,
  CANT_PUT_CARD_AT_THE_MOMENT, CANT_PUT_CARD_IF_FOLDED, CANT_USE_CARD_TWICE, GAME_FULL,
  NOT_IN_GAME, NOT_IN_YOUR_HAND, NO_NEXT_TURN, WRONG_MATCHING_AMOUNT, WRONG_PASSWORD,
};
use crate::game_state::{Card, GameBoard, GameRound, Player, State, Word};
use crate::utils::{generate_deck, get_n_cards, get_rng, get_score_for_word};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {
  bg: u64,
  password: Option<String>,
}

pub fn init<S: Storage, A: Api, Q: Querier>(
  deps: &mut Extern<S, A, Q>,
  env: Env,
  msg: InitMsg,
) -> InitResult {
  let block_time = env.block.time;
  let state = State {
    players: vec![],
    winner: None,
    game_board: GameBoard {
      round: GameRound::None,
      winner_for_turn: None,
      words: vec![],
      river: vec![],
      pool: 0,
      turn: 0,
    },
    deck: vec![],
    can_join: true,
    started_time: block_time,
    level_design: msg.bg,
    password: msg.password,
  };

  deps
    .storage
    .set(b"state", &serde_json::to_vec(&state).unwrap());

  Ok(InitResponse::default())
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
  Join { secret: u64, password: String },
  Bet {},
  Match {},
  Fold {},
  PutDownCard { indexes: Vec<u8> },
  RequestNextTurn {},
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
  deps: &mut Extern<S, A, Q>,
  env: Env,
  msg: HandleMsg,
) -> HandleResult {
  match msg {
    HandleMsg::Join { secret, password } => {
      if env.message.sent_funds.len() != 1
        || env.message.sent_funds[0].amount != Uint128(1_000_000)
        || env.message.sent_funds[0].denom != *"uscrt"
      {
        return Err(StdError::generic_err(AT_LEAST_1SCRT));
      }

      let mut state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();

      if let Some(ref pass) = state.password {
        if &password != pass {
          return Err(StdError::generic_err(WRONG_PASSWORD));
        }
      }

      if state.players.len() == 4 {
        return Err(StdError::generic_err(GAME_FULL));
      }

      for player in state.players.clone() {
        if player.addr == env.message.sender {
          return Err(StdError::generic_err(ALREADY_IN_GAME));
        }
      }

      state.players.push(Player {
        addr: env.clone().message.sender,
        secret,
        hand: vec![],
        hp: 5,
        bet: 0,
        bet2: 0,
        folded: false,
      });

      if state.players.len() == 2 {
        // after the second player joins we need to start generating decks
        state.deck = generate_deck(get_rng(&state, &env));

        state.game_board.river = get_n_cards(&mut state, 5).to_owned();
      }

      if state.players.len() >= 2 {
        for i in 0..state.players.len() {
          if state.players[i].hand.is_empty() {
            state.players[i].hand = get_n_cards(&mut state, 5).to_owned();
          }
        }

        if state.game_board.round == GameRound::None {
          state.game_board.round = GameRound::Blind;
        }
      }

      deps
        .storage
        .set(b"state", &serde_json::to_vec(&state).unwrap());
      Ok(HandleResponse::default())
    }
    HandleMsg::PutDownCard { indexes } => {
      let mut state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();
      require_at_least_two_players(&mut state)?;
      let requester = get_requesting_player(&deps, env.clone())?;
      if state.game_board.round != GameRound::Choice {
        return Err(StdError::generic_err(CANT_PUT_CARD_AT_THE_MOMENT));
      }
      if requester.folded {
        return Err(StdError::generic_err(CANT_PUT_CARD_IF_FOLDED));
      }

      let mut indexes_used: Vec<u8> = vec![];
      for index in indexes.iter() {
        if indexes_used.contains(index) {
          return Err(StdError::generic_err(CANT_USE_CARD_TWICE));
        }
        if index > &(4_u8) && index < &(250_u8) {
          // 250...255 can be treated as the indexes for the cards on the board
          return Err(StdError::generic_err(NOT_IN_YOUR_HAND));
        }
        indexes_used.push(*index);
      }
      let mut transfers: Vec<CosmosMsg> = vec![];

      let mut word: Vec<Card> = vec![];
      for index in indexes.iter() {
        let uindex = *index as usize;
        word.push(if index >= &(250_u8) {
          state.game_board.river[uindex - 250].clone()
        } else {
          requester.hand[uindex].clone()
        });
      }
      let mut new_hand: Vec<Card> = vec![];
      for i in 0..requester.hand.len() {
        if !indexes.contains(&(i as u8)) {
          new_hand.push(requester.hand[i].clone());
        }
      }

      for i in 0..state.game_board.words.len() {
        if state.game_board.words[i].player_addr == requester.addr {
          return Err(StdError::generic_err(ALREADY_PUT_DOWN));
        }
      }

      state.game_board.words.push(Word {
        cards: word,
        player_addr: requester.addr.clone(),
      });
      if new_hand.len() < 5 {
        let count: u8 = 5 - new_hand.len() as u8;
        new_hand.append(&mut get_n_cards(&mut state, count));
        for i in 0..state.players.len() {
          if state.players[i].addr == requester.addr {
            state.players[i].hand = new_hand.clone();
          }
        }
      }

      let non_folded_players = get_non_folded_players(&mut state);

      if state.game_board.words.len() == non_folded_players.len() {
        let winner = get_winner_for_turn(&state);

        match winner {
          None => {
            state.game_board.words = vec![];
            state.game_board.round = GameRound::Blind;
          }
          Some(winner) => {
            for i in 0..state.players.len() {
              if winner.player_addr != state.players[i].addr && state.players[i].hp > 0 {
                state.players[i].hp -= 1;
              }
              state.players[i].bet = 0;
              state.players[i].bet2 = 0;
            }

            give_winner_their_money(
              &mut state,
              &mut transfers,
              env.contract.address,
              winner.player_addr,
            );
          }
        }
      }

      deps
        .storage
        .set(b"state", &serde_json::to_vec(&state).unwrap());
      send_transfers_if_any(transfers)
    }
    HandleMsg::Bet {} => {
      let mut state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();
      require_at_least_two_players(&mut state)?;

      if env.message.sent_funds.len() != 1 {
        return Err(StdError::generic_err("Didn't get any funds"));
      }
      if env.message.sent_funds[0].amount < Uint128(1_000_000) {
        return Err(StdError::generic_err("Less than 1scrt"));
      }
      if env.message.sent_funds[0].denom != *"uscrt" {
        return Err(StdError::generic_err("Not sending any scrt"));
      }
      if state.game_board.round != GameRound::Blind && state.game_board.round != GameRound::Flop {
        return Err(StdError::generic_err("You can't bet anything now"));
      }

      get_requesting_player(&deps, env.clone())?;
      state.game_board.pool += env.message.sent_funds[0].amount.clone().u128() as u64;
      for i in 0..state.players.len() {
        if state.players[i].addr == env.message.sender {
          if state.players[i].folded {
            return Err(StdError::generic_err(CANT_BET_IF_FOLDED));
          }
          if state.game_board.round == GameRound::Blind {
            state.players[i].bet += env.message.sent_funds[0].amount.clone().u128() as u64;
          } else {
            state.players[i].bet2 += env.message.sent_funds[0].amount.clone().u128() as u64;
          }
        }
      }
      let players_that_submitted_a_bet = if state.game_board.round == GameRound::Blind {
        state.players.iter().filter(|p| p.bet > 0).count()
      } else {
        state.players.iter().filter(|p| p.bet2 > 0).count()
      };
      if players_that_submitted_a_bet == state.players.len() {
        // if every player submitted a bet go to matching round
        state.game_board.round = if state.game_board.round == GameRound::Flop {
          GameRound::Matching2
        } else {
          GameRound::Matching
        }
      }

      let highest_bet = get_highest_bet(&state);

      advance_to_next_round_if_all_players_bets_match_or_have_folded(&mut state, highest_bet)?;

      deps
        .storage
        .set(b"state", &serde_json::to_vec(&state).unwrap());
      Ok(HandleResponse::default())
    }
    HandleMsg::RequestNextTurn {} => {
      let mut state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();
      require_at_least_two_players(&mut state)?;
      get_requesting_player(&deps, env)?;

      match state.game_board.winner_for_turn {
        None => return Err(StdError::generic_err(NO_NEXT_TURN)),
        Some(ref _winner) => {
          let zero_count = state.players.iter().filter(|&p| p.hp == 0).count();
          if state.players.len() == (zero_count + 1) {
            // if there's a winner
            state.winner = state.game_board.winner_for_turn.clone();
          }
          state.game_board.turn += 1;
          state.game_board.winner_for_turn = None;
          state.game_board.words = vec![];
          state.game_board.river = get_n_cards(&mut state, 5);
          state.game_board.round = GameRound::Blind;
          state.game_board.pool = 0;
          for i in 0..state.players.len() {
            state.players[i].bet = 0;
            state.players[i].bet2 = 0;
            state.players[i].folded = false;
          }
          deps
            .storage
            .set(b"state", &serde_json::to_vec(&state).unwrap());
        }
      }

      Ok(HandleResponse::default())
    }
    HandleMsg::Match {} => {
      let mut state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();
      require_at_least_two_players(&mut state)?;

      let highest_bet = get_highest_bet(&state);

      if env.message.sent_funds.len() != 1 {
        return Err(StdError::generic_err("Didn't get any funds"));
      }
      if env.message.sent_funds[0].denom != *"uscrt" {
        return Err(StdError::generic_err("Not sending any scrt"));
      }
      for i in 0..state.players.len() {
        if state.players[i].addr == env.message.sender {
          if state.game_board.round == GameRound::Matching {
            if env.message.sent_funds[0].amount
              < Uint128((highest_bet - state.players[i].bet) as u128)
            {
              return Err(StdError::generic_err(WRONG_MATCHING_AMOUNT));
            }
            state.game_board.pool += highest_bet - state.players[i].bet;
            state.players[i].bet = highest_bet;
          } else {
            if env.message.sent_funds[0].amount
              < Uint128((highest_bet - state.players[i].bet2) as u128)
            {
              return Err(StdError::generic_err(WRONG_MATCHING_AMOUNT));
            }
            state.game_board.pool += highest_bet - state.players[i].bet2;
            state.players[i].bet2 = highest_bet;
          }
        }
      }

      advance_to_next_round_if_all_players_bets_match_or_have_folded(&mut state, highest_bet)?;

      deps
        .storage
        .set(b"state", &serde_json::to_vec(&state).unwrap());

      Ok(HandleResponse::default())
    }
    HandleMsg::Fold {} => {
      let mut state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();
      require_at_least_two_players(&mut state)?;

      let highest_bet = get_highest_bet(&state);

      for i in 0..state.players.len() {
        if state.players[i].addr == env.message.sender {
          state.players[i].folded = true;
        }
      }

      advance_to_next_round_if_all_players_bets_match_or_have_folded(&mut state, highest_bet)?;
      let transfers = advance_to_next_turn_if_all_players_but_one_folded(&mut state, env);

      deps
        .storage
        .set(b"state", &serde_json::to_vec(&state).unwrap());

      send_transfers_if_any(transfers)
    }
  }
}

fn send_transfers_if_any(transfers: Vec<CosmosMsg>) -> Result<HandleResponse, StdError> {
  if transfers.len() > 0 {
    Ok(HandleResponse {
      messages: transfers,
      log: vec![],
      data: None,
    })
  } else {
    Ok(HandleResponse::default())
  }
}

fn give_winner_their_money(
  mut state: &mut State,
  transfers: &mut Vec<CosmosMsg>,
  contract_addr: HumanAddr,
  winner: HumanAddr,
) {
  state.game_board.winner_for_turn = Some(winner.clone());
  if state.game_board.pool > 0 {
    transfers.push(CosmosMsg::Bank(BankMsg::Send {
      from_address: contract_addr,
      to_address: winner.clone(),
      amount: vec![Coin::new(state.game_board.pool as u128, "uscrt")],
    }));
  }
  state.game_board.pool = 0;
}

fn get_highest_bet(state: &State) -> u64 {
  let mut highest_bet = 0;
  for player in &state.players {
    if state.game_board.round == GameRound::Matching {
      if player.bet > highest_bet {
        highest_bet = player.clone().bet;
      }
    } else {
      if player.bet2 > highest_bet {
        highest_bet = player.clone().bet2;
      }
    }
  }
  highest_bet
}

fn advance_to_next_turn_if_all_players_but_one_folded(
  state: &mut State,
  env: Env,
) -> Vec<CosmosMsg> {
  let mut transfers: Vec<CosmosMsg> = vec![];
  let non_folded = get_non_folded_players(state);
  if (state.players.len() - 1) == non_folded.len() {
    let winner = non_folded[0].clone().addr;
    give_winner_their_money(state, &mut transfers, env.contract.address, winner)
  }
  transfers
}

fn advance_to_next_round_if_all_players_bets_match_or_have_folded(
  state: &mut State,
  highest_bet: u64,
) -> StdResult<bool> {
  for i in 0..state.players.len() {
    if !state.players[i].folded {
      let player_bet = if state.game_board.round == GameRound::Matching {
        state.players[i].bet
      } else {
        state.players[i].bet2
      };
      if player_bet != highest_bet {
        return Ok(false);
      }
    }
  }
  if state.game_board.round == GameRound::Matching {
    state.game_board.round = GameRound::Flop;
  } else if state.game_board.round == GameRound::Matching2 {
    state.game_board.round = GameRound::Choice;
  }
  Ok(true)
}

fn get_non_folded_players(state: &mut State) -> Vec<Player> {
  let mut players = vec![];
  for i in 0..state.players.len() {
    if !state.players[i].folded {
      players.push(state.players[i].clone());
    }
  }
  players
}

fn get_winner_for_turn(state: &State) -> Option<Word> {
  let mut max_score = 0;
  let mut repetitions = 0;
  let mut highest_scoring_word: Option<Word> = None;

  for i in 0..state.game_board.words.len() {
    let word = state.game_board.words[i].clone();
    let score_for_word = get_score_for_word(&*word.cards);
    match score_for_word.cmp(&max_score) {
      Ordering::Equal => {
        repetitions += 1;
      }
      Ordering::Less => {}
      Ordering::Greater => {
        max_score = score_for_word;
        highest_scoring_word = Some(word.clone());
        repetitions = 0;
      }
    }
  }
  if repetitions != 0 {
    return None;
  }
  highest_scoring_word
}

fn require_at_least_two_players(state: &mut State) -> StdResult<bool> {
  if state.players.len() < 2 {
    return Err(StdError::generic_err(NOT_IN_GAME));
  }
  Ok(true)
}

pub(crate) fn get_requesting_player<S: Storage, A: Api, Q: Querier>(
  deps: &&mut Extern<S, A, Q>,
  env: Env,
) -> Result<Player, StdError> {
  let state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();

  for player in &state.players {
    if player.addr == env.message.sender {
      return Ok(player.clone());
    }
  }
  Err(StdError::generic_err(NOT_IN_GAME))
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
struct PlayerHand {
  cards: Vec<Card>,
}
