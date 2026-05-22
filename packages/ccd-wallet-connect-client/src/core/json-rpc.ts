/**
 * JSON-RPC 2.0 helpers for the connect client package.
 *
 * This module keeps wire-protocol request creation and response parsing in one
 * place so higher-level client code can stay focused on feature behavior.
 */
import { ConnectClientError } from "./errors.js";
import type {
  JsonRpcFailure,
  JsonRpcId,
  JsonRpcRequest,
  JsonRpcResponse,
} from "./types.js";

export function createJsonRpcRequest<TParams>(
  id: JsonRpcId,
  method: string,
  params?: TParams,
): JsonRpcRequest<TParams> {
  return {
    jsonrpc: "2.0",
    id,
    method,
    ...(params === undefined ? {} : { params }),
  };
}

export function parseJsonRpcResponse(text: string): JsonRpcResponse {
  const parsed = JSON.parse(text) as unknown;
  if (!isRecord(parsed) || parsed.jsonrpc !== "2.0") {
    throw new ConnectClientError("invalid JSON-RPC response");
  }
  if (!("id" in parsed)) {
    throw new ConnectClientError("JSON-RPC response is missing id");
  }
  if ("error" in parsed) {
    if (!isRecord(parsed.error)) {
      throw new ConnectClientError("invalid JSON-RPC error response");
    }
    return {
      jsonrpc: "2.0",
      id: parsed.id as JsonRpcId,
      error: {
        code: Number(parsed.error.code),
        message: String(parsed.error.message),
        data: parsed.error.data,
      },
    } satisfies JsonRpcFailure;
  }
  if (!("result" in parsed)) {
    throw new ConnectClientError("JSON-RPC response is missing result");
  }
  return {
    jsonrpc: "2.0",
    id: parsed.id as JsonRpcId,
    result: parsed.result,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
