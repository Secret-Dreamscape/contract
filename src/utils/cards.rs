use cosmwasm_std::Env;
use lazy_static::lazy_static;
use rand::{seq::SliceRandom, SeedableRng};
use rand_chacha::ChaChaRng;
use sha2::{Digest, Sha256};

use crate::game_state::{Card, State};

lazy_static! {
  static ref ALLOWED_WORDS: Vec<&'static str> = {
    let word_txt: &str = include_str!("../words.txt");
    word_txt.split('\n').collect()
  };
}

pub fn get_score_for_word(cards: &[Card]) -> u16 {
  let card_to_point: Vec<u16> = vec![
    1, 3, 3, 2, 1, 4, 2, 4, 1, 8, 5, 1, 3, 1, 1, 3, 10, 1, 1, 1, 1, 4, 4, 8, 4, 8,
  ];
  let mut score: u16 = 0;
  let mut golds = 0;
  let mut word = String::from("");
  for card in cards {
    if card.gold {
      golds += 1;
    }
    score += card_to_point.get(card.letter as usize).unwrap();
    word.push((card.letter + b'A') as char);
  }
  for _ in 0..golds {
    score *= 2;
  }
  if !ALLOWED_WORDS.contains(&&*word) {
    return 0;
  }
  score
}

pub fn get_n_cards(state: &mut State, count: u8) -> Vec<Card> {
  let mut cards = vec![];
  for _ in 0..count {
    cards.push(state.deck.swap_remove(0))
  }
  cards
}

pub fn generate_deck(mut rng: ChaChaRng) -> Vec<Card> {
  let count_per_card: Vec<(u8, u8)> = vec![
    (4, 12),
    (0, 9),
    (8, 9),
    (14, 8),
    (13, 6),
    (17, 6),
    (19, 6),
    (11, 4),
    (18, 4),
    (20, 4),
    (3, 4),
    (6, 3),
    (1, 2),
    (2, 2),
    (12, 2),
    (15, 2),
    (5, 2),
    (7, 2),
    (21, 2),
    (22, 2),
    (24, 2),
    (10, 1),
    (9, 1),
    (23, 1),
    (16, 1),
    (25, 1),
  ];

  let mut deck: Vec<Card> = vec![];
  for (_, (letter, count)) in count_per_card.iter().enumerate() {
    for _ in 0..*count {
      deck.push(Card {
        letter: *letter,
        gold: false,
      })
    }
  }
  deck.shuffle(&mut rng);
  for i in 0..5 {
    // we set the first 5 cards to be golden
    deck[i].gold = true;
  }
  deck.shuffle(&mut rng); // and shuffle the deck again to move the gold cards around
  deck
}

pub fn get_rng(state: &State, env: &Env) -> ChaChaRng {
  let mut combined_secret: Vec<u8> = env.block.time.to_be_bytes().to_vec();
  for player in state.players.iter() {
    combined_secret.extend(&player.secret.to_be_bytes());
  }
  let random_seed: [u8; 32] = Sha256::digest(&combined_secret).into();
  ChaChaRng::from_seed(random_seed)
}
