/**
 * State model for the connect example app.
 *
 * This module keeps the core integration behavior framework-agnostic so tests
 * can validate the pairing flow without depending on a browser DOM or React
 * rendering details.
 */
import {
  DEFAULT_CONNECT_URL,
  createConnectClient,
  type ConnectClient,
  type ConnectClientOptions,
  type PairResult,
  type SessionContext,
} from "@ccd-wallet/connect-client";

/**
 * Mutable example-app state rendered in the reference UI.
 */
export interface ExampleAppState {
  /** WebSocket URL used for the local connect server. */
  serverUrl: string;
  /** Six-digit challenge shown to the user before pairing. */
  challenge: string;
  /** Human-readable status message for the current flow state. */
  status: string;
  /** Session token returned by successful pairing, if available. */
  sessionToken: string;
  /** Approved session context returned by the connect flow, if available. */
  context: SessionContext | null;
}

/**
 * Minimal interface the example app needs from the connect client package.
 */
export interface ConnectClientLike {
  /** Opens the WebSocket connection. */
  connect(): Promise<void>;
  /** Requests pairing with an application-provided challenge. */
  pair(challenge: string): Promise<PairResult>;
  /** Retrieves approved session context using a session token. */
  getSessionContext(sessionToken: string): Promise<SessionContext>;
  /** Closes the current WebSocket connection. */
  close(): void;
}

/**
 * Factory used to create connect clients for the example app.
 */
export type ConnectClientFactory = (
  options: ConnectClientOptions,
) => ConnectClientLike;

/**
 * Function used to generate a new six-digit pairing challenge.
 */
export type ChallengeGenerator = () => string;

/**
 * Imperative model API used by the example app UI and tests.
 */
export interface ExampleAppModel {
  /** Returns the current app state snapshot. */
  getState(): ExampleAppState;
  /** Updates the connect-server URL used for future pairing attempts. */
  setServerUrl(url: string): void;
  /** Updates the displayed pairing challenge. */
  setChallenge(challenge: string): void;
  /** Replaces the current challenge with a newly generated one. */
  regenerateChallenge(): void;
  /** Requests pairing through the connect client package. */
  pair(): Promise<void>;
  /** Refreshes approved session context for the active session token. */
  refresh(): Promise<void>;
  /** Resets local UI state and closes any active client connection. */
  reset(): void;
  /** Registers a callback invoked whenever state changes. */
  subscribe(listener: (state: ExampleAppState) => void): () => void;
}

/**
 * Options for constructing the example app model.
 */
export interface ExampleAppModelOptions {
  /** Optional connect client factory, primarily useful for tests. */
  clientFactory?: ConnectClientFactory;
  /** Optional challenge generator, primarily useful for tests. */
  challengeGenerator?: ChallengeGenerator;
  /** Optional initial connect-server URL. */
  initialServerUrl?: string;
}

/**
 * Generates a six-digit challenge suitable for the current pairing flow.
 *
 * @returns A zero-padded six-digit challenge string.
 * @example
 * ```ts
 * const challenge = generateChallenge();
 * ```
 */
export function generateChallenge(): string {
  const bytes = new Uint8Array(6);
  if (typeof globalThis.crypto?.getRandomValues === "function") {
    globalThis.crypto.getRandomValues(bytes);
  } else {
    for (let index = 0; index < bytes.length; index += 1) {
      bytes[index] = Math.floor(Math.random() * 256);
    }
  }

  return Array.from(bytes, (value) => (value % 10).toString()).join("");
}

/**
 * Creates the stateful model backing the example app.
 *
 * @param options - Optional hooks for tests and custom initialization.
 * @returns A model exposing the example integration flow.
 * @example
 * ```ts
 * const model = createExampleAppModel();
 * await model.pair();
 * ```
 */
export function createExampleAppModel(
  options: ExampleAppModelOptions = {},
): ExampleAppModel {
  const clientFactory = options.clientFactory ?? defaultClientFactory;
  const challengeGenerator = options.challengeGenerator ?? generateChallenge;
  const listeners = new Set<(state: ExampleAppState) => void>();

  let state: ExampleAppState = {
    serverUrl: options.initialServerUrl ?? DEFAULT_CONNECT_URL,
    challenge: challengeGenerator(),
    status: "Ready to pair.",
    sessionToken: "",
    context: null,
  };
  let client: ConnectClientLike | undefined;

  const emit = (): void => {
    for (const listener of listeners) {
      listener({ ...state });
    }
  };

  const update = (patch: Partial<ExampleAppState>): void => {
    state = { ...state, ...patch };
    emit();
  };

  const closeClient = (): void => {
    client?.close();
    client = undefined;
  };

  return {
    getState(): ExampleAppState {
      return { ...state };
    },

    setServerUrl(url: string): void {
      update({ serverUrl: url });
    },

    setChallenge(challenge: string): void {
      update({ challenge });
    },

    regenerateChallenge(): void {
      update({ challenge: challengeGenerator() });
    },

    async pair(): Promise<void> {
      closeClient();
      update({ status: "Connecting and requesting pairing..." });
      try {
        client = clientFactory({ url: state.serverUrl });
        await client.connect();
        const result = await client.pair(state.challenge);
        update({
          status: "Pairing approved.",
          sessionToken: result.sessionToken,
          context: result.context,
        });
      } catch (error) {
        closeClient();
        update({
          status: formatErrorStatus(error),
          sessionToken: "",
          context: null,
        });
      }
    },

    async refresh(): Promise<void> {
      if (!client || !state.sessionToken) {
        update({ status: "No active session to refresh." });
        return;
      }

      update({ status: "Refreshing approved session context..." });
      try {
        const context = await client.getSessionContext(state.sessionToken);
        update({ status: "Session context refreshed.", context });
      } catch (error) {
        update({ status: formatErrorStatus(error) });
      }
    },

    reset(): void {
      closeClient();
      state = {
        serverUrl: state.serverUrl,
        challenge: challengeGenerator(),
        status: "Ready to pair.",
        sessionToken: "",
        context: null,
      };
      emit();
    },

    subscribe(listener: (state: ExampleAppState) => void): () => void {
      listeners.add(listener);
      listener({ ...state });
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

function defaultClientFactory(options: ConnectClientOptions): ConnectClient {
  return createConnectClient(options);
}

function formatErrorStatus(error: unknown): string {
  if (error instanceof Error) {
    return `Error: ${error.message}`;
  }
  return "Error: Unknown failure";
}
