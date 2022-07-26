import { CosmWasmSigner, Link, testutils } from "@confio/relayer";
import { assert } from "@cosmjs/utils";
import test from "ava";
import { Order } from "cosmjs-types/ibc/core/channel/v1/channel";

const { osmosis: oldOsmo, setup, wasmd } = testutils;
const osmosis = { ...oldOsmo, minFee: "0.025uosmo" };

// TODO: replace these with be auto-generated helpers from ts-codegen
import { checkRemoteBalance, fundRemoteAccount, listAccounts, showAccount } from "./controller";
import { assertPacketsFromA, IbcVersion, setupContracts, setupOsmosisClient, setupWasmClient } from "./utils";

let wasmIds: Record<string, number> = {};
let osmosisIds: Record<string, number> = {};

test.before(async (t) => {
  console.debug("Upload contracts to wasmd...");
  const wasmContracts = {
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

// TODO: re-active this
test.skip("set up channel with ics20 contract", async (t) => {
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
  console.log(`Controller Port: ${controllerPort}`);
  assert(controllerPort);

  // instantiate ica host on osmosis
  const osmoClient = await setupOsmosisClient();
  const initHost = {
    reflect_code_id: osmosisIds.whitelist,
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
  console.log(`Host Port: ${hostPort}`);
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
    reflect_code_id: osmosisIds.whitelist,
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

test.serial("connect account and send tokens", async (t) => {
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

  // Finally, we use an simple-ica method to send this back
});

/*
test.serial("send packets with ics20 contract", async (t) => {
  const cosmwasm = await setupWasmClient();

  // instantiate cw20
  const initMsg = init(cosmwasm.senderAddress, "CASH", "123456789000");
  const { contractAddress: cw20Addr } = await cosmwasm.sign.instantiate(
    cosmwasm.senderAddress,
    codeIds.cw20,
    initMsg,
    "CASH",
    "auto"
  );
  t.truthy(cw20Addr);
  let bal = await balance(cosmwasm, cw20Addr);
  t.is("123456789000", bal);

  // instantiate ics20
  const ics20Msg = {
    default_timeout: 3600,
    gov_contract: cosmwasm.senderAddress,
    allowlist: [
      {
        contract: cw20Addr,
        gas_limit: 250000,
      },
    ],
  };
  const { contractAddress: ics20Addr } = await cosmwasm.sign.instantiate(
    cosmwasm.senderAddress,
    codeIds.ics20,
    ics20Msg,
    "ICSX",
    "auto"
  );
  t.truthy(ics20Addr);

  const { ibcPortId: wasmPort } = await cosmwasm.sign.getContract(ics20Addr);
  console.log(`Ibc Port: ${wasmPort}`);
  assert(wasmPort);

  const [src, dest] = await setup(gaia, wasmd);
  const link = await Link.createWithNewConnections(src, dest);
  const channels = await link.createChannel("A", gaia.ics20Port, wasmPort, ics20.ordering, ics20.version);

  // send cw20 tokens to ics20 contract and create a new packet
  // (dest chain is wasmd)
  const sendMsg = sendTokens(ics20Addr, "456789000", {
    channel: channels.dest.channelId,
    remote_address: src.senderAddress,
  });
  await cosmwasm.sign.execute(cosmwasm.senderAddress, cw20Addr, sendMsg, "auto", "Send CW20 tokens via ICS20");

  // let's see if the balance went down
  bal = await balance(cosmwasm, cw20Addr);
  t.is("123000000000", bal);

  // check source balance
  const preBalance = await src.sign.getAllBalances(src.senderAddress);
  t.is(1, preBalance.length);
  t.is("uatom", preBalance[0].denom);

  // easy way to move all packets and verify the results
  let info = await link.relayAll();
  assertPacketsFromB(info, 1, true);

  // check source balances increased
  const relayedBalance = await src.sign.getAllBalances(src.senderAddress);
  t.is(2, relayedBalance.length);
  const ibcCoin = relayedBalance.find((d) => d.denom !== "uatom");
  assert(ibcCoin);
  t.is("456789000", ibcCoin.amount);
  console.log(ibcCoin);

  // send this token back over the channel
  const timeoutHeight = await dest.timeoutHeight(500);
  await src.transferTokens(channels.src.portId, channels.src.channelId, ibcCoin, dest.senderAddress, timeoutHeight);
  await src.waitOneBlock();

  // easy way to move all packets
  info = await link.relayAll();
  assertPacketsFromA(info, 1, true);
  // extra check just because... not really needed
  assertPacketsFromB(info, 0, true);

  // balance updated on recipient
  const gotBal = await balance(cosmwasm, cw20Addr, dest.senderAddress);
  t.is(gotBal, "456789000");

  // send native token over channel (from dest -> cosmwasm chain)
  const timeoutHeight2 = await dest.timeoutHeight(500);
  const nativeCoin = {
    denom: "uatom",
    amount: "111111",
  };
  await src.transferTokens(channels.src.portId, channels.src.channelId, nativeCoin, dest.senderAddress, timeoutHeight2);
  await src.waitOneBlock();

  // relay and verify this fails (as it should)
  info = await link.relayAll();
  assertPacketsFromA(info, 1, false);
});
*/
