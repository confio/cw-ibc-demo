#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, QueryRequest,
    Response, StdResult, WasmMsg,
};

use cw2::set_contract_version;
use simple_ica::ReceiveIcaResponseMsg;

use crate::error::ContractError;
use crate::msg::{AdminResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ResultResponse};
use crate::state::{Config, CONFIG, RESULTS};

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
        ExecuteMsg::ReceiveIcaResponse(resp) => execute_receive_ibc_response(deps, env, info, resp),
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
        msg: to_json_binary(&ica_msg)?,
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
    msgs: Vec<QueryRequest<Empty>>,
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
        msg: to_json_binary(&ica_msg)?,
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
        msg: to_json_binary(&ica_msg)?,
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
        msg: to_json_binary(&ica_msg)?,
        funds: info.funds,
    };

    let res = Response::new().add_message(msg);
    Ok(res)
}

pub fn execute_receive_ibc_response(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    resp: ReceiveIcaResponseMsg,
) -> Result<Response, ContractError> {
    // only the simple ica controller can send this message as callback
    let cfg = CONFIG.load(deps.storage)?;
    if !cfg.simple_ica_controller.eq(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    RESULTS.save(deps.storage, &resp.id, &resp.msg)?;

    let res = Response::new()
        .add_attribute("action", "receive_callback")
        .add_attribute("id", resp.id);
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Admin {} => to_json_binary(&query_admin(deps)?),
        QueryMsg::Result { id } => to_json_binary(&query_result(deps, id)?),
    }
}

pub fn query_admin(deps: Deps) -> StdResult<AdminResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(AdminResponse {
        admin: cfg.admin.into(),
    })
}

pub fn query_result(deps: Deps, id: String) -> StdResult<ResultResponse> {
    let result = RESULTS.load(deps.storage, &id)?;
    Ok(ResultResponse { result })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::{coins, BankMsg, BankQuery, SubMsg, WasmMsg};
    use simple_ica::{IbcQueryResponse, StdAck};

    #[test]
    fn send_message_enforces_permissions() {
        let mut deps = mock_dependencies();

        let alice = deps.api.addr_make("alice");
        let bob = deps.api.addr_make("bob");
        let carl = deps.api.addr_make("carl");
        let ica = deps.api.addr_make("simple_ica");
        let channel = "channel-23";

        // instantiate the contract
        let instantiate_msg = InstantiateMsg {
            simple_ica_controller: ica.to_string(),
        };
        let info = message_info(&alice, &[]);
        instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

        // try to send without permissions
        let msgs = vec![BankMsg::Send {
            to_address: carl.to_string(),
            amount: coins(10000, "DAI"),
        }
        .into()];
        let execute_msg = ExecuteMsg::SendMsgs {
            channel_id: channel.to_string(),
            msgs: msgs.clone(),
            callback_id: "test".to_string(),
        };

        // bob cannot execute them
        let info = message_info(&bob, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, execute_msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // but alice can (original owner)
        let info = message_info(&alice, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, execute_msg).unwrap();
        let expected = vec![SubMsg::new(WasmMsg::Execute {
            contract_addr: ica.to_string(),
            msg: to_json_binary(&simple_ica_controller::msg::ExecuteMsg::SendMsgs {
                channel_id: channel.to_string(),
                msgs,
                callback_id: Some("test".to_string()),
            })
            .unwrap(),
            funds: vec![],
        })];
        assert_eq!(res.messages, expected);
    }

    #[test]
    fn query_and_callback_work() {
        let mut deps = mock_dependencies();

        let alice = deps.api.addr_make("alice");
        let bob = deps.api.addr_make("bob");
        let ica = deps.api.addr_make("simple_ica");
        let channel = "channel-23";
        let callback = "my-balance";

        // instantiate the contract
        let instantiate_msg = InstantiateMsg {
            simple_ica_controller: ica.to_string(),
        };
        let info = message_info(&alice, &[]);
        instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

        // try to send without permissions
        let queries = vec![BankQuery::Balance {
            address: bob.to_string(),
            denom: "ujuno".to_string(),
        }
        .into()];
        let execute_msg = ExecuteMsg::IbcQuery {
            channel_id: channel.to_string(),
            msgs: queries.clone(),
            callback_id: callback.to_string(),
        };

        // alice can execute
        let info = message_info(&alice, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, execute_msg).unwrap();
        let expected = vec![SubMsg::new(WasmMsg::Execute {
            contract_addr: ica.to_string(),
            msg: to_json_binary(&simple_ica_controller::msg::ExecuteMsg::IbcQuery {
                channel_id: channel.to_string(),
                msgs: queries,
                callback_id: Some(callback.to_string()),
            })
            .unwrap(),
            funds: vec![],
        })];
        assert_eq!(res.messages, expected);

        // we get a callback
        let ack = StdAck::Result(
            to_json_binary(&IbcQueryResponse {
                results: vec![b"{}".into()],
            })
            .unwrap(),
        );
        let info = message_info(&ica, &[]);
        let msg = ExecuteMsg::ReceiveIcaResponse(ReceiveIcaResponseMsg {
            id: callback.to_string(),
            msg: ack.clone(),
        });
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // now make sure we can query this
        let data = query_result(deps.as_ref(), callback.to_string()).unwrap();
        assert_eq!(data.result, ack);
        // and show how to parse those results
        let result: IbcQueryResponse = data.result.unwrap_into();
        assert_eq!(result.results, vec![Binary::from(b"{}")]);
    }
}
