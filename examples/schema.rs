use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use secret_dreamscape::contract::{HandleMsg, InitMsg};
use secret_dreamscape::query::{
  CanJoinResponse, GameState, PlayerStatus, QueryMsg, Result,
};

fn main() {
  let mut out_dir = current_dir().unwrap();
  out_dir.push("schema");
  create_dir_all(&out_dir).unwrap();
  remove_schemas(&out_dir).unwrap();

  export_schema(&schema_for!(InitMsg), &out_dir);
  export_schema(&schema_for!(HandleMsg), &out_dir);
  export_schema(&schema_for!(CanJoinResponse), &out_dir);
  export_schema(&schema_for!(GameState), &out_dir);
  export_schema(&schema_for!(PlayerStatus), &out_dir);
  export_schema(&schema_for!(QueryMsg), &out_dir);
  export_schema(&schema_for!(Result), &out_dir);
}
