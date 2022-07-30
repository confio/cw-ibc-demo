use cosmwasm_std::CosmosMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Just needs to know the code_id of a reflect contract to spawn sub-accounts
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub cw1_code_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns (reflect) account that is attached to this channel,
    /// or none.
    Account { channel_id: String },
    /// Returns all (channel, reflect_account) pairs.
    /// No pagination - this is a test contract
    ListAccounts {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AccountResponse {
    pub account: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ListAccountsResponse {
    pub accounts: Vec<AccountInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AccountInfo {
    pub account: String,
    pub channel_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReflectExecuteMsg {
    ReflectMsg { msgs: Vec<CosmosMsg> },
}
