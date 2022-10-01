use cosmwasm_std::{from_slice, to_binary, Binary, Coin, CosmosMsg, Empty, QueryRequest};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use osmo_bindings::{OsmosisMsg, OsmosisQuery};

/// This is the message we send over the IBC channel
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PacketMsg {
    Dispatch {
        sender: String,
        msgs: Vec<CosmosMsg<OsmosisMsg>>,
        callback_id: Option<String>,
    },
    IbcQuery {
        sender: String,
        msgs: Vec<QueryRequest<OsmosisQuery>>,
        callback_id: Option<String>,
    },
    WhoAmI {},
    Balances {},
}