/**
 * WebSocket runtime helpers for the connect client package.
 *
 * This module isolates environment-facing WebSocket concerns such as resolving
 * a global constructor and normalizing incoming message payloads.
 */
import { ConnectClientError } from "./errors.js";
import type { WebSocketConstructor } from "./types.js";

export const OPEN_READY_STATE = 1;

export function resolveGlobalWebSocket(): WebSocketConstructor {
  const candidate = globalThis.WebSocket;
  if (typeof candidate !== "function") {
    throw new ConnectClientError(
      "No WebSocket implementation is available; pass one with the WebSocket option",
    );
  }
  return candidate as WebSocketConstructor;
}

export function normalizeMessageData(data: unknown): string {
  if (typeof data === "string") {
    return data;
  }
  throw new ConnectClientError("WebSocket message data must be a string");
}
