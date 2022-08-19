use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin, IbcPacketAckMsg, Timestamp};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    pub admin: Addr,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
pub struct AccountData {
    /// last block balance was updated (0 is never)
    pub last_update_time: Timestamp,
    /// In normal cases, it should be set, but there is a delay between binding
    /// the channel and making a query and in that time it is empty.
    ///
    /// Since we do not have a way to validate the remote address format, this
    /// must not be of type `Addr`.
    pub remote_addr: Option<String>,
    pub remote_balance: Vec<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct IbcQueryResponse {
    /// last block balance was updated (0 is never)
    pub last_update_time: Timestamp,
    pub response: IbcPacketAckMsg,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const ACCOUNTS: Map<&str, AccountData> = Map::new("accounts");
pub const LATEST_QUERIES: Map<&str, IbcQueryResponse> = Map::new("querys");
