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
} from "@ccd-wallet/connect-client";

/**
 * Mutable example-app state rendered in the reference UI.
 */
export interface ExampleAppState {
  /** WebSocket URL used for the local connect server. */
  serverUrl: string;
  /** Genesis hash of the target network requested by the application. */
  networkGenesisHash: string;
  /** Six-digit challenge shown to the user before pairing. */
  challenge: string;
  /** Human-readable status message for the current flow state. */
  status: string;
  /** Session token returned by successful pairing, if available. */
  sessionToken: string;
  /** Account address returned by a successful account request, if available. */
  accountAddress: string;
}

/**
 * Minimal interface the example app needs from the connect client package.
 */
export interface ConnectClientLike {
  /** Opens the WebSocket connection. */
  connect(): Promise<void>;
  /** Requests pairing with an application-provided challenge. */
  pair(challenge: string): Promise<PairResult>;
  /** Requests an account address for a session token and target network. */
  requestAccount(
    sessionToken: string,
    networkGenesisHash: string,
  ): Promise<string>;
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
  /** Updates the target network genesis hash. */
  setNetworkGenesisHash(networkGenesisHash: string): void;
  /** Updates the displayed pairing challenge. */
  setChallenge(challenge: string): void;
  /** Replaces the current challenge with a newly generated one. */
  regenerateChallenge(): void;
  /** Requests pairing through the connect client package. */
  pair(): Promise<void>;
  /** Requests an account address for the configured target network. */
  requestAccount(): Promise<void>;
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
  /** Optional initial target network genesis hash. */
  initialNetworkGenesisHash?: string;
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

const DEFAULT_OPTS = {
  // testnet genesis hash
  initialNetworkGenesisHash:
    "4221332d34e1694168c2a0c0b3fd0f273809612cb13d000d5c2e00e85f50f796",
} satisfies ExampleAppModelOptions;

/**
 * Creates the stateful model backing the example app.
 *
 * @param options - Optional hooks for tests and custom initialization.
 * @returns A model exposing the example integration flow.
 * @example
 * ```ts
 * const model = createExampleAppModel();
 * await model.pair();
 * await model.requestAccount();
 * ```
 */
export function createExampleAppModel(
  options: ExampleAppModelOptions = DEFAULT_OPTS,
): ExampleAppModel {
  const clientFactory = options.clientFactory ?? defaultClientFactory;
  const challengeGenerator = options.challengeGenerator ?? generateChallenge;
  const listeners = new Set<(state: ExampleAppState) => void>();

  let state: ExampleAppState = {
    serverUrl: options.initialServerUrl ?? DEFAULT_CONNECT_URL,
    networkGenesisHash: options.initialNetworkGenesisHash ?? "",
    challenge: challengeGenerator(),
    status: "Ready to pair.",
    sessionToken: "",
    accountAddress: "",
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

    setNetworkGenesisHash(networkGenesisHash: string): void {
      update({ networkGenesisHash });
    },

    setChallenge(challenge: string): void {
      update({ challenge });
    },

    regenerateChallenge(): void {
      update({ challenge: challengeGenerator() });
    },

    async pair(): Promise<void> {
      closeClient();
      update({
        status: "Connecting and requesting pairing...",
        accountAddress: "",
      });
      try {
        client = clientFactory({ url: state.serverUrl });
        await client.connect();
        const result = await client.pair(state.challenge);
        update({
          status:
            "Pairing approved. Request an account for the target network.",
          sessionToken: result.sessionToken,
        });
      } catch (error) {
        closeClient();
        update({
          status: formatErrorStatus(error),
          sessionToken: "",
          accountAddress: "",
        });
      }
    },

    async requestAccount(): Promise<void> {
      if (!client || !state.sessionToken) {
        update({ status: "No active session. Pair first." });
        return;
      }
      if (!state.networkGenesisHash) {
        update({ status: "Enter a target network genesis hash first." });
        return;
      }

      update({
        status: "Requesting account authority for the target network...",
      });
      try {
        const accountAddress = await client.requestAccount(
          state.sessionToken,
          state.networkGenesisHash,
        );
        update({ status: "Account approved.", accountAddress });
      } catch (error) {
        update({ status: formatErrorStatus(error), accountAddress: "" });
      }
    },

    reset(): void {
      closeClient();
      state = {
        serverUrl: state.serverUrl,
        networkGenesisHash: state.networkGenesisHash,
        challenge: challengeGenerator(),
        status: "Ready to pair.",
        sessionToken: "",
        accountAddress: "",
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
