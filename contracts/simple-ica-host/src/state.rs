use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Binary};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub cw1_code_id: u64,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const PENDING: Item<String> = Item::new("pending");
pub const ACCOUNTS: Map<&str, Addr> = Map::new("accounts");

// this stores all results from current dispatch
pub const RESULTS: Item<Vec<Binary>> = Item::new("results");
