import { Link, testutils } from "@confio/relayer";
import { assert } from "@cosmjs/utils";
import test from "ava";
import { Order } from "cosmjs-types/ibc/core/channel/v1/channel";

const { osmosis: oldOsmo, setup, wasmd } = testutils;

const osmosis = { ...oldOsmo, minFee: "0.025uosmo" };

// TODO: replace these with be auto-generated helpers from ts-codegen
// import { balance, init, sendTokens } from "./cw20";
// import { assertPacketsFromA, assertPacketsFromB, setupContracts } from "./utils";
import { setupContracts, setupOsmosisClient, setupWasmClient } from "./utils";

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

test.serial("set up channel with ics20 contract", async (t) => {
  // instantiate ica controller on wasmd
  const cosmwasm = await setupWasmClient();
  const initController = {};
  const { contractAddress: controllerAddr } = await cosmwasm.sign.instantiate(
    cosmwasm.senderAddress,
    wasmIds.controller,
    initController,
    "simple controller",
    "auto"
  );
  t.truthy(controllerAddr);
  const { ibcPortId: controllerPort } = await cosmwasm.sign.getContract(controllerAddr);
  console.log(`Controller Port: ${controllerPort}`);
  assert(controllerPort);

  // instantiate ica host on osmosis
  const osmo = await setupOsmosisClient();
  const initHost = {
    reflect_code_id: osmosisIds.whitelist,
  };
  const { contractAddress: hostAddr } = await osmo.sign.instantiate(
    osmo.senderAddress,
    osmosisIds.host,
    initHost,
    "simple host",
    "auto"
  );
  t.truthy(hostAddr);
  const { ibcPortId: hostPort } = await osmo.sign.getContract(hostAddr);
  console.log(`Host Port: ${hostPort}`);
  assert(hostPort);

  const [src, dest] = await setup(wasmd, osmosis);
  const link = await Link.createWithNewConnections(src, dest);
  await link.createChannel("A", controllerPort, hostPort, Order.ORDER_UNORDERED, "simple-ica-v1");
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
