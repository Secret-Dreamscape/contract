use cosmwasm_std::{HumanAddr, StdResult, Storage};
use cosmwasm_storage::{ReadonlySingleton, Singleton};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct State {
  pub(crate) player1: Option<Player>,
  pub(crate) player2: Option<Player>,

  pub(crate) game_board: GameBoard,

  pub(crate) winner: Option<HumanAddr>,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Player {
  pub(crate) addr: HumanAddr,
  pub(crate) secret: u128,
  pub(crate) deck: Vec<Card>,
  pub(crate) hp: u8,
  pub(crate) deposit: u128,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub(crate) struct Card {
  pub(crate) value: u8,
  pub(crate) gold: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct GameBoard {
  pub(crate) turn: u8,
  pub(crate) turn_ended: bool,
  pub(crate) winner_for_turn: Option<HumanAddr>,
  pub(crate) direction: bool,
  pub(crate) cards: (Option<Card>, Option<Card>),
  pub(crate) pool: u128,
}

impl State {
  pub fn save<S: Storage>(&self, storage: &mut S) -> StdResult<()> {
    Singleton::new(storage, b"state").save(self)
  }

  pub fn load<S: Storage>(storage: &S) -> StdResult<State> {
    ReadonlySingleton::new(storage, b"state").load()
  }
}
