use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_binary, Binary, CosmosMsg, StdResult, WasmMsg};

use crate::StdAck;

/// ReceiveIbcResponseMsg should be de/serialized under `Receive()` variant in a ExecuteMsg
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct ReceiveIcaResponseMsg {
    /// The ID chosen by the caller in the `callback_id`
    pub id: String,
    pub msg: StdAck,
}

impl ReceiveIcaResponseMsg {
    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = SimpleIcaReceiverExecuteMsg::ReceiveIcaResponse(self);
        to_binary(&msg)
    }

    /// creates a cosmos_msg sending this struct to the named contract
    pub fn into_cosmos_msg<T: Into<String>, C>(self, contract_addr: T) -> StdResult<CosmosMsg<C>>
    where
        C: Clone + std::fmt::Debug + PartialEq + JsonSchema,
    {
        let msg = self.into_binary()?;
        let execute = WasmMsg::Execute {
            contract_addr: contract_addr.into(),
            msg,
            funds: vec![],
        };
        Ok(execute.into())
    }
}

/// This is just a helper to properly serialize the above message.
/// The actual receiver should include this variant in the larger ExecuteMsg enum
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
enum SimpleIcaReceiverExecuteMsg {
    ReceiveIcaResponse(ReceiveIcaResponseMsg),
}
