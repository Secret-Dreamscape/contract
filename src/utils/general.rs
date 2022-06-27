use crate::game_state::{Player, State};

pub fn get_non_folded_players(state: &State) -> Vec<Player> {
  state
    .players
    .iter()
    .filter(|p| !p.folded)
    .cloned()
    .collect()
}
