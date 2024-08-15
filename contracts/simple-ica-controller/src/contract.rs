#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, CosmosMsg, Deps, DepsMut, Empty, Env, IbcMsg, MessageInfo, Order, QueryRequest,
    QueryResponse, Response, StdError, StdResult,
};

use simple_ica::PacketMsg;

use crate::ibc::PACKET_LIFETIME;
use crate::msg::{
    AccountInfo, AccountResponse, AdminResponse, ExecuteMsg, InstantiateMsg, LatestQueryResponse,
    ListAccountsResponse, QueryMsg,
};
use crate::state::{Config, ACCOUNTS, CONFIG, LATEST_QUERIES};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    let cfg = Config { admin: info.sender };
    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateAdmin { admin } => execute_update_admin(deps, info, admin),
        ExecuteMsg::SendMsgs {
            channel_id,
            msgs,
            callback_id,
        } => execute_send_msgs(deps, env, info, channel_id, msgs, callback_id),
        ExecuteMsg::CheckRemoteBalance { channel_id } => {
            execute_check_remote_balance(deps, env, info, channel_id)
        }
        ExecuteMsg::IbcQuery {
            channel_id,
            msgs,
            callback_id,
        } => execute_ibc_query(deps, env, info, channel_id, msgs, callback_id),
        ExecuteMsg::SendFunds {
            ica_channel_id,
            transfer_channel_id,
        } => execute_send_funds(deps, env, info, ica_channel_id, transfer_channel_id),
    }
}

pub fn execute_update_admin(
    deps: DepsMut,
    info: MessageInfo,
    new_admin: String,
) -> StdResult<Response> {
    // auth check
    let mut cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.admin {
        return Err(StdError::generic_err("Only admin may set new admin"));
    }
    cfg.admin = deps.api.addr_validate(&new_admin)?;
    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::new()
        .add_attribute("action", "handle_update_admin")
        .add_attribute("new_admin", cfg.admin))
}

pub fn execute_send_msgs(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: String,
    msgs: Vec<CosmosMsg>,
    callback_id: Option<String>,
) -> StdResult<Response> {
    // auth check
    let cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.admin {
        return Err(StdError::generic_err("Only admin may send messages"));
    }
    // ensure the channel exists (not found if not registered)
    ACCOUNTS.load(deps.storage, &channel_id)?;

    // construct a packet to send
    let sender = info.sender.into();
    let packet = PacketMsg::Dispatch {
        sender,
        msgs,
        callback_id,
    };
    let msg = IbcMsg::SendPacket {
        channel_id,
        data: to_json_binary(&packet)?,
        timeout: env.block.time.plus_seconds(PACKET_LIFETIME).into(),
    };

    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "handle_send_msgs");
    Ok(res)
}

pub fn execute_ibc_query(
    _deps: DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: String,
    msgs: Vec<QueryRequest<Empty>>,
    callback_id: Option<String>,
) -> StdResult<Response> {
    // construct a packet to send
    let sender = info.sender.into();
    let packet = PacketMsg::IbcQuery {
        sender,
        msgs,
        callback_id,
    };
    let msg = IbcMsg::SendPacket {
        channel_id,
        data: to_json_binary(&packet)?,
        timeout: env.block.time.plus_seconds(PACKET_LIFETIME).into(),
    };

    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "handle_check_remote_balance");
    Ok(res)
}

pub fn execute_check_remote_balance(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: String,
) -> StdResult<Response> {
    // auth check
    let cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.admin {
        return Err(StdError::generic_err("Only admin may send messages"));
    }
    // ensure the channel exists (not found if not registered)
    ACCOUNTS.load(deps.storage, &channel_id)?;

    // construct a packet to send
    let packet = PacketMsg::Balances {};
    let msg = IbcMsg::SendPacket {
        channel_id,
        data: to_json_binary(&packet)?,
        timeout: env.block.time.plus_seconds(PACKET_LIFETIME).into(),
    };

    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "handle_check_remote_balance");
    Ok(res)
}

pub fn execute_send_funds(
    deps: DepsMut,
    env: Env,
    mut info: MessageInfo,
    ica_channel_id: String,
    transfer_channel_id: String,
) -> StdResult<Response> {
    // intentionally no auth check

    // require some funds
    let amount = match info.funds.pop() {
        Some(coin) => coin,
        None => {
            return Err(StdError::generic_err(
                "you must send the coins you wish to ibc transfer",
            ))
        }
    };
    // if there are any more coins, reject the message
    if !info.funds.is_empty() {
        return Err(StdError::generic_err("you can only ibc transfer one coin"));
    }

    // load remote account
    let data = ACCOUNTS.load(deps.storage, &ica_channel_id)?;
    let remote_addr = match data.remote_addr {
        Some(addr) => addr,
        None => {
            return Err(StdError::generic_err(
                "We don't have the remote address for this channel",
            ))
        }
    };

    // construct a packet to send
    let msg = IbcMsg::Transfer {
        channel_id: transfer_channel_id,
        to_address: remote_addr,
        amount,
        timeout: env.block.time.plus_seconds(PACKET_LIFETIME).into(),
        memo: None,
    };

    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "handle_send_funds");
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    match msg {
        QueryMsg::Admin {} => to_json_binary(&query_admin(deps)?),
        QueryMsg::Account { channel_id } => to_json_binary(&query_account(deps, channel_id)?),
        QueryMsg::ListAccounts {} => to_json_binary(&query_list_accounts(deps)?),
        QueryMsg::LatestQueryResult { channel_id } => {
            to_json_binary(&query_latest_ibc_query_result(deps, channel_id)?)
        }
    }
}

fn query_account(deps: Deps, channel_id: String) -> StdResult<AccountResponse> {
    let account = ACCOUNTS.load(deps.storage, &channel_id)?;
    Ok(account.into())
}

fn query_latest_ibc_query_result(deps: Deps, channel_id: String) -> StdResult<LatestQueryResponse> {
    LATEST_QUERIES.load(deps.storage, &channel_id)
}

fn query_list_accounts(deps: Deps) -> StdResult<ListAccountsResponse> {
    let accounts = ACCOUNTS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| {
            let (channel_id, account) = r?;
            Ok(AccountInfo::convert(channel_id, account))
        })
        .collect::<StdResult<_>>()?;
    Ok(ListAccountsResponse { accounts })
}

fn query_admin(deps: Deps) -> StdResult<AdminResponse> {
    let Config { admin } = CONFIG.load(deps.storage)?;
    Ok(AdminResponse {
        admin: admin.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};

    const CREATOR: &str = "creator";

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();
        let creator = deps.api.addr_make(CREATOR);
        let msg = InstantiateMsg {};
        let info = message_info(&creator, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let admin = query_admin(deps.as_ref()).unwrap();
        assert_eq!(creator.to_string(), admin.admin);
    }
}
