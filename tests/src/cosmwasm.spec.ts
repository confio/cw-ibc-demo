import { CosmWasmSigner, Link, testutils } from "@confio/relayer";
// import { toBinary } from "@cosmjs/cosmwasm-stargate";
import { assert } from "@cosmjs/utils";
import test from "ava";
import { Order } from "cosmjs-types/ibc/core/channel/v1/channel";

const { osmosis: oldOsmo, setup, wasmd, randomAddress } = testutils;
const osmosis = { ...oldOsmo, minFee: "0.025uosmo" };

import { CallbackCapturerClient } from "./bindings/CallbackCapturer.client";
import { Binary, StdAck } from "./bindings/CallbackCapturer.types";
import {
  checkRemoteBalance,
  fundRemoteAccount,
  ibcQuery,
  listAccounts,
  remoteBankMultiSend,
  remoteBankSend,
  showAccount,
} from "./controller";
import {
  assertPacketsFromA,
  IbcVersion,
  parseAcknowledgementSuccess,
  parseBinary,
  setupContracts,
  setupOsmosisClient,
  setupWasmClient,
} from "./utils";

let wasmIds: Record<string, number> = {};
let osmosisIds: Record<string, number> = {};

test.before(async (t) => {
  console.debug("Upload contracts to wasmd...");
  const wasmContracts = {
    callback: "./internal/callback_capturer.wasm",
    controller: "./internal/simple_ica_controller.wasm",
  };
  const wasmSign = await setupWasmClient();
  wasmIds = await setupContracts(wasmSign, wasmContracts);

  console.debug("Upload contracts to osmosis...");
  const osmosisContracts = {
    host: "./internal/simple_ica_host.wasm",
    whitelist: "./external/cw1_whitelist.wasm",
  };
  const osmosisSign = await setupOsmosisClient();
  osmosisIds = await setupContracts(osmosisSign, osmosisContracts);

  t.pass();
});

test.serial("set up channel with ica contract", async (t) => {
  // instantiate ica controller on wasmd
  const wasmClient = await setupWasmClient();
  const initController = {};
  const { contractAddress: wasmCont } = await wasmClient.sign.instantiate(
    wasmClient.senderAddress,
    wasmIds.controller,
    initController,
    "simple controller",
    "auto"
  );
  t.truthy(wasmCont);
  const { ibcPortId: controllerPort } = await wasmClient.sign.getContract(wasmCont);
  t.log(`Controller Port: ${controllerPort}`);
  assert(controllerPort);

  // instantiate ica host on osmosis
  const osmoClient = await setupOsmosisClient();
  const initHost = {
    cw1_code_id: osmosisIds.whitelist,
  };
  const { contractAddress: osmoHost } = await osmoClient.sign.instantiate(
    osmoClient.senderAddress,
    osmosisIds.host,
    initHost,
    "simple host",
    "auto"
  );
  t.truthy(osmoHost);
  const { ibcPortId: hostPort } = await osmoClient.sign.getContract(osmoHost);
  t.log(`Host Port: ${hostPort}`);
  assert(hostPort);

  const [src, dest] = await setup(wasmd, osmosis);
  const link = await Link.createWithNewConnections(src, dest);
  await link.createChannel("A", controllerPort, hostPort, Order.ORDER_UNORDERED, IbcVersion);
});

interface SetupInfo {
  wasmClient: CosmWasmSigner;
  osmoClient: CosmWasmSigner;
  wasmController: string;
  osmoHost: string;
  link: Link;
  ics20: {
    wasm: string;
    osmo: string;
  };
}

async function demoSetup(): Promise<SetupInfo> {
  // instantiate ica controller on wasmd
  const wasmClient = await setupWasmClient();
  const initController = {};
  const { contractAddress: wasmController } = await wasmClient.sign.instantiate(
    wasmClient.senderAddress,
    wasmIds.controller,
    initController,
    "simple controller",
    "auto"
  );
  const { ibcPortId: controllerPort } = await wasmClient.sign.getContract(wasmController);
  assert(controllerPort);

  // instantiate ica host on osmosis
  const osmoClient = await setupOsmosisClient();
  const initHost = {
    cw1_code_id: osmosisIds.whitelist,
  };
  const { contractAddress: osmoHost } = await osmoClient.sign.instantiate(
    osmoClient.senderAddress,
    osmosisIds.host,
    initHost,
    "simple host",
    "auto"
  );
  const { ibcPortId: hostPort } = await osmoClient.sign.getContract(osmoHost);
  assert(hostPort);

  // create a connection and channel for simple-ica
  const [src, dest] = await setup(wasmd, osmosis);
  const link = await Link.createWithNewConnections(src, dest);
  await link.createChannel("A", controllerPort, hostPort, Order.ORDER_UNORDERED, IbcVersion);

  // also create a ics20 channel on this connection
  const ics20Info = await link.createChannel("A", wasmd.ics20Port, osmosis.ics20Port, Order.ORDER_UNORDERED, "ics20-1");
  const ics20 = {
    wasm: ics20Info.src.channelId,
    osmo: ics20Info.dest.channelId,
  };

  return {
    wasmClient,
    osmoClient,
    wasmController,
    osmoHost,
    link,
    ics20,
  };
}

