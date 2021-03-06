mod constants;
pub mod contract;
pub mod game_state;
pub mod query;
pub mod utils;

#[cfg(target_arch = "wasm32")]
mod wasm {
  use cosmwasm_std::{do_handle, do_init, do_query, ExternalApi, ExternalQuerier, ExternalStorage};

  use super::contract;
  use super::query;

  #[no_mangle]
  extern "C" fn init(env_ptr: u32, msg_ptr: u32) -> u32 {
    do_init(
      &contract::init::<ExternalStorage, ExternalApi, ExternalQuerier>,
      env_ptr,
      msg_ptr,
    )
  }

  #[no_mangle]
  extern "C" fn handle(env_ptr: u32, msg_ptr: u32) -> u32 {
    do_handle(
      &contract::handle::<ExternalStorage, ExternalApi, ExternalQuerier>,
      env_ptr,
      msg_ptr,
    )
  }

  #[no_mangle]
  extern "C" fn query(msg_ptr: u32) -> u32 {
    do_query(
      &query::query::<ExternalStorage, ExternalApi, ExternalQuerier>,
      msg_ptr,
    )
  }

  // Other C externs like cosmwasm_vm_version_1, allocate, deallocate are available
  // automatically because we `use cosmwasm_std`.
}
