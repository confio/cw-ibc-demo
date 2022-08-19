use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use simple_ica::StdAck;

pub const CONFIG: Item<Config> = Item::new("config");
pub const RESULTS: Map<&str, StdAck> = Map::new("results");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub admin: Addr,
    pub simple_ica_controller: Addr,
}