test.serial("connect account and send tokens over", async (t) => {
  const { wasmClient, wasmController, link, ics20 } = await demoSetup();

  // there is an initial packet to relay for the whoami run
  let info = await link.relayAll();
  assertPacketsFromA(info, 1, true);

  // now we query the address locally
  const accounts = await listAccounts(wasmClient, wasmController);
  t.is(accounts.length, 1);
  assert(accounts[0].remote_addr);
  const channelId = accounts[0].channel_id;

  // verify we get the address by channelId
  let account = await showAccount(wasmClient, wasmController, channelId);
  const remoteAddr = account.remote_addr;
  assert(remoteAddr);
  t.is(remoteAddr, accounts[0].remote_addr);
  t.log(`Remote address: ${remoteAddr}`);
  t.deepEqual(account.remote_balance, []);

  // let's send some money to the remoteAddr
  const start = await wasmClient.sign.getBalance(wasmClient.senderAddress, wasmd.denomFee);
  t.log(start);
  const toSend = { amount: "2020808", denom: wasmd.denomFee };
  await fundRemoteAccount(wasmClient, wasmController, channelId, ics20.wasm, toSend);
  // move the ics20 packet now
  info = await link.relayAll();
  // note: we cannot use the assertPacketsFromA helper, as that assumes simple-ica ack shape,
  // which is different than the ics20 ack we get here
  t.is(info.packetsFromA, 1);

  // make sure the balance went down (remember, some fees also taken)
  const middle = await wasmClient.sign.getBalance(wasmClient.senderAddress, wasmd.denomFee);
  t.true(parseInt(start.amount) >= parseInt(middle.amount) + 2020808);

  // and query it remotely
  await checkRemoteBalance(wasmClient, wasmController, channelId);
  info = await link.relayAll();
  assertPacketsFromA(info, 1, true);

  // now the balance should show up
  account = await showAccount(wasmClient, wasmController, channelId);
  t.is(account.remote_addr, remoteAddr);
  t.is(account.remote_balance.length, 1);
  t.log(account.remote_balance[0]);
  const remoteAmt = account.remote_balance[0];
  const remoteDenom = remoteAmt.denom;
  t.log(remoteDenom);
});

test.serial("control action on remote chain", async (t) => {
  const { wasmClient, wasmController, link, osmoClient } = await demoSetup();

  // there is an initial packet to relay for the whoami run
  let info = await link.relayAll();
  assertPacketsFromA(info, 1, true);

  // get the account info
  const accounts = await listAccounts(wasmClient, wasmController);
  t.is(accounts.length, 1);
  const { remote_addr: remoteAddr, channel_id: channelId } = accounts[0];
  assert(remoteAddr);
  assert(channelId);

  // send some osmo to the remote address (using another funded account there)
  const initFunds = { amount: "2500600", denom: osmosis.denomFee };
  await osmoClient.sign.sendTokens(osmoClient.senderAddress, remoteAddr, [initFunds], "auto");

  // make a new empty account on osmosis
  const emptyAddr = randomAddress(osmosis.prefix);
  const noFunds = await osmoClient.sign.getBalance(emptyAddr, osmosis.denomFee);
  t.is(noFunds.amount, "0");

  // from wasmd, send a packet to transfer funds from remoteAddr to emptyAddr
  const sendFunds = { amount: "1200300", denom: osmosis.denomFee };
  await remoteBankSend(wasmClient, wasmController, channelId, emptyAddr, [sendFunds]);

  // relay this over
  info = await link.relayAll();
  assertPacketsFromA(info, 1, true);
  // TODO: add helper for this
  const contractData = parseAcknowledgementSuccess(info.acksFromB[0]);
  // check we get { results : ['']} (one message with no data)
  t.deepEqual(contractData, { results: [""] });

  // ensure that the money was transfered
  const gotFunds = await osmoClient.sign.getBalance(emptyAddr, osmosis.denomFee);
  t.deepEqual(gotFunds, sendFunds);
});

