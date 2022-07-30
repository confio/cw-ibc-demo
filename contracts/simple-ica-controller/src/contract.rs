use cosmwasm_std::{
    entry_point, to_binary, CosmosMsg, Deps, DepsMut, Env, IbcMsg, MessageInfo, Order,
    QueryResponse, Response, StdError, StdResult, WasmQuery,
};
use simple_ica::PacketMsg;

use crate::ibc::PACKET_LIFETIME;
use crate::msg::{
    AccountInfo, AccountResponse, AdminResponse, ExecuteMsg, InstantiateMsg, ListAccountsResponse,
    QueryMsg,
};
use crate::state::{Config, ACCOUNTS, CONFIG};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    // we store the reflect_id for creating accounts later
    let cfg = Config { admin: info.sender };
    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateAdmin { admin } => execute_update_admin(deps, info, admin),
        ExecuteMsg::SendMsgs {
            channel_id,
            msgs,
            callback,
        } => execute_send_msgs(deps, env, info, channel_id, msgs, callback),
        ExecuteMsg::CheckRemoteBalance { channel_id } => {
            execute_check_remote_balance(deps, env, info, channel_id)
        }
        ExecuteMsg::IbcQuery {
            channel_id,
            msgs,
            callback,
        } => execute_ibc_query(deps, env, info, channel_id, msgs, callback),
        ExecuteMsg::SendFunds {
            reflect_channel_id,
            transfer_channel_id,
        } => execute_send_funds(deps, env, info, reflect_channel_id, transfer_channel_id),
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
    callback: Option<String>,
) -> StdResult<Response> {
    // auth check
    let cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.admin {
        return Err(StdError::generic_err("Only admin may send messages"));
    }
    // ensure the channel exists (not found if not registered)
    ACCOUNTS.load(deps.storage, &channel_id)?;

    // construct a packet to send
    let packet = PacketMsg::Dispatch { msgs, callback };
    let msg = IbcMsg::SendPacket {
        channel_id,
        data: to_binary(&packet)?,
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
    _info: MessageInfo,
    channel_id: String,
    msgs: Vec<WasmQuery>,
    callback: Option<String>,
) -> StdResult<Response> {
    // construct a packet to send
    let packet = PacketMsg::IbcQuery { msgs, callback };
    let msg = IbcMsg::SendPacket {
        channel_id,
        data: to_binary(&packet)?,
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
        data: to_binary(&packet)?,
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
    reflect_channel_id: String,
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
    let data = ACCOUNTS.load(deps.storage, &reflect_channel_id)?;
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
    };

    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "handle_send_funds");
    Ok(res)
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    match msg {
        QueryMsg::Admin {} => to_binary(&query_admin(deps)?),
        QueryMsg::Account { channel_id } => to_binary(&query_account(deps, channel_id)?),
        QueryMsg::ListAccounts {} => to_binary(&query_list_accounts(deps)?),
    }
}

fn query_account(deps: Deps, channel_id: String) -> StdResult<AccountResponse> {
    let account = ACCOUNTS.load(deps.storage, &channel_id)?;
    Ok(account.into())
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
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    const CREATOR: &str = "creator";

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let admin = query_admin(deps.as_ref()).unwrap();
        assert_eq!(CREATOR, admin.admin.as_str());
    }
}
