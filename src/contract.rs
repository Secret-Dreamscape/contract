use crate::contract::HandleMsg::PutDownCard;
use crate::game_state::{Card, GameBoard, Player, State};
use cosmwasm_std::{
  to_binary, Api, BankMsg, Coin, CosmosMsg, Env, Extern, HandleResponse, HandleResult, HumanAddr,
  InitResponse, InitResult, Querier, QueryResult, StdError, StdResult, Storage, Uint128,
};
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
      turn: 0,
      turn_ended: false,
      winner_for_turn: None,
      direction: false,
      cards: (None, None),
    },
  };

  state.save(&mut deps.storage)?;

  Ok(InitResponse::default())
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
  Join { secret: u128 },
  GetHand {},
  GetBoard {},
  //  Bet {},
  PutDownCard { index: usize },
}

fn generate_deck(mut rng: ChaCha20Rng) -> Vec<Card> {
  let deck_size: u8 = 40;
  let mut deck: Vec<Card> = vec![];
  for _i in 0..deck_size {
    deck.push(Card {
      value: (rng.next_u32() % 26) as u8,
      gold: (rng.next_u32() % 2) == 0,
    })
  }
  return deck;
}

fn get_rng(state: &State, env: &Env) -> ChaCha20Rng {
  let mut combined_secret: Vec<u8> = env.block.time.to_be_bytes().to_vec();
  if !state.player1.is_none() {
    combined_secret.extend(state.player1.as_ref().unwrap().secret.to_be_bytes());
  }
  if !state.player2.is_none() {
    combined_secret.extend(state.player2.as_ref().unwrap().secret.to_be_bytes());
  }
  let random_seed: [u8; 32] = Sha256::digest(&combined_secret).into();
  return ChaCha20Rng::from_seed(random_seed);
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
        || env.message.sent_funds[0].denom != String::from("uscrt")
      {
        return Err(StdError::generic_err(
          "Must deposit 1 SCRT to enter the game.",
        ));
      }

      let mut state: State = State::load(&deps.storage)?;

      if state.player1.is_none() {
        state.player1 = Some(Player {
          addr: env.message.sender.clone(),
          secret,
          deck: generate_deck(get_rng(&state, &env)),
          hp: 5,
        });

        state.save(&mut deps.storage)?;
        Ok(HandleResponse::default())
      } else if state.player2.is_none() {
        state.player2 = Some(Player {
          addr: env.message.sender.clone(),
          secret,
          deck: generate_deck(get_rng(&state, &env)),
          hp: 5,
        });

        state.save(&mut deps.storage)?;

        Ok(HandleResponse::default())
      } else {
        Err(StdError::generic_err("Game is full."))
      }
    }
    HandleMsg::GetHand {} => {
      let requester = get_requesting_player(&deps, env);
      if requester.is_none() {
        return Err(StdError::unauthorized());
      }
      let hand: Vec<Card> = (&requester.unwrap().deck[0..4]).to_vec();

      Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&PlayerHand { cards: hand }).unwrap()),
      })
    }
    HandleMsg::PutDownCard { index } => {
      if index > 4 {
        return Err(StdError::generic_err(
          "You cannot place a card that's not in your hand",
        ));
      }
      let mut state: State = State::load(&deps.storage)?;
      require_both_players(&mut state)?;
      let request_attempt = get_requesting_player(&deps, env);
      if request_attempt.is_none() {
        return Err(StdError::unauthorized());
      }
      let requester: Player = request_attempt.unwrap();
      let card: Card = requester.deck[index].clone();
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

      state.save(&mut deps.storage)?;

      return Ok(HandleResponse::default());
    }
    HandleMsg::GetBoard {} => {
      let mut state: State = State::load(&deps.storage)?;
      require_both_players(&mut state)?;
      let request_attempt = get_requesting_player(&deps, env);
      if request_attempt.is_none() {
        return Err(StdError::unauthorized());
      }
      let player1 = state.player1.unwrap();
      let player2 = state.player2.unwrap();
      let requester: Player = request_attempt.unwrap();
      let direction: bool = state.game_board.direction;
      return match state.game_board.cards {
        (None, None) => show_game_board((None, None), direction),
        (Some(card), None) => {
          if requester.addr != player1.addr {
            return show_game_board(
              (
                Some(CardView {
                  card: None,
                  visible: false,
                }),
                None,
              ),
              direction,
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
          )
        }
        (None, Some(card)) => {
          if requester.addr != player2.addr {
            return show_game_board(
              (
                None,
                Some(CardView {
                  card: None,
                  visible: false,
                }),
              ),
              direction,
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
          )
        }
        (Some(card1), Some(card2)) => {
          // calculate who wins
          if !state.game_board.turn_ended {
            // TODO: this code sucks, but I don't understand rust enough to fix this yet...
            // I can't refactor the repeated check into a variable or even a lambda because it complains
            // that I'm using a variable that's been moved, whatever the heck that means and there is seemingly no way to fix that
            let winner = match (card1.gold, card2.gold) {
              (false, false) => {
                if card1.value == card2.value {
                  None
                } else if card1.value > card2.value {
                  if direction {
                    Some(player1)
                  } else {
                    Some(player2)
                  }
                } else {
                  if direction {
                    Some(player2)
                  } else {
                    Some(player1)
                  }
                }
              }
              (true, false) => Some(player1),
              (false, true) => Some(player2),
              (true, true) => {
                if card1.value == card2.value {
                  None
                } else if card1.value > card2.value {
                  if direction {
                    Some(player1)
                  } else {
                    Some(player2)
                  }
                } else {
                  if direction {
                    Some(player2)
                  } else {
                    Some(player1)
                  }
                }
              }
            };

            state.game_board.turn_ended = true;

            match winner {
              None => {}
              Some(winner) => {
                if winner.addr == player1.addr {
                  let mut newplayer2 = player2.clone();
                  newplayer2.hp -= 1;
                  state.player2 = Some(newplayer2);
                } else {
                  let mut newplayer1 = player1.clone();
                  newplayer1.hp -= 1;
                  state.player1 = Some(newplayer1);
                }
                state.game_board.winner_for_turn = Some(winner.addr);
              }
            }
          }

          show_game_board(
            (
              Some(CardView {
                card: Some(card1),
                visible: true,
              }),
              Some(CardView {
                card: Some(card2),
                visible: true,
              }),
            ),
            direction,
          )
        }
      };
    } //    HandleMsg::Bet {} => {}
  }
}

