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
  SESSION_GET_CONTEXT_METHOD,
  type ConnectClientOptions,
  type JsonRpcErrorObject,
  type JsonRpcFailure,
  type JsonRpcId,
  type JsonRpcRequest,
  type JsonRpcResponse,
  type JsonRpcSuccess,
  type PairParams,
  type PairResult,
  type SessionContext,
  type SessionParams,
  type WebSocketConstructor,
  type WebSocketLike,
} from "./core/types.js";
