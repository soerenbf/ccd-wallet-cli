/**
 * Shared public types and constants for the connect client package.
 *
 * This module contains the stable type contract exposed by the client,
 * including JSON-RPC envelopes, connect method parameters, pairing/session
 * models, account-request models, and the minimal WebSocket surface required by
 * the package.
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
 * JSON-RPC method name for requesting account authority on a target network.
 */
export const REQUEST_ACCOUNT_METHOD = "requestAccount";

/**
 * JSON-RPC method name for requesting smart contract initialization.
 */
export const REQUEST_CONTRACT_INIT_METHOD = "requestContractInit";

/**
 * JSON-RPC method name for requesting smart contract update execution.
 */
export const REQUEST_CONTRACT_UPDATE_METHOD = "requestContractUpdate";

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
 * Successful result of the `pair` method.
 */
export interface PairResult {
  /** Session token identifying the approved browser session. */
  sessionToken: string;
}

/**
 * Parameters for the `requestAccount` method.
 */
export interface RequestAccountParams {
  /** Session token returned by a successful pairing request. */
  sessionToken: string;
  /** Genesis hash of the target network for which account authority is requested. */
  networkGenesisHash: string;
}

/**
 * Successful result of the `requestAccount` method.
 */
export interface RequestAccountResult {
  /** Account address approved for the requested network. */
  accountAddress: string;
}

/**
 * Smart contract instance address.
 */
export interface ContractAddress {
  /** Contract instance index. */
  index: number;
  /** Contract instance subindex. */
  subindex: number;
}

/**
 * Parameters for the `requestContractInit` method.
 */
export interface ContractInitParams {
  /** Session token returned by a successful pairing request. */
  sessionToken: string;
  /** Hex-encoded module reference to initialize from. */
  moduleRef: string;
  /** Init function name, e.g. `init_my_contract`. */
  initName: string;
  /** CCD amount to attach, in microCCD, encoded as a decimal string. */
  amountMicroCcd: string;
  /** Maximum contract execution energy the caller allows. */
  maxContractExecutionEnergy: number;
  /** Serialized contract parameter bytes encoded as hex. */
  parameterHex: string;
  /** Optional base64-encoded versioned module schema or schema descriptor. */
  schema?: unknown;
  /** Whether the wallet should simulate the request before prompting. */
  validate?: boolean;
}

/**
 * Successful result of the `requestContractInit` method.
 */
export interface ContractInitResult {
  /** Submitted transaction hash. */
  transactionHash: string;
}

/**
 * Parameters for the `requestContractUpdate` method.
 */
export interface ContractUpdateParams {
  /** Session token returned by a successful pairing request. */
  sessionToken: string;
  /** Contract instance to invoke. */
  contractAddress: ContractAddress;
  /** Fully-qualified receive name, e.g. `my_contract.transfer`. */
  receiveName: string;
  /** CCD amount to attach, in microCCD, encoded as a decimal string. */
  amountMicroCcd: string;
  /** Maximum contract execution energy the caller allows. */
  maxContractExecutionEnergy: number;
  /** Serialized contract parameter bytes encoded as hex. */
  parameterHex: string;
  /** Optional base64-encoded versioned module schema or schema descriptor. */
  schema?: unknown;
  /** Whether the wallet should simulate the request before prompting. */
  validate?: boolean;
}

/**
 * Successful result of the `requestContractUpdate` method.
 */
export interface ContractUpdateResult {
  /** Submitted transaction hash. */
  transactionHash: string;
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
