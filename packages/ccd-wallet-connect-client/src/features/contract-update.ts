/**
 * Contract-update feature helpers for the connect client package.
 *
 * This module contains the thin feature-level wrapper around the
 * `requestContractUpdate` protocol method.
 */
import { REQUEST_CONTRACT_UPDATE_METHOD } from "../core/types.js";
import type { ContractUpdateParams, ContractUpdateResult } from "../core/types.js";

export interface ContractUpdateRequester {
  request<TResult, TParams = unknown>(
    method: string,
    params?: TParams,
  ): Promise<TResult>;
}

export async function requestContractUpdate(
  requester: ContractUpdateRequester,
  params: ContractUpdateParams,
): Promise<ContractUpdateResult> {
  return requester.request<ContractUpdateResult, ContractUpdateParams>(
    REQUEST_CONTRACT_UPDATE_METHOD,
    params,
  );
}
