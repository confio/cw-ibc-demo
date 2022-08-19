use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CosmosMsg, Empty, IbcPacketAckMsg, WasmQuery};
use simple_ica::ReceiveIbcResponseMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub simple_ica_controller: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    SendMsgs {
        channel_id: String,
        /// Note: we don't handle custom messages on remote chains
        msgs: Vec<CosmosMsg<Empty>>,
        /// We store the the result under this id
        callback_id: String,
    },
    CheckRemoteBalance {
        channel_id: String,
    },
    IbcQuery {
        channel_id: String,
        msgs: Vec<WasmQuery>,
        /// We store the the result under this id
        callback_id: String,
    },
    /// If you sent funds to this contract, it will attempt to ibc transfer them
    /// to the account on the remote side of this channel.
    /// If we don't have the address yet, this fails.
    SendFunds {
        /// The channel id we use above to send the simple-ica query on
        ica_channel_id: String,
        /// The channel to use for ibctransfer. This is bound to a different
        /// port and handled by a different module.
        /// It should connect to the same chain as the ica_channel_id does
        transfer_channel_id: String,
    },
    ReceiveIbcResponse(ReceiveIbcResponseMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Shows admin
    Admin {},
    // Get result for the given callback id
    QueryResult {
        id: String,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AdminResponse {
    pub admin: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryResultResponse {
    pub query: IbcPacketAckMsg,
}
