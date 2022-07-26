import { CosmWasmSigner } from "@confio/relayer";
// import { toBase64, toUtf8 } from "@cosmjs/encoding";

export interface Coin {
  denom: string;
  amount: string;
}
export interface AccountInfo {
  channel_id: string;
  last_update_time: string; // nanoseconds as string
  remote_addr?: string;
  remote_balance: Coin[];
}

export interface AccountResponse {
  last_update_time: string; // nanoseconds as string
  remote_addr?: string;
  remote_balance: Coin[];
}

export async function listAccounts(cosmwasm: CosmWasmSigner, controllerAddr: string): Promise<AccountInfo[]> {
  const query = { list_accounts: {} };
  const res = await cosmwasm.sign.queryContractSmart(controllerAddr, query);
  return res.accounts;
}

export async function showAccount(
  cosmwasm: CosmWasmSigner,
  controllerAddr: string,
  channelId: string
): Promise<AccountResponse> {
  const query = { account: { channel_id: channelId } };
  const res = await cosmwasm.sign.queryContractSmart(controllerAddr, query);
  return res;
}

// export function showAccou(targetAddr: string, amount: string, msg: Record<string, unknown>): Record<string, unknown> {
//   const encoded = toBase64(toUtf8(JSON.stringify(msg)));
//   const sendMsg = {
//     send: {
//       contract: targetAddr,
//       amount,
//       msg: encoded,
//     },
//   };
//   return sendMsg;
// }
