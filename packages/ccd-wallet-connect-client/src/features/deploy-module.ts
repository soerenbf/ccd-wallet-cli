/**
 * Deploy-module feature helpers for the connect client package.
 *
 * This module contains the thin feature-level wrapper around the
 * `requestDeployModule` protocol method.
 */
import { REQUEST_DEPLOY_MODULE_METHOD } from "../core/types.js";
import type { DeployModuleParams, DeployModuleResult } from "../core/types.js";

export interface DeployModuleRequester {
  request<TResult, TParams = unknown>(
    method: string,
    params?: TParams,
  ): Promise<TResult>;
}

/**
 * Requests wallet-approved smart contract module deployment.
 *
 * @param requester - Client-like object capable of sending JSON-RPC requests.
 * @param params - Deploy-module parameters including the active session token,
 * hex-encoded module bytes, and optional validation flag.
 * @returns The transaction hash returned by the wallet after submission.
 * @throws {import("../core/errors.js").ConnectClientError} If the socket is not
 * open, the token is invalid, the user declines, duplicate validation rejects,
 * or submission fails.
 * @example
 * ```ts
 * const { transactionHash } = await requestDeployModule(client, {
 *   sessionToken,
 *   moduleHex: "0061736d...",
 *   validate: true,
 * });
 * ```
 */
export async function requestDeployModule(
  requester: DeployModuleRequester,
  params: DeployModuleParams,
): Promise<DeployModuleResult> {
  return requester.request<DeployModuleResult, DeployModuleParams>(
    REQUEST_DEPLOY_MODULE_METHOD,
    params,
  );
}
