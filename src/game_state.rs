use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub(crate) struct State {
  pub(crate) players: Vec<Player>,
  pub(crate) can_join: bool,

  pub(crate) game_board: GameBoard,
  pub(crate) winner: Option<HumanAddr>,
  pub(crate) deck: Vec<Card>,
  pub(crate) started_time: u64,
  pub(crate) level_design: u64,
  pub(crate) password: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub(crate) struct Player {
  pub(crate) addr: HumanAddr,
  pub(crate) secret: u64,
  pub(crate) hp: u8,
  pub(crate) hand: Vec<Card>,
  pub(crate) bet: u64,
  pub(crate) bet2: u64,
  pub(crate) folded: bool,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub(crate) struct Card {
  pub(crate) letter: u8,
  pub(crate) gold: bool,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub(crate) struct GameBoard {
  pub(crate) turn: u8,
  pub(crate) round: GameRound,
  pub(crate) winner_for_turn: Option<HumanAddr>,
  pub(crate) words: Vec<Word>,
  pub(crate) river: Vec<Card>,
  pub(crate) pool: u64,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub(crate) struct Word {
  pub(crate) cards: Vec<Card>,
  pub(crate) player_addr: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, PartialEq)]
pub(crate) enum GameRound {
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

#[derive(Serialize, Deserialize, Clone, JsonSchema, PartialEq)]
pub(crate) enum GameRoundFamily {
  // turn not started yet, initial state
  // Family games are minimum stakes (play for fun) - don't allow dictionary
  None,

  // Community cards are not shown and every player must either bet or check
  PreFlop,

  // If one player had a larger bet than the others, then all other players must either match or fold
  Matching,

  // 1st 3 Community Cards are shown and same betting mechanics are applied to this round
  Flop,

  // same mechanics as Matching Round (1)
  Matching2,

  // 4th community card is shown
  Turn, 

  // same mechanics as Matching Round (1)
  Matching3,

  // 5th community card is shown
  River,

  // same mechanics as Matching Round (1)
  Matching4,

  // players choose a word to play
  Choice,
}