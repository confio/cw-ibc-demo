mod ibc_msg;

pub use crate::ibc_msg::{
    AcknowledgementMsg, BalancesResponse, DispatchResponse, PacketMsg, WhoAmIResponse,
};

pub const IBC_APP_VERSION: &str = "simple-ica-v1";
