/**
 * Contract-initialization feature helpers for the connect client package.
 *
 * This module contains the thin feature-level wrapper around the
 * `requestContractInit` protocol method.
 */
import { REQUEST_CONTRACT_INIT_METHOD } from "../core/types.js";
import type { ContractInitParams, ContractInitResult } from "../core/types.js";

export interface ContractInitRequester {
  request<TResult, TParams = unknown>(
    method: string,
    params?: TParams,
  ): Promise<TResult>;
}

export async function requestContractInit(
  requester: ContractInitRequester,
  params: ContractInitParams,
): Promise<ContractInitResult> {
  return requester.request<ContractInitResult, ContractInitParams>(
    REQUEST_CONTRACT_INIT_METHOD,
    params,
  );
}
