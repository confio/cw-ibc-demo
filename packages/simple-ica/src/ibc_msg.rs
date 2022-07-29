use cosmwasm_std::{Coin, ContractResult, CosmosMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// This is the message we send over the IBC channel
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PacketMsg {
    Dispatch { msgs: Vec<CosmosMsg> },
    WhoAmI {},
    Balances {},
}

// /// This is a generic ICS acknowledgement format.
// /// Proto defined here: https://github.com/cosmos/cosmos-sdk/blob/v0.42.0/proto/ibc/core/channel/v1/channel.proto#L141-L147
// /// This is compatible with the JSON serialization
// #[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
// #[serde(rename_all = "snake_case")]
// pub enum Ics20Ack {
//     Result(Binary),
//     Error(String),
// }
//
// // create a serialized success message
// fn ack_success() -> Binary {
//     let res = Ics20Ack::Result(b"1".into());
//     to_binary(&res).unwrap()
// }
//
// // create a serialized error message
// fn ack_fail(err: String) -> Binary {
//     let res = Ics20Ack::Error(err);
//     to_binary(&res).unwrap()
// }

/// All IBC acknowledgements are wrapped in `ContractResult`.
/// The success value depends on the PacketMsg variant.
pub type AcknowledgementMsg<T> = ContractResult<T>;

/// This is the success response we send on ack for PacketMsg::Dispatch.
/// Just acknowledge success or error
pub type DispatchResponse = ();

/// This is the success response we send on ack for PacketMsg::WhoAmI.
/// Return the caller's account address on the remote chain
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WhoAmIResponse {
    pub account: String,
}

/// This is the success response we send on ack for PacketMsg::Balance.
/// Just acknowledge success or error
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BalancesResponse {
    pub account: String,
    pub balances: Vec<Coin>,
}
