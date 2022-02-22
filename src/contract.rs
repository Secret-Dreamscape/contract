use std::cmp::Ordering;

use cosmwasm_std::{
  Api, BankMsg, Coin, CosmosMsg, Env, Extern, HandleResponse, HandleResult, HumanAddr,
  InitResponse, InitResult, Querier, StdError, StdResult, Storage, Uint128,
};
use schemars::JsonSchema;
use secret_toolkit::utils::HandleCallback;
use serde::{Deserialize, Serialize};
use serde_json_wasm as serde_json;

use crate::constants::{
  ALREADY_IN_GAME, ALREADY_PUT_DOWN, CANT_BET_IF_FOLDED, CANT_CHECK_IF_NEED_TO_MATCH,
  CANT_PUT_CARD_AT_THE_MOMENT, CANT_PUT_CARD_IF_FOLDED, CANT_USE_CARD_TWICE, GAME_FULL,
  NOT_IN_GAME, NOT_IN_YOUR_HAND, NO_NEXT_TURN, WRONG_MATCHING_AMOUNT, WRONG_PASSWORD,
};
use crate::game_state::{Card, GameBoard, GameRound, Player, PlayerAction, State, Word};
use crate::utils::cards::{find_word_id, generate_deck, get_n_cards, get_rng, get_score_for_word};
use crate::utils::general::get_non_folded_players;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {
  pub bg: u64,
  pub password: Option<String>,
  pub label: String,
  pub stamp_addr: HumanAddr,
  pub stamp_hash: String,
  pub callback_addr: HumanAddr,
  pub callback_hash: String,
  pub min_buy: u64,
  pub max_buy: u64,
  pub jackpot_addr: HumanAddr,
  pub jackpot_hash: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PhonebookHandleMsg {
  RegisteredCallback {
    address: HumanAddr,
    private: bool,
    label: String,
    referrer: String,
  },
}

impl HandleCallback for PhonebookHandleMsg {
  const BLOCK_SIZE: usize = 256;
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
      rake_percentage: 10,
    },
    deck: vec![],
    can_join: true,
    started_time: block_time,
    level_design: msg.bg,
    password: msg.password.clone(),
    stamp_hash: msg.stamp_hash.clone(),
    stamp_addr: msg.stamp_addr,
    min_buy: msg.min_buy,
    max_buy: msg.max_buy,
    jackpot_addr: msg.jackpot_addr,
    jackpot_hash: msg.jackpot_hash,
  };

  let callback_msg = PhonebookHandleMsg::RegisteredCallback {
    address: env.contract.address.clone(),
    private: msg.password.is_some(),
    label: msg.label.clone(),
    referrer: msg.callback_hash.clone(),
  };

  let cosmos_msg =
    callback_msg.to_cosmos_msg(msg.callback_hash.clone(), msg.callback_addr.clone(), None)?;

  deps
    .storage
    .set(b"state", &serde_json::to_vec(&state).unwrap());

  Ok(InitResponse {
    messages: vec![cosmos_msg],
    log: vec![],
  })
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SecretDreamscapeNFT {
  pub id: String,
  pub letter: String,
  pub gold: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SecretDreamscapeJackpot {
  Fund {},
}

impl HandleCallback for SecretDreamscapeJackpot {
  const BLOCK_SIZE: usize = 256;
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
  Join {
    secret: u64,
    password: String,
    nfts: Vec<SecretDreamscapeNFT>,
  },
  BuyChips {},
  Bet {
    amount: u64,
  },
  Match {
    amount: u64,
  },
  Fold {},
  Check {},
  Leave {},
  PutDownCard {
    indexes: Vec<u8>,
    opened_dictionary: bool,
  },
  RequestNextTurn {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StampHandleMsg {
  Stamp {
    nft_id: String,
    word_id: u16,
    callee: HumanAddr,
  },
}

impl HandleCallback for StampHandleMsg {
  const BLOCK_SIZE: usize = 256;
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
    HandleMsg::Join {
      secret,
      password,
      nfts,
    } => {
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
        folded: state.players.len() > 1,
        checked: false,
        checked2: false,
        opened_dictionary: false,
        last_action: if state.players.len() > 1 {
          Some(PlayerAction::Folded)
        } else {
          None
        },
        nfts,
        chips: 0,
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
    HandleMsg::BuyChips {} => {
      get_requesting_player(&state, env.clone())?;
      if env.message.sent_funds.len() != 1 {
        return Err(StdError::generic_err(
          "You can only send SCRT to this function",
        ));
      }
      if env.message.sent_funds[0].denom != "uscrt" {
        return Err(StdError::generic_err(
          "You can only send SCRT to this function",
        ));
      }

      let amount = env.message.sent_funds[0].amount.u128() as u64;

      if amount < state.min_buy || amount > state.max_buy {
        return Err(StdError::generic_err(
          "You didn't send enough SCRT to this function or you sent too much",
        ));
      }

      for i in 0..state.players.len() {
        if state.players[i].addr == env.message.sender {
          if state.players[i].chips + amount > state.max_buy {
            return Err(StdError::generic_err(
              "You can't buy more chips than the maximum amount",
            ));
          }
          state.players[i].chips += amount;
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
      let requester = get_requesting_player(&state, env.clone())?;
      if state.game_board.round != GameRound::Choice {
        return Err(StdError::generic_err(CANT_PUT_CARD_AT_THE_MOMENT));
      }
      if requester.folded {
        return Err(StdError::generic_err(CANT_PUT_CARD_IF_FOLDED));
      }

      if state
        .game_board
        .words
        .iter()
        .any(|w| w.player_addr == requester.addr)
      {
        return Err(StdError::generic_err(ALREADY_PUT_DOWN));
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
      let mut messages: Vec<CosmosMsg> = vec![];
      let mut new_hand: Vec<Card> = vec![];
      for i in 0..requester.hand.len() {
        if !indexes.contains(&(i as u8)) {
          new_hand.push(requester.hand[i].clone());
        }
      }
      let mut word_string: String = "".to_string();
      let word_id = find_word_id(word_string.as_str()).unwrap_or(0);
      for i in 0..word.len() {
        let card = word[i].clone();
        let letter = ('A' as u8 + card.letter) as char;
        word_string.push(letter);
        let nft_for_card = requester
          .nfts
          .iter()
          .find(|nft| nft.letter == letter.to_string() && nft.gold == card.gold);
        if let Some(nft) = nft_for_card {
          let message = StampHandleMsg::Stamp {
            nft_id: nft.clone().id,
            word_id: word_id as u16,
            callee: env.clone().message.sender,
          };
          let cosmos_msg =
            message.to_cosmos_msg(state.stamp_hash.clone(), state.stamp_addr.clone(), None)?;
          messages.push(cosmos_msg);
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

        let mut winner_indexes: Vec<usize> = vec![];

        winners.iter().for_each(|w| {
          winner_indexes.push(
            state
              .players
              .iter()
              .position(|p| p.addr == w.player_addr)
              .unwrap(),
          );
        });

        state.game_board.winner_for_turn = Some(winners[0].clone().player_addr);

        for i in 0..state.players.len() {
          // if !winner_addresses.contains(&state.players[i].addr.clone()) && state.players[i].hp > 0 {
          //  state.players[i].hp -= 1;
          // }
          state.players[i].bet = 0;
          state.players[i].bet2 = 0;
        }

        let transfers = give_winners_their_money(&mut state, winner_indexes)?;
        for t in transfers {
          messages.push(t.clone());
        }
      }

      deps
        .storage
        .set(b"state", &serde_json::to_vec(&state).unwrap());
      send_messages_if_any(messages)
    }
    HandleMsg::Bet { amount } => {
      require_at_least_two_players(&mut state)?;

      let player = get_requesting_player(&state, env.clone())?;

      if player.folded {
        return Err(StdError::generic_err(CANT_BET_IF_FOLDED));
      }

      if amount < 125_000 {
        return Err(StdError::generic_err("Less than  0.125scrt"));
      }

      if player.chips < amount {
        return Err(StdError::generic_err("Not enough chips"));
      }

      if state.game_board.round != GameRound::Blind && state.game_board.round != GameRound::Flop {
        return Err(StdError::generic_err("You can't bet anything now"));
      }

      state.game_board.pool += amount;
      for i in 0..state.players.len() {
        if state.players[i].addr == env.message.sender {
          if state.players[i].folded {
            return Err(StdError::generic_err(CANT_BET_IF_FOLDED));
          }
          state.players[i].last_action = Some(PlayerAction::SentBet(amount));
          if state.game_board.round == GameRound::Blind {
            state.players[i].bet += amount;
          } else {
            state.players[i].bet2 += amount;
          }
          state.players[i].chips -= amount;
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
      get_requesting_player(&state, env)?;

      match state.game_board.winner_for_turn {
        None => return Err(StdError::generic_err(NO_NEXT_TURN)),
        Some(ref _winner) => {
          // let zero_count = state.players.iter().filter(|&p| p.hp == 0).count();
          // if state.players.len() == (zero_count + 1) {
          //   // if there's a winner
          //   state.winner = state.game_board.winner_for_turn.clone();
          // }
          state.game_board.turn += 1;
          state.game_board.winner_for_turn = None;
          state.game_board.words = vec![];
          state.game_board.river = get_n_cards(&mut state, 5);
          state.game_board.round = GameRound::Blind;
          state.game_board.pool = 0;
          for i in 0..state.players.len() {
            state.players[i].bet = 0;
            state.players[i].bet2 = 0;
            if state.players[i].chips < 125_000 {
              state.players[i].folded = true;
              state.players[i].last_action = Some(PlayerAction::Folded);
            } else {
              state.players[i].folded = false;
              state.players[i].last_action = None;
            }
            state.players[i].checked = false;
            state.players[i].checked2 = false;
            state.players[i].opened_dictionary = false;
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
    HandleMsg::Match { amount } => {
      require_at_least_two_players(&mut state)?;

      let highest_bet = get_highest_bet(&state);

      let player = get_requesting_player(&state, env.clone())?;

      if player.folded {
        return Err(StdError::generic_err(CANT_BET_IF_FOLDED));
      }

      if player.chips < amount {
        return Err(StdError::generic_err("Not enough chips"));
      }

      for i in 0..state.players.len() {
        if state.players[i].addr == env.message.sender {
          if state.game_board.round == GameRound::Matching {
            if amount < (highest_bet - state.players[i].bet) {
              return Err(StdError::generic_err(WRONG_MATCHING_AMOUNT));
            }
            state.game_board.pool += highest_bet - state.players[i].bet;
            state.players[i].bet = highest_bet;
          } else {
            if amount < (highest_bet - state.players[i].bet2) {
              return Err(StdError::generic_err(WRONG_MATCHING_AMOUNT));
            }
            state.game_board.pool += highest_bet - state.players[i].bet2;
            state.players[i].bet2 = highest_bet;
          }
          state.players[i].chips -= amount;
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
      let messages = advance_to_next_turn_if_all_players_but_one_folded(&mut state)?;

      deps
        .storage
        .set(b"state", &serde_json::to_vec(&state).unwrap());

      send_messages_if_any(messages)
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
      let messages = advance_to_next_turn_if_all_players_but_one_folded(&mut state)?;
      deps
        .storage
        .set(b"state", &serde_json::to_vec(&state).unwrap());

      send_messages_if_any(messages)
    }
    HandleMsg::Leave {} => {
      let mut chips: u64 = 0;
      for i in 0..state.players.len() {
        if state.players[i].addr == env.message.sender {
          chips = state.players[i].chips;
          state.players.remove(i);
          break;
        }
      }

      advance_turn_if_necessary(&mut state);
      let mut messages = advance_to_next_turn_if_all_players_but_one_folded(&mut state)?;
      if chips > 0 {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
          from_address: env.contract.address.clone(),
          to_address: env.message.sender.clone(),
          amount: vec![Coin::new(chips as u128, "uscrt")],
        }));
      }
      deps
        .storage
        .set(b"state", &serde_json::to_vec(&state).unwrap());

      send_messages_if_any(messages)
    }
  }
}

fn send_messages_if_any(messages: Vec<CosmosMsg>) -> Result<HandleResponse, StdError> {
  if !messages.is_empty() {
    Ok(HandleResponse {
      messages,
      log: vec![],
      data: None,
    })
  } else {
    Ok(HandleResponse::default())
  }
}

fn give_winners_their_money(
  mut state: &mut State,
  winners: Vec<usize>,
) -> Result<Vec<CosmosMsg>, StdError> {
  state.game_board.winner_for_turn = Some(state.players[winners[0]].addr.clone());
  let rake = state.game_board.pool * state.game_board.rake_percentage / 100;
  let amount_per_transfer = (state.game_board.pool - rake) / (winners.len() as u64);
  if state.game_board.pool > 0 {
    for i in 0..winners.len() {
      state.players[winners[i]].chips += amount_per_transfer;
    }
    state.game_board.pool = 0;
    let msg = SecretDreamscapeJackpot::Fund {}.to_cosmos_msg(
      state.jackpot_hash.clone(),
      state.jackpot_addr.clone(),
      Some(Uint128(rake as u128)),
    )?;
    return Ok(vec![msg]);
  }
  return Ok(vec![]);
}

fn get_highest_bet(state: &State) -> u64 {
  if state.game_board.round == GameRound::Matching {
    state.players.iter().fold(
      0,
      |acc, player| {
        if player.bet > acc {
          player.bet
        } else {
          acc
        }
      },
    )
  } else {
    state.players.iter().fold(
      0,
      |acc, player| {
        if player.bet2 > acc {
          player.bet2
        } else {
          acc
        }
      },
    )
  }
}

fn advance_to_next_turn_if_all_players_but_one_folded(
  state: &mut State,
) -> Result<Vec<CosmosMsg>, StdError> {
  let non_folded = get_non_folded_players(state);
  if non_folded.len() == 1 {
    let winner = non_folded[0].clone().addr;
    let non_folded_index = state.players.iter().position(|p| p.addr == winner).unwrap();
    return give_winners_their_money(state, vec![non_folded_index]);
  }
  Ok(vec![])
}

fn get_bet_stats(state: &mut State) -> (bool, bool, bool, bool, bool) {
  let mut last_bet = 0;
  let mut all_non_folded_players_bet = true;
  let mut all_players_bet_the_same_amount = true;
  let at_least_one_player_checked = state.players.iter().any(|p| match state.game_board.round {
    GameRound::Blind => p.checked,
    GameRound::Flop => p.checked2,
    _ => false,
  });
  let all_non_folded_players_checked =
    state
      .players
      .iter()
      .filter(|p| !p.folded)
      .all(|p| match state.game_board.round {
        GameRound::Blind => p.checked,
        GameRound::Flop => p.checked2,
        _ => false,
      });
  let all_players_acted = state.players.iter().all(|p| p.last_action.is_some());
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
      if player_bet == 0 {
        if check_status {
          continue;
        }
        all_non_folded_players_bet = false;
      } else if player_bet != last_bet {
        if last_bet != 0 {
          all_players_bet_the_same_amount = false;
        }
        last_bet = player_bet;
      }
    }
  }
  (
    all_players_acted,
    all_non_folded_players_bet,
    all_players_bet_the_same_amount,
    at_least_one_player_checked,
    all_non_folded_players_checked,
  )
}

fn advance_turn_if_necessary(state: &mut State) {
  let previous_round = state.game_board.round.clone();
  let (
    all_players_acted,
    all_non_folded_players_bet,
    all_players_bet_the_same_amount,
    at_least_one_player_checked,
    all_non_folded_players_checked,
  ) = get_bet_stats(state);
  if state.game_board.round != GameRound::Matching
    && state.game_board.round != GameRound::Matching2
    && !all_players_acted
  {
    return;
  }
  if all_non_folded_players_checked {
    match state.game_board.round {
      GameRound::Blind => state.game_board.round = GameRound::Flop,
      GameRound::Flop => state.game_board.round = GameRound::Choice,
      GameRound::Matching => state.game_board.round = GameRound::Flop,
      GameRound::Matching2 => state.game_board.round = GameRound::Choice,
      _ => {}
    }
  } else if at_least_one_player_checked {
    match state.game_board.round {
      GameRound::Blind => state.game_board.round = GameRound::Matching,
      GameRound::Flop => state.game_board.round = GameRound::Matching2,
      _ => {}
    }
  } else {
    match state.game_board.round {
      GameRound::None => {}
      GameRound::Blind => match (all_non_folded_players_bet, all_players_bet_the_same_amount) {
        (true, false) => state.game_board.round = GameRound::Matching,
        (true, true) => state.game_board.round = GameRound::Flop,
        _ => {}
      },
      GameRound::Matching => {
        if let (true, true) = (all_non_folded_players_bet, all_players_bet_the_same_amount) {
          state.game_board.round = GameRound::Flop
        }
      }
      GameRound::Flop => match (all_non_folded_players_bet, all_players_bet_the_same_amount) {
        (true, false) => state.game_board.round = GameRound::Matching2,
        (true, true) => state.game_board.round = GameRound::Choice,
        _ => {}
      },
      GameRound::Matching2 => {
        if let (true, true) = (all_non_folded_players_bet, all_players_bet_the_same_amount) {
          state.game_board.round = GameRound::Choice
        }
      }
      GameRound::Choice => {}
    }
  }
  if previous_round != state.game_board.round {
    state
      .players
      .iter_mut()
      .filter(|p| !p.folded)
      .for_each(|p| {
        p.last_action = None;
      });
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

pub(crate) fn get_requesting_player(state: &State, env: Env) -> Result<Player, StdError> {
  let player = state.players.iter().find(|p| p.addr == env.message.sender);
  match player {
    Some(p) => Ok(p.clone()),
    None => Err(StdError::generic_err(NOT_IN_GAME)),
  }
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
struct PlayerHand {
  cards: Vec<Card>,
}
