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
  CANT_CHECK_IF_NEED_TO_MATCH, CANT_PUT_CARD_AT_THE_MOMENT, CANT_PUT_CARD_IF_FOLDED,
  CANT_USE_CARD_TWICE, GAME_FULL, NOT_IN_GAME, NOT_IN_YOUR_HAND, NO_NEXT_TURN,
  WRONG_MATCHING_AMOUNT, WRONG_PASSWORD,
};
use crate::game_state::{Card, GameBoard, GameRound, Player, PlayerAction, State, Word};
use crate::utils::cards::{generate_deck, get_n_cards, get_rng, get_score_for_word};
use crate::utils::general::get_non_folded_players;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {
  pub bg: u64,
  pub password: Option<String>,
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
  Join {
    secret: u64,
    password: String,
  },
  Bet {},
  Match {},
  Fold {},
  Check {},
  Leave {},
  PutDownCard {
    indexes: Vec<u8>,
    opened_dictionary: bool,
  },
  RequestNextTurn {},
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
  deps: &mut Extern<S, A, Q>,
  env: Env,
  msg: HandleMsg,
) -> HandleResult {
  let mut state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();
  if state.winner.is_some() {
    return Err(StdError::generic_err("Game is over"));
  }
  match msg {
    HandleMsg::Join { secret, password } => {
      if env.message.sent_funds.len() != 1
        || env.message.sent_funds[0].amount != Uint128(1_000_000)
        || env.message.sent_funds[0].denom != *"uscrt"
      {
        return Err(StdError::generic_err(AT_LEAST_1SCRT));
      }

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
        checked: false,
        checked2: false,
        opened_dictionary: false,
        last_action: None,
      });

      if state.players.len() == 2 {
        // after the second player joins we need to start generating decks
        state.deck = generate_deck(get_rng(&state, &env));

        state.game_board.river = get_n_cards(&mut state, 5);
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
    HandleMsg::PutDownCard {
      indexes,
      opened_dictionary,
    } => {
      require_at_least_two_players(&mut state)?;
      let requester = get_requesting_player(&deps, env.clone())?;
      if state.game_board.round != GameRound::Choice {
        return Err(StdError::generic_err(CANT_PUT_CARD_AT_THE_MOMENT));
      }
      if requester.folded {
        return Err(StdError::generic_err(CANT_PUT_CARD_IF_FOLDED));
      }

      for i in 0..state.game_board.words.len() {
        if state.game_board.words[i].player_addr == requester.addr {
          return Err(StdError::generic_err(ALREADY_PUT_DOWN));
        }
      }

      let mut indexes_used: Vec<u8> = vec![];
      let mut word: Vec<Card> = vec![];
      for index in indexes.iter() {
        if indexes_used.contains(index) {
          return Err(StdError::generic_err(CANT_USE_CARD_TWICE));
        }
        if index > &(4_u8) && index < &(250_u8) {
          // 250...255 can be treated as the indexes for the cards on the board
          return Err(StdError::generic_err(NOT_IN_YOUR_HAND));
        }
        word.push(if index >= &(250_u8) {
          state.game_board.river[(*index as usize) - 250].clone()
        } else {
          requester.hand[(*index as usize)].clone()
        });
        indexes_used.push(*index);
      }
      let mut transfers: Vec<CosmosMsg> = vec![];
      let mut new_hand: Vec<Card> = vec![];
      for i in 0..requester.hand.len() {
        if !indexes.contains(&(i as u8)) {
          new_hand.push(requester.hand[i].clone());
        }
      }

      state.game_board.words.push(Word {
        cards: word,
        player_addr: requester.addr.clone(),
      });

      for i in 0..state.players.len() {
        if state.players[i].addr == requester.addr {
          state.players[i].opened_dictionary = opened_dictionary;
          state.players[i].last_action = Some(PlayerAction::ChoseWord);
          state.players[i].hand = new_hand.clone();
          break;
        }
      }

      let non_folded_players = get_non_folded_players(&state);

      if state.game_board.words.len() == non_folded_players.len() {
        let winners = get_winners_for_turn(&state);

        let mut winner_addresses = vec![];

        state.game_board.winner_for_turn = Some(winners[0].clone().player_addr);

        for i in 0..winners.len() {
          winner_addresses.push(winners[i].player_addr.clone());
        }

        for i in 0..state.players.len() {
          if !winner_addresses.contains(&state.players[i].addr.clone()) && state.players[i].hp > 0 {
            state.players[i].hp -= 1;
          }
          state.players[i].bet = 0;
          state.players[i].bet2 = 0;
        }

        give_winners_their_money(
          &mut state,
          &mut transfers,
          env.contract.address,
          winner_addresses,
        );
      }

      deps
        .storage
        .set(b"state", &serde_json::to_vec(&state).unwrap());
      send_transfers_if_any(transfers)
    }
    HandleMsg::Bet {} => {
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
          let amount = env.message.sent_funds[0].amount;
          state.players[i].last_action = Some(PlayerAction::SentBet(amount.u128() as u64));
          if state.game_board.round == GameRound::Blind {
            state.players[i].bet += amount.u128() as u64;
          } else {
            state.players[i].bet2 += amount.u128() as u64;
          }
        }
      }

      advance_turn_if_necessary(&mut state);

      deps
        .storage
        .set(b"state", &serde_json::to_vec(&state).unwrap());
      Ok(HandleResponse::default())
    }
    HandleMsg::RequestNextTurn {} => {
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
            state.players[i].checked = false;
            state.players[i].checked2 = false;
            state.players[i].opened_dictionary = false;
            state.players[i].last_action = None;
            let mut new_hand = state.players[i].hand.clone();
            if new_hand.len() < 5 {
              let count: u8 = 5 - new_hand.len() as u8;
              new_hand.append(&mut get_n_cards(&mut state, count));
              state.players[i].hand = new_hand.clone();
            }
          }
          deps
            .storage
            .set(b"state", &serde_json::to_vec(&state).unwrap());
        }
      }

      Ok(HandleResponse::default())
    }
    HandleMsg::Match {} => {
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
          state.players[i].last_action = Some(PlayerAction::MatchedBet);
        }
      }

      advance_turn_if_necessary(&mut state);

      deps
        .storage
        .set(b"state", &serde_json::to_vec(&state).unwrap());

      Ok(HandleResponse::default())
    }
    HandleMsg::Fold {} => {
      require_at_least_two_players(&mut state)?;

      for i in 0..state.players.len() {
        if state.players[i].addr == env.message.sender {
          state.players[i].folded = true;
          state.players[i].hand = get_n_cards(&mut state, 5);
          state.players[i].last_action = Some(PlayerAction::Folded);
        }
      }

      advance_turn_if_necessary(&mut state);
      let transfers = advance_to_next_turn_if_all_players_but_one_folded(&mut state, env);

      deps
        .storage
        .set(b"state", &serde_json::to_vec(&state).unwrap());

      send_transfers_if_any(transfers)
    }
    HandleMsg::Check {} => {
      require_at_least_two_players(&mut state)?;

      match state.game_board.round {
        GameRound::Blind | GameRound::Flop => {
          for i in 0..state.players.len() {
            if state.players[i].addr == env.message.sender {
              if state.game_board.round == GameRound::Blind {
                state.players[i].checked = true;
              } else {
                state.players[i].checked2 = true;
              }
              state.players[i].last_action = Some(PlayerAction::Checked);
            }
          }
        }
        _ => return Err(StdError::generic_err(CANT_CHECK_IF_NEED_TO_MATCH)),
      }

      advance_turn_if_necessary(&mut state);
      let transfers = advance_to_next_turn_if_all_players_but_one_folded(&mut state, env);
      deps
        .storage
        .set(b"state", &serde_json::to_vec(&state).unwrap());

      send_transfers_if_any(transfers)
    }
    HandleMsg::Leave {} => {
      for i in 0..state.players.len() {
        if state.players[i].addr == env.message.sender {
          state.players.remove(i);
          break;
        }
      }

      advance_turn_if_necessary(&mut state);
      let transfers = advance_to_next_turn_if_all_players_but_one_folded(&mut state, env);
      deps
        .storage
        .set(b"state", &serde_json::to_vec(&state).unwrap());

      send_transfers_if_any(transfers)
    }
  }
}

