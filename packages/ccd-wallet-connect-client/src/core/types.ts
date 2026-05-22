/**
 * Shared public types and constants for the connect client package.
 *
 * This module contains the stable type contract exposed by the client,
 * including JSON-RPC envelopes, connect method parameters, session models, and
 * the minimal WebSocket surface required by the package.
 */

/**
 * Default WebSocket endpoint for the local `ccd-wallet connect` server.
 */
export const DEFAULT_CONNECT_URL = "ws://127.0.0.1:22771";

/**
 * JSON-RPC method name for initiating browser pairing.
 */
export const PAIR_METHOD = "pair";

/**
 * JSON-RPC method name for retrieving approved session context.
 */
export const SESSION_GET_CONTEXT_METHOD = "session.getContext";

/**
 * JSON-RPC 2.0 request identifier type used by the connect protocol.
 */
export type JsonRpcId = string | number | null;

/**
 * JSON-RPC 2.0 request envelope.
 *
 * @typeParam TParams - Shape of the `params` payload for the request.
 */
export interface JsonRpcRequest<TParams = unknown> {
  /** JSON-RPC protocol version. */
  jsonrpc: "2.0";
  /** Request identifier used to correlate responses. */
  id: JsonRpcId;
  /** Method name being invoked. */
  method: string;
  /** Optional method parameters sent with the request. */
  params?: TParams;
}

/**
 * Successful JSON-RPC 2.0 response envelope.
 *
 * @typeParam TResult - Shape of the `result` payload.
 */
export interface JsonRpcSuccess<TResult = unknown> {
  /** JSON-RPC protocol version. */
  jsonrpc: "2.0";
  /** Identifier of the request this response belongs to. */
  id: JsonRpcId;
  /** Successful method result payload. */
  result: TResult;
}

/**
 * JSON-RPC 2.0 error payload.
 */
export interface JsonRpcErrorObject {
  /** Machine-readable error code. */
  code: number;
  /** Human-readable error message. */
  message: string;
  /** Optional structured error details. */
  data?: unknown;
}

/**
 * Failed JSON-RPC 2.0 response envelope.
 */
export interface JsonRpcFailure {
  /** JSON-RPC protocol version. */
  jsonrpc: "2.0";
  /** Identifier of the request this response belongs to. */
  id: JsonRpcId;
  /** Error payload returned by the server. */
  error: JsonRpcErrorObject;
}

/**
 * Union of successful and failed JSON-RPC 2.0 responses.
 *
 * @typeParam TResult - Shape of the successful `result` payload.
 */
export type JsonRpcResponse<TResult = unknown> =
  | JsonRpcSuccess<TResult>
  | JsonRpcFailure;

/**
 * Parameters for the `pair` method.
 */
export interface PairParams {
  /** Six-digit challenge displayed to the user in the calling application. */
  challenge: string;
}

/**
 * Parameters for the `session.getContext` method.
 */
export interface SessionParams {
  /** Session token returned by a successful pairing request. */
  sessionToken: string;
}

/**
 * Approved session context returned by the connect server.
 */
export interface SessionContext {
  /** Genesis hash of the approved network for the active session. */
  networkGenesisHash: string;
  /** Account address approved for the active session. */
  accountAddress: string;
}

/**
 * Successful result of the `pair` method.
 */
export interface PairResult {
  /** Session token identifying the approved browser session. */
  sessionToken: string;
  /** Approved network and account context for the session. */
  context: SessionContext;
}

/**
 * Options for constructing a {@link ConnectClient}.
 */
export interface ConnectClientOptions {
  /**
   * WebSocket URL of the local connect server.
   *
   * Defaults to {@link DEFAULT_CONNECT_URL}.
   */
  url?: string;
  /**
   * Optional WebSocket constructor used by the client.
   *
   * Pass this when the runtime does not provide `globalThis.WebSocket` or when
   * tests need to inject a mock implementation.
   */
  WebSocket?: WebSocketConstructor;
}

/**
 * Constructor signature for a WebSocket implementation compatible with the
 * connect client.
 */
export interface WebSocketConstructor {
  /**
   * Creates a new WebSocket connection.
   *
   * @param url - WebSocket URL to connect to.
   */
  new (url: string): WebSocketLike;
}

/**
 * Minimal WebSocket surface required by the connect client.
 */
export interface WebSocketLike {
  /** Current connection state represented as a numeric ready-state value. */
  readonly readyState: number;
  /** Callback fired when the socket opens successfully. */
  onopen: ((event: unknown) => void) | null;
  /** Callback fired when a message is received from the server. */
  onmessage: ((event: { data: unknown }) => void) | null;
  /** Callback fired when the socket encounters an error. */
  onerror: ((event: unknown) => void) | null;
  /** Callback fired when the socket closes. */
  onclose: ((event: unknown) => void) | null;
  /**
   * Sends a serialized message to the server.
   *
   * @param data - Text payload to send over the socket.
   */
  send(data: string): void;
  /**
   * Closes the WebSocket connection.
   */
  close(): void;
}
