use cosmwasm_std::{to_binary, Api, Extern, HumanAddr, Querier, QueryResult, StdError, Storage};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json_wasm as serde_json;

use crate::constants::{NOT_IN_GAME, WAITING_FOR_PLAYERS};
use crate::game_state::{Card, GameRound, Player, PlayerAction, State, Word};
use crate::utils::cards::get_score_for_word;
use crate::utils::general::get_non_folded_players;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
  GetResult {},
  CanJoin {},
  GetGameState { secret: u64 },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Result {
  winner: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GameState {
  pub pool: u64,

  pub players: Vec<PlayerStatus>,

  pub turn: u8,
  pub round: GameRound,

  pub hand: Vec<Card>,
  pub river: Option<Vec<Card>>,

  pub words: Vec<WordView>,
  pub winner: Option<HumanAddr>,
  pub level_design: u64,
  pub min_buy: u64,
  pub max_buy: u64,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct WordView {
  pub word: Option<Word>,
  pub points: u16,
  pub visible: bool,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PlayerStatus {
  hp: u8,
  bet: u64,
  addr: HumanAddr,
  folded: bool,
  pub last_action: Option<PlayerAction>,
  opened_dictionary: bool,
  chips: u64,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CanJoinResponse {
  can_join: bool,
  started_time: u64,
  pub requires_password: bool,
}

fn get_stats_for_players(saved_state: &State, output_state: &mut GameState) {
  for player in &saved_state.players {
    output_state.players.push(PlayerStatus {
      hp: player.clone().hp,
      addr: player.clone().addr,
      bet: if saved_state.game_board.round == GameRound::Flop
        || saved_state.game_board.round == GameRound::Matching2
        || saved_state.game_board.round == GameRound::Choice
      {
        player.clone().bet2
      } else {
        player.clone().bet
      },
      folded: player.folded,
      last_action: player.clone().last_action,
      opened_dictionary: player.clone().opened_dictionary,
      chips: player.clone().chips,
    })
  }
}

fn get_hand(
  secret: &u64,
  saved_state: &State,
  output_state: &mut GameState,
) -> serde::__private::Result<(), StdError> {
  for player in &saved_state.players {
    if &player.secret == secret {
      output_state.hand = (*player.hand).to_owned();
      return Ok(());
    }
  }
  Err(StdError::generic_err(NOT_IN_GAME))
}

fn get_words(
  secret: &u64,
  saved_state: &State,
  output_state: &mut GameState,
) -> serde::__private::Result<(), StdError> {
  if saved_state.game_board.round != GameRound::Choice {
    return Ok(());
  }

  let mut player_with_secret: Option<Player> = None;
  for player in &saved_state.players {
    if &player.secret == secret {
      player_with_secret = Some(player.clone());
      break;
    }
  }

  if let Some(player) = player_with_secret {
    let non_folded_players = get_non_folded_players(saved_state);
    let words_submitted_count = saved_state.game_board.words.len();
    for word in &saved_state.game_board.words {
      if words_submitted_count == non_folded_players.len() {
        output_state.words.push(WordView {
          word: Some(word.clone()),
          points: get_score_for_word(&word.cards),
          visible: true,
        });
      } else {
        output_state.words.push(WordView {
          word: Some(Word {
            cards: if word.player_addr == player.addr {
              word.cards.clone()
            } else {
              vec![]
            },
            player_addr: word.player_addr.clone(),
          }),
          points: if word.player_addr == player.addr {
            get_score_for_word(&word.cards)
          } else {
            0
          },
          visible: word.player_addr == player.addr,
        });
      }
    }
    Ok(())
  } else {
    Err(StdError::generic_err(NOT_IN_GAME))
  }
}

fn get_river(saved_state: &State, output_state: &mut GameState) {
  match saved_state.game_board.round {
    GameRound::None | GameRound::Blind | GameRound::Matching => {}
    GameRound::Flop | GameRound::Matching2 | GameRound::Choice => {
      output_state.river = Some((*saved_state.game_board.river).to_owned());
    }
  }
}

pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
  match msg {
    QueryMsg::GetResult {} => {
      let state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();

      if state.winner.is_none() {
        return Err(StdError::generic_err(WAITING_FOR_PLAYERS));
      }

      to_binary(&vec![state.winner.unwrap()])
    }
    QueryMsg::CanJoin {} => {
      let state: State = serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();
      let can_join = state.players.len() < 4;
      let resp = CanJoinResponse {
        can_join,
        started_time: state.started_time,
        requires_password: state.password.is_some(),
      };
      Ok(to_binary(&resp).unwrap())
    }
    QueryMsg::GetGameState { secret } => {
      let saved_state: State =
        serde_json::from_slice(&deps.storage.get(b"state").unwrap()).unwrap();

      let mut output_state = GameState {
        words: vec![],
        pool: saved_state.game_board.pool,
        river: None,
        turn: saved_state.game_board.turn,
        winner: if let Some(winner) = saved_state.winner.clone() {
          Some(winner)
        } else {
          saved_state.game_board.winner_for_turn.clone()
        },
        round: saved_state.game_board.round.clone(),
        players: vec![],
        hand: vec![],
        level_design: saved_state.level_design,
        min_buy: saved_state.min_buy,
        max_buy: saved_state.max_buy,
      };

      get_stats_for_players(&saved_state, &mut output_state);
      get_hand(&secret, &saved_state, &mut output_state)?;
      get_words(&secret, &saved_state, &mut output_state)?;
      get_river(&saved_state, &mut output_state);

      Ok(to_binary(&output_state).unwrap())
    }
  }
}