test.serial("handle errors on dispatch", async (t) => {
  const { wasmClient, wasmController, link, osmoClient } = await demoSetup();

  // there is an initial packet to relay for the whoami run
  let info = await link.relayAll();
  assertPacketsFromA(info, 1, true);

  // get the account info
  const accounts = await listAccounts(wasmClient, wasmController);
  t.is(accounts.length, 1);
  const { remote_addr: remoteAddr, channel_id: channelId } = accounts[0];
  assert(remoteAddr);
  assert(channelId);

  // send some osmo to the remote address (using another funded account there)
  const initFunds = { amount: "2500600", denom: osmosis.denomFee };
  await osmoClient.sign.sendTokens(osmoClient.senderAddress, remoteAddr, [initFunds], "auto");

  // make a new empty account on osmosis
  const emptyAddr = randomAddress(osmosis.prefix);
  const noFunds = await osmoClient.sign.getBalance(emptyAddr, osmosis.denomFee);
  t.is(noFunds.amount, "0");

  // from wasmd, send a packet to transfer funds from remoteAddr to emptyAddr
  const sendFunds = { amount: "1200300", denom: "no-such-funds" };
  await remoteBankSend(wasmClient, wasmController, channelId, emptyAddr, [sendFunds]);

  // relay this over
  info = await link.relayAll();
  assertPacketsFromA(info, 1, false);

  // ensure that no money was transfered
  const gotNoFunds = await osmoClient.sign.getBalance(emptyAddr, osmosis.denomFee);
  t.is(gotNoFunds.amount, "0");
});

test.serial("properly rollback first submessage if second fails", async (t) => {
  const { wasmClient, wasmController, link, osmoClient } = await demoSetup();

  // there is an initial packet to relay for the whoami run
  let info = await link.relayAll();
  assertPacketsFromA(info, 1, true);

  // get the account info
  const accounts = await listAccounts(wasmClient, wasmController);
  t.is(accounts.length, 1);
  const { remote_addr: remoteAddr, channel_id: channelId } = accounts[0];
  assert(remoteAddr);
  assert(channelId);

  // send some osmo to the remote address (using another funded account there)
  const initFunds = { amount: "2500600", denom: osmosis.denomFee };
  await osmoClient.sign.sendTokens(osmoClient.senderAddress, remoteAddr, [initFunds], "auto");

  // make a new empty account on osmosis
  const emptyAddr = randomAddress(osmosis.prefix);
  const noFunds = await osmoClient.sign.getBalance(emptyAddr, osmosis.denomFee);
  t.is(noFunds.amount, "0");

  // from wasmd, send a packet to transfer funds from remoteAddr to emptyAddr
  // first message with valid funds, second with invalid
  // should return error ack, both transfers should eb rolled back
  const goodSend = { amount: "1200300", denom: osmosis.denomFee };
  const badSend = { amount: "1200300", denom: "no-such-funds" };
  const contents = [
    { to_address: emptyAddr, amount: [goodSend] },
    { to_address: emptyAddr, amount: [badSend] },
  ];
  await remoteBankMultiSend(wasmClient, wasmController, channelId, contents);

  // relay this over
  info = await link.relayAll();
  assertPacketsFromA(info, 1, false);

  // ensure that no money was transfered
  const gotNoFunds = await osmoClient.sign.getBalance(emptyAddr, osmosis.denomFee);
  t.is(gotNoFunds.amount, "0");
});

