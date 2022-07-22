use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub reflect_code_id: u64,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const PENDING: Item<String> = Item::new("pending");
pub const ACCOUNTS: Map<&str, Addr> = Map::new("accounts");
