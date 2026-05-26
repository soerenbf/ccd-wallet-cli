/**
 * High-level client implementation for the `ccd-wallet connect` protocol.
 *
 * This module owns connection lifecycle, request/response correlation, and the
 * flat public client API while delegating protocol details and feature helpers
 * to internal modules.
 */
import { createJsonRpcRequest, parseJsonRpcResponse } from "./core/json-rpc.js";
import { ConnectClientError } from "./core/errors.js";
import {
  DEFAULT_CONNECT_URL,
  type ConnectClientOptions,
  type ContractInitParams,
  type ContractInitResult,
  type ContractUpdateParams,
  type ContractUpdateResult,
  type JsonRpcId,
  type WebSocketConstructor,
  type WebSocketLike,
} from "./core/types.js";
import {
  OPEN_READY_STATE,
  normalizeMessageData,
  resolveGlobalWebSocket,
} from "./core/websocket.js";
import { requestAccount } from "./features/account.js";
import { requestContractInit } from "./features/contract-init.js";
import { requestContractUpdate } from "./features/contract-update.js";
import { pair } from "./features/pairing.js";

interface PendingRequest {
  resolve: (value: unknown) => void;
  reject: (reason: unknown) => void;
}

/**
 * High-level client for the `ccd-wallet connect` WebSocket JSON-RPC API.
 *
 * The client keeps a single WebSocket connection open, correlates JSON-RPC
 * requests and responses, and exposes a flat API for the currently supported
 * pairing and account-request methods.
 *
 * @example
 * ```ts
 * import { ConnectClient } from "@ccd-wallet/connect-client";
 *
 * const client = new ConnectClient();
 * await client.connect();
 * const pairing = await client.pair("123456");
 * const accountAddress = await client.requestAccount(
 *   pairing.sessionToken,
 *   "network-genesis-hash",
 * );
 * client.close();
 * ```
 */
export class ConnectClient {
  private readonly url: string;
  private readonly WebSocketCtor: WebSocketConstructor;
  private socket: WebSocketLike | undefined;
  private nextId = 1;
  private readonly pending = new Map<JsonRpcId, PendingRequest>();

  /**
   * Creates a new connect client.
   *
   * @param options - Optional client configuration such as the WebSocket URL or
   * injected WebSocket constructor.
   * @returns A new {@link ConnectClient} instance.
   * @throws {ConnectClientError} If no WebSocket implementation is available and
   * no constructor is provided through `options.WebSocket`.
   * @example
   * ```ts
   * const client = new ConnectClient({
   *   url: "ws://127.0.0.1:22771",
   * });
   * ```
   */
  constructor(options: ConnectClientOptions = {}) {
    this.url = options.url ?? DEFAULT_CONNECT_URL;
    this.WebSocketCtor = options.WebSocket ?? resolveGlobalWebSocket();
  }

  /**
   * Indicates whether the underlying WebSocket connection is currently open.
   */
  get isConnected(): boolean {
    return this.socket?.readyState === OPEN_READY_STATE;
  }

  /**
   * Opens the WebSocket connection to the connect server if it is not already
   * open.
   *
   * @returns A promise that resolves once the socket is open.
   * @throws {ConnectClientError} If the WebSocket connection cannot be opened.
   * @example
   * ```ts
   * await client.connect();
   * ```
   */
  connect(): Promise<void> {
    if (this.isConnected) {
      return Promise.resolve();
    }

    return new Promise((resolve, reject) => {
      const socket = new this.WebSocketCtor(this.url);
      this.socket = socket;

      socket.onopen = () => resolve();
      socket.onerror = (event) => {
        reject(
          new ConnectClientError(
            "failed to open WebSocket connection",
            undefined,
            event,
          ),
        );
      };
      socket.onclose = (event) => {
        this.rejectPending(
          new ConnectClientError(
            "WebSocket connection closed",
            undefined,
            event,
          ),
        );
        if (this.socket === socket) {
          this.socket = undefined;
        }
      };
      socket.onmessage = (event) => this.handleMessage(event.data);
    });
  }

  /**
   * Closes the current WebSocket connection and rejects any in-flight requests.
   *
   * @returns Nothing.
   * @throws Does not throw intentionally, but pending requests are rejected with
   * {@link ConnectClientError}.
   * @example
   * ```ts
   * client.close();
   * ```
   */
  close(): void {
    const socket = this.socket;
    this.socket = undefined;
    this.rejectPending(new ConnectClientError("WebSocket connection closed"));
    socket?.close();
  }

  /**
   * Requests browser pairing using an application-provided challenge.
   *
   * @param challenge - Six-digit challenge shown to the user in the calling
   * application.
   * @returns The approved session token returned by the connect server.
   * @throws {ConnectClientError} If the socket is not open, the server rejects
   * pairing, or the response is invalid.
   * @example
   * ```ts
   * const pairing = await client.pair("123456");
   * ```
   */
  pair(challenge: string) {
    return pair(this, challenge);
  }

