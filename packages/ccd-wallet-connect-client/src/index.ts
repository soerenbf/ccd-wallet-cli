/**
 * Public export surface for the connect client package.
 *
 * This module keeps the package API flat while the implementation is organized
 * into shared `core/` helpers and feature-specific modules.
 */
export { ConnectClient, createConnectClient } from "./client.js";
export { ConnectClientError } from "./core/errors.js";
export {
  DEFAULT_CONNECT_URL,
  PAIR_METHOD,
  REQUEST_ACCOUNT_METHOD,
  type ConnectClientOptions,
  type JsonRpcErrorObject,
  type JsonRpcFailure,
  type JsonRpcId,
  type JsonRpcRequest,
  type JsonRpcResponse,
  type JsonRpcSuccess,
  type PairParams,
  type PairResult,
  type RequestAccountParams,
  type RequestAccountResult,
  type WebSocketConstructor,
  type WebSocketLike,
} from "./core/types.js";
