use crate::game_state::{Player, State};

pub fn get_non_folded_players(state: &State) -> Vec<Player> {
  let mut players = vec![];
  for i in 0..state.players.len() {
    if !state.players[i].folded {
      players.push(state.players[i].clone());
    }
  }
  players
}
