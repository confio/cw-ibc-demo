#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
    WasmMsg, WasmQuery,
};

use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    AdminResponse, ExecuteMsg, InstantiateMsg, MessageResultResponse, QueryMsg, QueryResultResponse,
};
use crate::state::{Config, CONFIG, MESSAGE_RESULT, QUERY_RESULT};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:callback-capturer";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let cfg = Config {
        admin: info.sender,
        simple_ica_controller: deps.api.addr_validate(&msg.simple_ica_controller)?,
    };
    CONFIG.save(deps.storage, &cfg)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SendMsgs {
            channel_id,
            msgs,
            callback_id,
        } => execute_send_msgs(deps, env, info, channel_id, msgs, callback_id),
        ExecuteMsg::IbcQuery {
            channel_id,
            msgs,
            callback_id,
        } => execute_ibc_query(deps, env, info, channel_id, msgs, callback_id),
        ExecuteMsg::CheckRemoteBalance { channel_id } => {
            execute_check_remote_balance(deps, env, info, channel_id)
        }
        ExecuteMsg::SendFunds {
            ica_channel_id,
            transfer_channel_id,
        } => execute_send_funds(deps, env, info, ica_channel_id, transfer_channel_id),
    }
}

pub fn execute_send_msgs(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    channel_id: String,
    msgs: Vec<CosmosMsg<Empty>>,
    callback_id: String,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    if !cfg.admin.eq(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let ica_msg = simple_ica_controller::msg::ExecuteMsg::SendMsgs {
        channel_id,
        msgs,
        callback_id: Some(callback_id),
    };
    let msg = WasmMsg::Execute {
        contract_addr: cfg.simple_ica_controller.into(),
        msg: to_binary(&ica_msg)?,
        funds: vec![],
    };

    let res = Response::new().add_message(msg);
    Ok(res)
}

pub fn execute_ibc_query(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    channel_id: String,
    msgs: Vec<WasmQuery>,
    callback_id: String,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    if !cfg.admin.eq(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let ica_msg = simple_ica_controller::msg::ExecuteMsg::IbcQuery {
        channel_id,
        msgs,
        callback_id: Some(callback_id),
    };
    let msg = WasmMsg::Execute {
        contract_addr: cfg.simple_ica_controller.into(),
        msg: to_binary(&ica_msg)?,
        funds: vec![],
    };

    let res = Response::new().add_message(msg);
    Ok(res)
}

pub fn execute_check_remote_balance(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    channel_id: String,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    if !cfg.admin.eq(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let ica_msg = simple_ica_controller::msg::ExecuteMsg::CheckRemoteBalance { channel_id };
    let msg = WasmMsg::Execute {
        contract_addr: cfg.simple_ica_controller.into(),
        msg: to_binary(&ica_msg)?,
        funds: vec![],
    };

    let res = Response::new().add_message(msg);
    Ok(res)
}

pub fn execute_send_funds(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    ica_channel_id: String,
    transfer_channel_id: String,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    if !cfg.admin.eq(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let ica_msg = simple_ica_controller::msg::ExecuteMsg::SendFunds {
        ica_channel_id,
        transfer_channel_id,
    };
    let msg = WasmMsg::Execute {
        contract_addr: cfg.simple_ica_controller.into(),
        msg: to_binary(&ica_msg)?,
        funds: info.funds,
    };

    let res = Response::new().add_message(msg);
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Admin {} => to_binary(&query_admin(deps)?),
        QueryMsg::QueryResult { id } => to_binary(&query_query_result(deps, id)?),
        QueryMsg::MessageResult { id } => to_binary(&query_message_result(deps, id)?),
    }
}

pub fn query_admin(deps: Deps) -> StdResult<AdminResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(AdminResponse {
        admin: cfg.admin.into(),
    })
}

pub fn query_query_result(deps: Deps, id: String) -> StdResult<QueryResultResponse> {
    let query = QUERY_RESULT.load(deps.storage, &id)?;
    Ok(QueryResultResponse { query })
}

pub fn query_message_result(deps: Deps, id: String) -> StdResult<MessageResultResponse> {
    let msg = MESSAGE_RESULT.load(deps.storage, &id)?;
    Ok(MessageResultResponse { msg })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, coins, BankMsg, StakingMsg, SubMsg, WasmMsg};

    #[test]
    fn instantiate_and_modify_config() {
        let mut deps = mock_dependencies();

        let alice = "alice";
        let bob = "bob";
        let carl = "carl";

        let anyone = "anyone";

        // instantiate the contract
        let instantiate_msg = InstantiateMsg {
            admins: vec![alice.to_string(), bob.to_string(), carl.to_string()],
            mutable: true,
        };
        let info = mock_info(anyone, &[]);
        instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

        // ensure expected config
        let expected = AdminListResponse {
            admins: vec![alice.to_string(), bob.to_string(), carl.to_string()],
            mutable: true,
        };
        assert_eq!(query_admin_list(deps.as_ref()).unwrap(), expected);

        // anyone cannot modify the contract
        let msg = ExecuteMsg::UpdateAdmins {
            admins: vec![anyone.to_string()],
        };
        let info = mock_info(anyone, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // but alice can kick out carl
        let msg = ExecuteMsg::UpdateAdmins {
            admins: vec![alice.to_string(), bob.to_string()],
        };
        let info = mock_info(alice, &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // ensure expected config
        let expected = AdminListResponse {
            admins: vec![alice.to_string(), bob.to_string()],
            mutable: true,
        };
        assert_eq!(query_admin_list(deps.as_ref()).unwrap(), expected);

        // carl cannot freeze it
        let info = mock_info(carl, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Freeze {}).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // but bob can
        let info = mock_info(bob, &[]);
        execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Freeze {}).unwrap();
        let expected = AdminListResponse {
            admins: vec![alice.to_string(), bob.to_string()],
            mutable: false,
        };
        assert_eq!(query_admin_list(deps.as_ref()).unwrap(), expected);

        // and now alice cannot change it again
        let msg = ExecuteMsg::UpdateAdmins {
            admins: vec![alice.to_string()],
        };
        let info = mock_info(alice, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn execute_messages_has_proper_permissions() {
        let mut deps = mock_dependencies();

        let alice = "alice";
        let bob = "bob";
        let carl = "carl";

        // instantiate the contract
        let instantiate_msg = InstantiateMsg {
            admins: vec![alice.to_string(), carl.to_string()],
            mutable: false,
        };
        let info = mock_info(bob, &[]);
        instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

        let freeze: ExecuteMsg<Empty> = ExecuteMsg::Freeze {};
        let msgs = vec![
            BankMsg::Send {
                to_address: bob.to_string(),
                amount: coins(10000, "DAI"),
            }
            .into(),
            WasmMsg::Execute {
                contract_addr: "some contract".into(),
                msg: to_binary(&freeze).unwrap(),
                funds: vec![],
            }
            .into(),
        ];

        // make some nice message
        let execute_msg = ExecuteMsg::Execute { msgs: msgs.clone() };

        // bob cannot execute them
        let info = mock_info(bob, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, execute_msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // but carl can
        let info = mock_info(carl, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, execute_msg).unwrap();
        assert_eq!(
            res.messages,
            msgs.into_iter().map(SubMsg::new).collect::<Vec<_>>()
        );
        assert_eq!(res.attributes, [("action", "execute")]);
    }

    #[test]
    fn can_execute_query_works() {
        let mut deps = mock_dependencies();

        let alice = "alice";
        let bob = "bob";

        let anyone = "anyone";

        // instantiate the contract
        let instantiate_msg = InstantiateMsg {
            admins: vec![alice.to_string(), bob.to_string()],
            mutable: false,
        };
        let info = mock_info(anyone, &[]);
        instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

        // let us make some queries... different msg types by owner and by other
        let send_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: anyone.to_string(),
            amount: coins(12345, "ushell"),
        });
        let staking_msg = CosmosMsg::Staking(StakingMsg::Delegate {
            validator: anyone.to_string(),
            amount: coin(70000, "ureef"),
        });

        // owner can send
        let res = query_can_execute(deps.as_ref(), alice.to_string(), send_msg.clone()).unwrap();
        assert!(res.can_execute);

        // owner can stake
        let res = query_can_execute(deps.as_ref(), bob.to_string(), staking_msg.clone()).unwrap();
        assert!(res.can_execute);

        // anyone cannot send
        let res = query_can_execute(deps.as_ref(), anyone.to_string(), send_msg).unwrap();
        assert!(!res.can_execute);

        // anyone cannot stake
        let res = query_can_execute(deps.as_ref(), anyone.to_string(), staking_msg).unwrap();
        assert!(!res.can_execute);
    }
}