fn require_both_players(state: &mut State) -> StdResult<bool> {
  if state.player1.is_none() || state.player2.is_none() {
    return Err(StdError::unauthorized());
  }
  return Ok(true);
}

fn show_game_board(cards: (Option<CardView>, Option<CardView>), direction: bool) -> HandleResult {
  return Ok(HandleResponse {
    messages: vec![],
    log: vec![],
    data: Some(to_binary(&GameBoardView { cards, direction }).unwrap()),
  });
}

fn get_requesting_player<S: Storage, A: Api, Q: Querier>(
  deps: &&mut Extern<S, A, Q>,
  env: Env,
) -> Option<Player> {
  let state: State = State::load(&deps.storage).ok()?;
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
  return requester;
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
struct PlayerHand {
  cards: Vec<Card>,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
struct GameBoardView {
  cards: (Option<CardView>, Option<CardView>),
  direction: bool,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
struct CardView {
  card: Option<Card>,
  visible: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
  GetResult {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
struct Result {
  winner: HumanAddr,
}

pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
  match msg {
    QueryMsg::GetResult {} => {
      let state = State::load(&deps.storage)?;

      if state.winner.is_none() {
        return Err(StdError::generic_err("Still waiting for players."));
      }

      return Ok(to_binary(&Result {
        winner: state.winner.unwrap(),
      })?);
    }
  }
}
