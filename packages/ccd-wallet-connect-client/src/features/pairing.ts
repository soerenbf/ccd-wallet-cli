/**
 * Pairing feature helpers for the connect client package.
 *
 * This module contains the thin feature-level wrapper around the `pair`
 * protocol method while leaving transport and JSON-RPC details to the shared
 * client core.
 */
import { PAIR_METHOD } from "../core/types.js";
import type { PairParams, PairResult } from "../core/types.js";

export interface PairingRequester {
  request<TResult, TParams = unknown>(
    method: string,
    params?: TParams,
  ): Promise<TResult>;
}

export function pair(
  requester: PairingRequester,
  challenge: string,
): Promise<PairResult> {
  return requester.request<PairResult, PairParams>(PAIR_METHOD, { challenge });
}
