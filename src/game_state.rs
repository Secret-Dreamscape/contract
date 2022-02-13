use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::contract::SecretDreamscapeNFT;

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub struct State {
  pub players: Vec<Player>,
  pub can_join: bool,

  pub game_board: GameBoard,
  pub winner: Option<HumanAddr>,
  pub deck: Vec<Card>,
  pub started_time: u64,
  pub level_design: u64,
  pub password: Option<String>,
  pub stamp_hash: String,
  pub stamp_addr: HumanAddr,

  pub min_buy: u64,
  pub max_buy: u64,
  pub jackpot_addr: HumanAddr,
  pub jackpot_hash: String,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, PartialEq)]
pub enum PlayerAction {
  SentBet(u64),
  MatchedBet,
  Folded,
  ChoseWord,
  Checked,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub struct Player {
  pub addr: HumanAddr,
  pub secret: u64,
  pub hp: u8,
  pub hand: Vec<Card>,
  pub bet: u64,
  pub bet2: u64,
  pub folded: bool,
  pub checked: bool,
  pub checked2: bool,
  pub opened_dictionary: bool,
  pub last_action: Option<PlayerAction>,
  pub nfts: Vec<SecretDreamscapeNFT>,
  pub chips: u64,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub struct Card {
  pub letter: u8,
  pub gold: bool,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub struct GameBoard {
  pub turn: u8,
  pub round: GameRound,
  pub winner_for_turn: Option<HumanAddr>,
  pub words: Vec<Word>,
  pub river: Vec<Card>,
  pub pool: u64,
  pub rake_percentage: u64,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub struct Word {
  pub cards: Vec<Card>,
  pub player_addr: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, PartialEq)]
pub enum GameRound {
  // turn not started yet, initial state
  None,

  // Community cards are not shown and every player must either bet or check
  Blind,

  // If one player had a larger bet than the others, then all other players must either match or fold
  Matching,

  //Community Cards are shown and same betting mechanics are applied to this round
  Flop,

  // same mechanics as Matching Round (1)
  Matching2,

  // players choose a word to play
  Choice,
}
