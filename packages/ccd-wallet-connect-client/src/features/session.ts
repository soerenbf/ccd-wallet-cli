/**
 * Session feature helpers for the connect client package.
 *
 * This module contains feature-level helpers for retrieving approved session
 * context through the shared client request pipeline.
 */
import { SESSION_GET_CONTEXT_METHOD } from "../core/types.js";
import type { SessionContext, SessionParams } from "../core/types.js";

export interface SessionRequester {
  request<TResult, TParams = unknown>(
    method: string,
    params?: TParams,
  ): Promise<TResult>;
}

export function getSessionContext(
  requester: SessionRequester,
  sessionToken: string,
): Promise<SessionContext> {
  return requester.request<SessionContext, SessionParams>(
    SESSION_GET_CONTEXT_METHOD,
    {
      sessionToken,
    },
  );
}