test.serial("query remote chain", async (t) => {
  const { wasmClient, wasmController, link, osmoClient } = await demoSetup();

  // there is an initial packet to relay for the whoami run
  let info = await link.relayAll();
  assertPacketsFromA(info, 1, true);

  // get the account info
  const accounts = await listAccounts(wasmClient, wasmController);
  t.is(accounts.length, 1);
  const { remote_addr: remoteAddr, channel_id: channelId } = accounts[0];
  assert(remoteAddr);
  assert(channelId);

  // send some osmo to the remote address (using another funded account there)
  const initFunds = { amount: "2500600", denom: osmosis.denomFee };
  await osmoClient.sign.sendTokens(osmoClient.senderAddress, remoteAddr, [initFunds], "auto");

  // Use IBC queries to query account info from the remote contract
  const bankQuery = [{ bank: { all_balances: { address: remoteAddr } } }];
  await ibcQuery(wasmClient, wasmController, channelId, bankQuery);

  // relay this over
  info = await link.relayAll();
  assertPacketsFromA(info, 1, true);

  // now query the latest query info stored
  const bankStored = await wasmClient.sign.queryContractSmart(wasmController, {
    latest_query_result: { channel_id: channelId },
  });
  const firstTime = bankStored.last_update_time;
  t.truthy(firstTime);
  assert(bankStored.response.result);
  const storedSuccess = parseBinary(bankStored.response.result);
  t.log(storedSuccess);
  assert(storedSuccess.results);
  t.is(storedSuccess.results.length, 1);
  const storedBank = parseBinary(storedSuccess.results[0]);
  t.log(storedBank);
  t.deepEqual(storedBank, { amount: [initFunds] });

  // Demo a failing query
  const badQuery = [{ wasm: { smart: { contract_addr: "no such contract", msg: "e30=" } } }];
  await ibcQuery(wasmClient, wasmController, channelId, badQuery);
  // this should return an error acknowledgement
  info = await link.relayAll();
  assertPacketsFromA(info, 1, false);

  // now query this is stored properly
  const badStored = await wasmClient.sign.queryContractSmart(wasmController, {
    latest_query_result: { channel_id: channelId },
  });
  const secondTime = badStored.last_update_time;
  t.truthy(secondTime);
  t.notDeepEqual(firstTime, secondTime);
  t.log(badStored.response);
  assert(badStored.response.error);
});

test.only("query with callback", async (t) => {
  const { wasmClient, wasmController, link, osmoClient } = await demoSetup();

  // there is an initial packet to relay for the whoami run
  let info = await link.relayAll();
  assertPacketsFromA(info, 1, true);

  // switch over to use the callback contract
  const { contractAddress: callbackAddr } = await wasmClient.sign.instantiate(
    wasmClient.senderAddress,
    wasmIds.callback,
    { simple_ica_controller: wasmController },
    "Callback #1",
    "auto"
  );
  const callback = new CallbackCapturerClient(wasmClient.sign, wasmClient.senderAddress, callbackAddr);

  // and update the admin, so the callback contract can use the controller
  await wasmClient.sign.execute(
    wasmClient.senderAddress,
    wasmController,
    { update_admin: { admin: callbackAddr } },
    "auto"
  );

  // get the account info
  const accounts = await listAccounts(wasmClient, wasmController);
  t.is(accounts.length, 1);
  const { remote_addr: remoteAddr, channel_id: channelId } = accounts[0];
  assert(remoteAddr);
  assert(channelId);

  // send some osmo to the remote address (using another funded account there)
  const initFunds = { amount: "2500600", denom: osmosis.denomFee };
  await osmoClient.sign.sendTokens(osmoClient.senderAddress, remoteAddr, [initFunds], "auto");

  // No result here at first
  try {
    await callback.result({ id: "demo1" });
    t.fail("should have thrown");
  } catch (e) {
    t.truthy(e);
  }

  // Use IBC queries to query account info from the remote contract
  const bankQuery = [{ bank: { all_balances: { address: remoteAddr } } }];
  await callback.ibcQuery({ callbackId: "demo1", channelId, msgs: bankQuery });

  // relay this over
  info = await link.relayAll();
  assertPacketsFromA(info, 1, true);

  // now ensure the callback is stored in the callback contract
  const cb = await callback.result({ id: "demo1" });
  t.log(cb.result);
  if (!isSuccess(cb.result)) {
    t.fail(`got error ${cb.result.error}`);
  } else {
    const storedSuccess = parseBinary(cb.result.result);
    t.log(storedSuccess);
    assert(storedSuccess.results);
    t.is(storedSuccess.results.length, 1);
    const storedBank = parseBinary(storedSuccess.results[0]);
    t.log(storedBank);
    t.deepEqual(storedBank, { amount: [initFunds] });
  }

  // // Demo a failing query
  // const badQuery = [{ wasm: { smart: { contract_addr: "no such contract", msg: "e30=" } } }];
  // await ibcQuery(wasmClient, wasmController, channelId, badQuery);
  // // this should return an error acknowledgement
  // info = await link.relayAll();
  // assertPacketsFromA(info, 1, false);

  // // now query this is stored properly
  // const badStored = await wasmClient.sign.queryContractSmart(wasmController, {
  //   latest_query_result: { channel_id: channelId },
  // });
  // const secondTime = badStored.last_update_time;
  // t.truthy(secondTime);
  // t.notDeepEqual(firstTime, secondTime);
  // t.log(badStored.response);
  // assert(badStored.response.error);
});

function isSuccess(ack: StdAck): ack is { result: Binary } {
  return (ack as { result: Binary }).result !== undefined;
}