  /**
   * Requests an approved account address for a target network.
   *
   * @param sessionToken - Session token returned by a successful pairing call.
   * @param networkGenesisHash - Genesis hash of the target network.
   * @returns The approved account address for the requested network.
   * @throws {ConnectClientError} If the socket is not open, the token is
   * invalid, or the response is invalid.
   * @example
   * ```ts
   * const accountAddress = await client.requestAccount(
   *   pairing.sessionToken,
   *   "network-genesis-hash",
   * );
   * ```
   */
  requestAccount(sessionToken: string, networkGenesisHash: string) {
    return requestAccount(this, sessionToken, networkGenesisHash);
  }

  /**
   * Requests wallet-approved smart contract initialization.
   *
   * @param params - Contract initialization parameters, including the active
   * session token, module reference, init name, amount, energy ceiling, and
   * serialized parameter bytes.
   * @returns The submitted transaction hash returned by the wallet.
   * @throws {ConnectClientError} If the socket is not open, the token is
   * invalid, the user declines, or submission fails.
   * @example
   * ```ts
   * const { transactionHash } = await client.requestContractInit({
   *   sessionToken,
   *   moduleRef: "...",
   *   initName: "init_my_contract",
   *   amountMicroCcd: "0",
   *   maxContractExecutionEnergy: 30000,
   *   parameterHex: "",
   *   validate: true,
   * });
   * ```
   */
  requestContractInit(params: ContractInitParams): Promise<ContractInitResult> {
    return requestContractInit(this, params);
  }

  /**
   * Requests wallet-approved smart contract update execution.
   *
   * @param params - Contract update parameters, including the active session
   * token, target contract address, receive name, amount, energy ceiling, and
   * serialized parameter bytes.
   * @returns The submitted transaction hash returned by the wallet.
   * @throws {ConnectClientError} If the socket is not open, the token is
   * invalid, the user declines, or submission fails.
   * @example
   * ```ts
   * const { transactionHash } = await client.requestContractUpdate({
   *   sessionToken,
   *   contractAddress: { index: 42, subindex: 0 },
   *   receiveName: "my_contract.transfer",
   *   amountMicroCcd: "0",
   *   maxContractExecutionEnergy: 30000,
   *   parameterHex: "",
   *   validate: true,
   * });
   * ```
   */
  requestContractUpdate(params: ContractUpdateParams): Promise<ContractUpdateResult> {
    return requestContractUpdate(this, params);
  }

  /**
   * Sends a raw JSON-RPC request over the active WebSocket connection.
   *
   * This is part of the public client surface so feature helpers can stay thin
   * and the API can grow without duplicating transport logic.
   *
   * @typeParam TResult - Expected response result payload.
   * @typeParam TParams - Request parameter payload.
   * @param method - JSON-RPC method name to invoke.
   * @param params - Optional method parameters.
   * @returns A promise resolving to the parsed JSON-RPC result payload.
   * @throws {ConnectClientError} If the socket is not open or a response error
   * is returned.
   * @example
   * ```ts
   * const result = await client.request("requestAccount", {
   *   sessionToken: "token",
   *   networkGenesisHash: "network-genesis-hash",
   * });
   * ```
   */
  request<TResult, TParams = unknown>(
    method: string,
    params?: TParams,
  ): Promise<TResult> {
    const socket = this.requireOpenSocket();
    const id = this.nextId++;
    const request = createJsonRpcRequest(id, method, params);

    const promise = new Promise<TResult>((resolve, reject) => {
      this.pending.set(id, {
        resolve: (value) => resolve(value as TResult),
        reject,
      });
    });

    try {
      socket.send(JSON.stringify(request));
    } catch (error) {
      this.pending.delete(id);
      throw error;
    }

    return promise;
  }

  private requireOpenSocket(): WebSocketLike {
    const socket = this.socket;
    if (!socket || socket.readyState !== OPEN_READY_STATE) {
      throw new ConnectClientError("WebSocket connection is not open");
    }
    return socket;
  }

  private handleMessage(data: unknown): void {
    const text = normalizeMessageData(data);
    const response = parseJsonRpcResponse(text);
    const pending = this.pending.get(response.id);
    if (!pending) {
      return;
    }
    this.pending.delete(response.id);

    if ("error" in response) {
      pending.reject(
        new ConnectClientError(
          response.error.message,
          response.error.code,
          response.error.data,
        ),
      );
      return;
    }

    pending.resolve(response.result);
  }

  private rejectPending(reason: unknown): void {
    for (const pending of this.pending.values()) {
      pending.reject(reason);
    }
    this.pending.clear();
  }
}

/**
 * Creates a new {@link ConnectClient}.
 *
 * @param options - Optional client configuration.
 * @returns A new {@link ConnectClient} instance.
 * @throws {ConnectClientError} If no WebSocket implementation is available.
 * @example
 * ```ts
 * const client = createConnectClient();
 * ```
 */
export function createConnectClient(
  options: ConnectClientOptions = {},
): ConnectClient {
  return new ConnectClient(options);
}
