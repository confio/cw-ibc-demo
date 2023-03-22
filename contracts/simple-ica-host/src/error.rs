use thiserror::Error;

use cosmwasm_std::StdError;
use cw_utils::ParseReplyError;

use simple_ica::SimpleIcaError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    ParseReply(#[from] ParseReplyError),

    #[error("{0}")]
    SimpleIca(#[from] SimpleIcaError),

    #[error("Cannot register over an existing channel")]
    ChannelAlreadyRegistered,

    #[error("Invalid reply id")]
    InvalidReplyId,

    #[error("You cannot call internal messages directly")]
    OnlySelfCall,
}
