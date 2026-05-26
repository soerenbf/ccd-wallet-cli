/**
 * Account-authority feature helpers for the connect client package.
 *
 * This module contains the feature-level wrapper around the `requestAccount`
 * protocol method while leaving transport and JSON-RPC details to the shared
 * client core.
 */
import { REQUEST_ACCOUNT_METHOD } from "../core/types.js";
import type { RequestAccountParams, RequestAccountResult } from "../core/types.js";

export interface AccountRequester {
  request<TResult, TParams = unknown>(
    method: string,
    params?: TParams,
  ): Promise<TResult>;
}

export async function requestAccount(
  requester: AccountRequester,
  sessionToken: string,
  networkGenesisHash: string,
): Promise<string> {
  const result = await requester.request<RequestAccountResult, RequestAccountParams>(
    REQUEST_ACCOUNT_METHOD,
    {
      sessionToken,
      networkGenesisHash,
    },
  );
  return result.accountAddress;
}
