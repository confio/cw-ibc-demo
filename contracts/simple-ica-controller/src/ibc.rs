#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, DepsMut, Env, Ibc3ChannelOpenResponse, IbcBasicResponse,
    IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcMsg, IbcPacketAckMsg,
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, StdResult,
};

use simple_ica::{
    check_order, check_version, BalancesResponse, PacketMsg, ReceiveIcaResponseMsg, StdAck,
    WhoAmIResponse,
};

use crate::error::ContractError;
use crate::msg::LatestQueryResponse;
use crate::state::{AccountData, ACCOUNTS, LATEST_QUERIES};

// TODO: make configurable?
/// packets live one hour
pub const PACKET_LIFETIME: u64 = 60 * 60;

#[cfg_attr(not(feature = "library"), entry_point)]
/// enforces ordering and versioing constraints
pub fn ibc_channel_open(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<Option<Ibc3ChannelOpenResponse>, ContractError> {
    let channel = msg.channel();
    check_order(&channel.order)?;
    check_version(&channel.version)?;
    if let Some(counter_version) = msg.counterparty_version() {
        check_version(counter_version)?;
    }

    Ok(None)
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// once it's established, we send a WhoAmI message
pub fn ibc_channel_connect(
    deps: DepsMut,
    env: Env,
    msg: IbcChannelConnectMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();
    let channel_id = &channel.endpoint.channel_id;

    // create an account holder the channel exists (not found if not registered)
    let data = AccountData::default();
    ACCOUNTS.save(deps.storage, channel_id, &data)?;

    // construct a packet to send
    let packet = PacketMsg::WhoAmI {};
    let msg = IbcMsg::SendPacket {
        channel_id: channel_id.clone(),
        data: to_json_binary(&packet)?,
        timeout: env.block.time.plus_seconds(PACKET_LIFETIME).into(),
    };

    Ok(IbcBasicResponse::new()
        .add_message(msg)
        .add_attribute("action", "ibc_connect")
        .add_attribute("channel_id", channel_id))
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// On closed channel, simply delete the account from our local store
pub fn ibc_channel_close(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();

    // remove the channel
    let channel_id = &channel.endpoint.channel_id;
    ACCOUNTS.remove(deps.storage, channel_id);

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "ibc_close")
        .add_attribute("channel_id", channel_id))
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// never should be called as the other side never sends packets
pub fn ibc_packet_receive(
    _deps: DepsMut,
    _env: Env,
    _packet: IbcPacketReceiveMsg,
) -> StdResult<IbcReceiveResponse> {
    Ok(IbcReceiveResponse::new(b"{}").add_attribute("action", "ibc_packet_ack"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_ack(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    // which local channel was this packet send from
    let caller = msg.original_packet.src.channel_id.clone();
    // we need to parse the ack based on our request
    let original_packet: PacketMsg = from_json(&msg.original_packet.data)?;
    let res: StdAck = from_json(&msg.acknowledgement.data)?;

    match original_packet {
        PacketMsg::Dispatch {
            sender,
            callback_id,
            ..
        } => acknowledge_dispatch(deps, env, caller, sender, callback_id, msg),
        PacketMsg::IbcQuery {
            sender,
            callback_id,
            ..
        } => acknowledge_query(deps, env, caller, sender, callback_id, msg),
        PacketMsg::WhoAmI {} => acknowledge_who_am_i(deps, caller, res),
        PacketMsg::Balances {} => acknowledge_balances(deps, env, caller, res),
    }
}

// receive PacketMsg::Dispatch response
#[allow(clippy::unnecessary_wraps)]
fn acknowledge_dispatch(
    _deps: DepsMut,
    _env: Env,
    _caller: String,
    sender: String,
    callback_id: Option<String>,
    ack: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    let res = IbcBasicResponse::new().add_attribute("action", "acknowledge_dispatch");
    match callback_id {
        Some(id) => {
            let msg: StdAck = from_json(&ack.acknowledgement.data)?;
            // Send IBC packet ack message to another contract
            let res = res
                .add_attribute("callback_id", &id)
                //  In production, you will want to think about gas limits for this callback.
                .add_message(ReceiveIcaResponseMsg { id, msg }.into_cosmos_msg(sender)?);
            Ok(res)
        }
        None => Ok(res),
    }
}

fn acknowledge_query(
    deps: DepsMut,
    env: Env,
    caller: String,
    sender: String,
    callback_id: Option<String>,
    ack: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    let msg: StdAck = from_json(&ack.acknowledgement.data)?;

    // store IBC response for later querying from the smart contract??
    LATEST_QUERIES.save(
        deps.storage,
        &caller,
        &LatestQueryResponse {
            last_update_time: env.block.time,
            response: msg.clone(),
        },
    )?;
    match callback_id {
        Some(id) => {
            // Send IBC packet ack message to another contract
            let msg = ReceiveIcaResponseMsg { id, msg }.into_cosmos_msg(sender)?;
            Ok(IbcBasicResponse::new()
                .add_attribute("action", "acknowledge_ibc_query")
                .add_message(msg))
        }
        None => Ok(IbcBasicResponse::new().add_attribute("action", "acknowledge_ibc_query")),
    }
}

// receive PacketMsg::WhoAmI response
// store address info in accounts info
fn acknowledge_who_am_i(
    deps: DepsMut,
    caller: String,
    ack: StdAck,
) -> Result<IbcBasicResponse, ContractError> {
    // ignore errors (but mention in log)
    let WhoAmIResponse { account } = match ack {
        StdAck::Result(res) => from_json(&res)?,
        StdAck::Error(e) => {
            return Ok(IbcBasicResponse::new()
                .add_attribute("action", "acknowledge_who_am_i")
                .add_attribute("error", e))
        }
    };

    ACCOUNTS.update(deps.storage, &caller, |acct| {
        match acct {
            Some(mut acct) => {
                // set the account the first time
                if acct.remote_addr.is_none() {
                    acct.remote_addr = Some(account);
                }
                Ok(acct)
            }
            None => Err(ContractError::UnregisteredChannel(caller.clone())),
        }
    })?;

    Ok(IbcBasicResponse::new().add_attribute("action", "acknowledge_who_am_i"))
}

// receive PacketMsg::Balances response
fn acknowledge_balances(
    deps: DepsMut,
    env: Env,
    caller: String,
    ack: StdAck,
) -> Result<IbcBasicResponse, ContractError> {
    // ignore errors (but mention in log)
    let BalancesResponse { account, balances } = match ack {
        StdAck::Result(res) => from_json(&res)?,
        StdAck::Error(e) => {
            return Ok(IbcBasicResponse::new()
                .add_attribute("action", "acknowledge_balances")
                .add_attribute("error", e))
        }
    };

    ACCOUNTS.update(deps.storage, &caller, |acct| match acct {
        Some(acct) => {
            if let Some(old) = acct.remote_addr {
                if old != account {
                    return Err(ContractError::RemoteAccountChanged { old, addr: account });
                }
            }
            Ok(AccountData {
                last_update_time: env.block.time,
                remote_addr: Some(account),
                remote_balance: balances,
            })
        }
        None => Err(ContractError::UnregisteredChannel(caller.clone())),
    })?;

    Ok(IbcBasicResponse::new().add_attribute("action", "acknowledge_balances"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// we just ignore these now. shall we store some info?
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketTimeoutMsg,
) -> StdResult<IbcBasicResponse> {
    Ok(IbcBasicResponse::new().add_attribute("action", "ibc_packet_timeout"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{execute, instantiate, query};
    use crate::msg::{AccountResponse, ExecuteMsg, InstantiateMsg, QueryMsg};

    use cosmwasm_std::testing::{
        message_info, mock_dependencies, mock_env, mock_ibc_channel_connect_ack,
        mock_ibc_channel_open_init, mock_ibc_channel_open_try, mock_ibc_packet_ack, MockApi,
        MockQuerier, MockStorage,
    };
    use cosmwasm_std::{coin, coins, BankMsg, CosmosMsg, IbcAcknowledgement, OwnedDeps};
    use simple_ica::{APP_ORDER, BAD_APP_ORDER, IBC_APP_VERSION};

    const CREATOR: &str = "creator";

    fn setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let creator = deps.api.addr_make(CREATOR);
        let msg = InstantiateMsg {};
        let info = message_info(&creator, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    // connect will run through the entire handshake to set up a proper connect and
    // save the account (tested in detail in `proper_handshake_flow`)
    fn connect(mut deps: DepsMut, channel_id: &str) {
        let handshake_open = mock_ibc_channel_open_init(channel_id, APP_ORDER, IBC_APP_VERSION);
        // first we try to open with a valid handshake
        ibc_channel_open(deps.branch(), mock_env(), handshake_open).unwrap();

        // then we connect (with counter-party version set)
        let handshake_connect =
            mock_ibc_channel_connect_ack(channel_id, APP_ORDER, IBC_APP_VERSION);
        let res = ibc_channel_connect(deps.branch(), mock_env(), handshake_connect).unwrap();

        // this should send a WhoAmI request, which is received some blocks later
        assert_eq!(1, res.messages.len());
        match &res.messages[0].msg {
            CosmosMsg::Ibc(IbcMsg::SendPacket {
                channel_id: packet_channel,
                ..
            }) => assert_eq!(packet_channel.as_str(), channel_id),
            o => panic!("Unexpected message: {:?}", o),
        };
    }

    fn who_am_i_response(deps: DepsMut, channel_id: &str, account: impl Into<String>) {
        let packet = PacketMsg::WhoAmI {};
        let res = StdAck::success(WhoAmIResponse {
            account: account.into(),
        });
        let ack = IbcAcknowledgement::new(res);
        let msg = mock_ibc_packet_ack(channel_id, &packet, ack).unwrap();
        let res = ibc_packet_ack(deps, mock_env(), msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn enforce_version_in_handshake() {
        let mut deps = setup();

        let wrong_order = mock_ibc_channel_open_try("channel-12", BAD_APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), wrong_order).unwrap_err();

        let wrong_version = mock_ibc_channel_open_try("channel-12", APP_ORDER, "reflect");
        ibc_channel_open(deps.as_mut(), mock_env(), wrong_version).unwrap_err();

        let valid_handshake = mock_ibc_channel_open_try("channel-12", APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), valid_handshake).unwrap();
    }

    #[test]
    fn proper_handshake_flow() {
        // setup and connect handshake
        let mut deps = setup();
        let channel_id = "channel-1234";
        connect(deps.as_mut(), channel_id);

        // check for empty account
        let q = QueryMsg::Account {
            channel_id: channel_id.into(),
        };
        let r = query(deps.as_ref(), mock_env(), q).unwrap();
        let acct: AccountResponse = from_json(&r).unwrap();
        assert!(acct.remote_addr.is_none());
        assert!(acct.remote_balance.is_empty());
        assert_eq!(0, acct.last_update_time.nanos());

        // now get feedback from WhoAmI packet
        let remote_addr = "account-789";
        who_am_i_response(deps.as_mut(), channel_id, remote_addr);

        // account should be set up
        let q = QueryMsg::Account {
            channel_id: channel_id.into(),
        };
        let r = query(deps.as_ref(), mock_env(), q).unwrap();
        let acct: AccountResponse = from_json(&r).unwrap();
        assert_eq!(acct.remote_addr.unwrap(), remote_addr);
        assert!(acct.remote_balance.is_empty());
        assert_eq!(0, acct.last_update_time.nanos());
    }

    #[test]
    fn dispatch_message_send_and_ack() {
        let channel_id = "channel-1234";
        let remote_addr = "account-789";

        // init contract
        let mut deps = setup();
        let creator = deps.api.addr_make(CREATOR);
        // channel handshake
        connect(deps.as_mut(), channel_id);
        // get feedback from WhoAmI packet
        who_am_i_response(deps.as_mut(), channel_id, remote_addr);

        // try to dispatch a message
        let msgs_to_dispatch = vec![BankMsg::Send {
            to_address: "my-friend".into(),
            amount: coins(123456789, "uatom"),
        }
        .into()];
        let handle_msg = ExecuteMsg::SendMsgs {
            channel_id: channel_id.into(),
            msgs: msgs_to_dispatch,
            callback_id: None,
        };
        let info = message_info(&creator, &[]);
        let mut res = execute(deps.as_mut(), mock_env(), info, handle_msg).unwrap();
        assert_eq!(1, res.messages.len());
        let msg = match res.messages.swap_remove(0).msg {
            CosmosMsg::Ibc(IbcMsg::SendPacket {
                channel_id, data, ..
            }) => {
                let ack = IbcAcknowledgement::new(StdAck::success(()));
                let mut msg = mock_ibc_packet_ack(&channel_id, &1u32, ack).unwrap();
                msg.original_packet.data = data;
                msg
            }
            o => panic!("Unexpected message: {:?}", o),
        };
        let res = ibc_packet_ack(deps.as_mut(), mock_env(), msg).unwrap();
        // no actions expected, but let's check the events to see it was dispatched properly
        assert_eq!(0, res.messages.len());
        assert_eq!(vec![("action", "acknowledge_dispatch")], res.attributes)
    }

    #[test]
    fn send_remote_funds() {
        let reflect_channel_id = "channel-1234";
        let remote_addr = "account-789";
        let transfer_channel_id = "transfer-2";

        // init contract
        let mut deps = setup();
        let creator = deps.api.addr_make(CREATOR);
        // channel handshake
        connect(deps.as_mut(), reflect_channel_id);
        // get feedback from WhoAmI packet
        who_am_i_response(deps.as_mut(), reflect_channel_id, remote_addr);

        // let's try to send funds to a channel that doesn't exist
        let msg = ExecuteMsg::SendFunds {
            ica_channel_id: "random-channel".into(),
            transfer_channel_id: transfer_channel_id.into(),
        };
        let info = message_info(&creator, &coins(12344, "utrgd"));
        execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        // let's try with no sent funds in the message
        let msg = ExecuteMsg::SendFunds {
            ica_channel_id: reflect_channel_id.into(),
            transfer_channel_id: transfer_channel_id.into(),
        };
        let info = message_info(&creator, &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        // 3rd times the charm
        let msg = ExecuteMsg::SendFunds {
            ica_channel_id: reflect_channel_id.into(),
            transfer_channel_id: transfer_channel_id.into(),
        };
        let info = message_info(&creator, &coins(12344, "utrgd"));
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(1, res.messages.len());
        match &res.messages[0].msg {
            CosmosMsg::Ibc(IbcMsg::Transfer {
                channel_id,
                to_address,
                amount,
                timeout,
                memo: _,
            }) => {
                assert_eq!(transfer_channel_id, channel_id.as_str());
                assert_eq!(remote_addr, to_address.as_str());
                assert_eq!(&coin(12344, "utrgd"), amount);
                assert!(timeout.block().is_none());
                assert!(timeout.timestamp().is_some());
            }
            o => panic!("unexpected message: {:?}", o),
        }
    }
}
