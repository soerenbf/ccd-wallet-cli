/**
 * Error types for the connect client package.
 *
 * This module contains the public error surface exposed by the client when
 * connection setup, protocol parsing, or server-reported JSON-RPC operations
 * fail.
 */

/**
 * Error raised by the connect client when connection setup, protocol parsing,
 * or server-reported JSON-RPC operations fail.
 */
export class ConnectClientError extends Error {
  /** Optional JSON-RPC or client-specific error code. */
  readonly code: number | undefined;
  /** Optional structured error details associated with the failure. */
  readonly data: unknown;

  /**
   * Creates a new connect client error.
   *
   * @param message - Human-readable error message.
   * @param code - Optional error code, typically from a JSON-RPC failure.
   * @param data - Optional structured error data.
   * @returns A {@link ConnectClientError} instance.
   * @example
   * ```ts
   * throw new ConnectClientError("rejected by user", -32000);
   * ```
   */
  constructor(message: string, code?: number, data?: unknown) {
    super(message);
    this.name = "ConnectClientError";
    this.code = code;
    this.data = data;
  }
}