fn send_transfers_if_any(transfers: Vec<CosmosMsg>) -> Result<HandleResponse, StdError> {
  if !transfers.is_empty() {
    Ok(HandleResponse {
      messages: transfers,
      log: vec![],
      data: None,
    })
  } else {
    Ok(HandleResponse::default())
  }
}

fn give_winners_their_money(
  mut state: &mut State,
  transfers: &mut Vec<CosmosMsg>,
  contract_addr: HumanAddr,
  winners: Vec<HumanAddr>,
) {
  state.game_board.winner_for_turn = Some(winners[0].clone());
  let amount_per_transfer = (state.game_board.pool as u128) / (winners.len() as u128);
  if state.game_board.pool > 0 {
    for i in 0..winners.len() {
      transfers.push(CosmosMsg::Bank(BankMsg::Send {
        from_address: contract_addr.clone(),
        to_address: winners[i].clone(),
        amount: vec![Coin::new(amount_per_transfer, "uscrt")],
      }));
    }
    state.game_board.pool = 0;
  }
}

fn get_highest_bet(state: &State) -> u64 {
  let mut highest_bet = 0;
  for player in &state.players {
    if state.game_board.round == GameRound::Matching {
      if player.bet > highest_bet {
        highest_bet = player.clone().bet;
      }
    } else if player.bet2 > highest_bet {
      highest_bet = player.clone().bet2;
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
  if non_folded.len() == 1 {
    let winner = non_folded[0].clone().addr;
    give_winners_their_money(state, &mut transfers, env.contract.address, vec![winner])
  }
  transfers
}

fn get_bet_stats(state: &mut State) -> (bool, bool) {
  let mut last_bet = 0;
  let mut all_non_folded_players_bet_or_checked = true;
  let mut all_players_bet_the_same_amount = true;
  for i in 0..state.players.len() {
    if !state.players[i].folded {
      let player_bet = match state.game_board.round {
        GameRound::None => 0,
        GameRound::Blind | GameRound::Matching => state.players[i].bet,
        GameRound::Flop | GameRound::Matching2 => state.players[i].bet2,
        GameRound::Choice => state.players[i].bet + state.players[i].bet2,
      };
      let check_status = match state.game_board.round {
        GameRound::Blind => state.players[i].checked,
        GameRound::Flop => state.players[i].checked2,
        _ => false,
      };
      if check_status {
        continue;
      }
      if player_bet == 0 {
        all_non_folded_players_bet_or_checked = false;
      } else if player_bet != last_bet {
        if last_bet != 0 {
          all_players_bet_the_same_amount = false;
        }
        last_bet = player_bet;
      }
    }
  }
  (
    all_non_folded_players_bet_or_checked,
    all_players_bet_the_same_amount,
  )
}

fn advance_turn_if_necessary(state: &mut State) {
  let previous_round = state.game_board.round.clone();
  match state.game_board.round {
    GameRound::None => {}
    GameRound::Blind => match get_bet_stats(state) {
      (true, false) => state.game_board.round = GameRound::Matching,
      (true, true) => state.game_board.round = GameRound::Flop,
      _ => {}
    },
    GameRound::Matching => {
      if let (true, true) = get_bet_stats(state) {
        state.game_board.round = GameRound::Flop
      }
    }
    GameRound::Flop => match get_bet_stats(state) {
      (true, false) => state.game_board.round = GameRound::Matching2,
      (true, true) => state.game_board.round = GameRound::Choice,
      _ => {}
    },
    GameRound::Matching2 => {
      if let (true, true) = get_bet_stats(state) {
        state.game_board.round = GameRound::Choice
      }
    }
    GameRound::Choice => {}
  }
  if previous_round != state.game_board.round {
    for i in 0..state.players.len() {
      state.players[i].last_action = None;
    }
  }
}

fn get_winners_for_turn(state: &State) -> Vec<Word> {
  let mut max_score = 0;
  let mut highest_scoring_words: Vec<Word> = vec![];

  for i in 0..state.game_board.words.len() {
    let word = state.game_board.words[i].clone();
    let score_for_word = get_score_for_word(&*word.cards);
    match score_for_word.cmp(&max_score) {
      Ordering::Equal => {
        highest_scoring_words.push(word.clone());
      }
      Ordering::Less => {}
      Ordering::Greater => {
        max_score = score_for_word;
        highest_scoring_words = vec![word.clone()];
      }
    }
  }
  highest_scoring_words
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
