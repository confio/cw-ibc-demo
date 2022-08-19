use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, IbcPacketAckMsg};
use cw_storage_plus::{Item, Map};

pub const CONFIG: Item<Config> = Item::new("config");
pub const QUERY_RESULT: Map<&str, IbcPacketAckMsg> = Map::new("query_result");
pub const MESSAGE_RESULT: Map<&str, IbcPacketAckMsg> = Map::new("message_result");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub admin: Addr,
    pub simple_ica_controller: Addr,
}
