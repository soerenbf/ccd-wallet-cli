/**
 * State model for the connect example app.
 *
 * This module keeps the core integration behavior framework-agnostic so tests
 * can validate the paired-session shell, deferred account-authority flow, and
 * smart-contract request preparation without depending on a browser DOM or
 * React rendering details.
 */
import {
  DEFAULT_CONNECT_URL,
  createConnectClient,
  type ConnectClient,
  type ConnectClientOptions,
  type ContractInitParams,
  type ContractInitResult,
  type ContractUpdateParams,
  type ContractUpdateResult,
  type PairResult,
} from "@ccd-wallet/connect-client";
import {
  defaultSmartContractTools,
  type PreparedSmartContractParameters,
  type SmartContractTools,
} from "./smart-contracts.js";

/**
 * Navigation targets shown in the paired example-app shell.
 */
export type ExamplePage = "smart-contracts" | "transactions" | "chain-updates";

/**
 * Smart-contract request modes supported by the example app.
 */
export type SmartContractMode = "init" | "update";

/**
 * Paired session context displayed globally once pairing succeeds.
 */
export interface PairedSessionContext {
  /** Session token returned by the wallet for the active paired session. */
  sessionToken: string;
  /** Network genesis hash bound to the active paired session. */
  networkGenesisHash: string;
}

/**
 * Account authority granted to the active paired session, when available.
 */
export interface AccountAuthorityState {
  /** Account address approved for the active session. */
  accountAddress: string;
}

/**
 * Mutable Smart Contracts page state.
 */
export interface SmartContractsState {
  /** Whether the form is preparing an init or update request. */
  mode: SmartContractMode;
  /** Module reference used for init requests. */
  moduleRef: string;
  /** Contract name used for schema lookup and receive-name construction. */
  contractName: string;
  /** Init function name sent to the connect server. */
  initName: string;
  /** Receive entrypoint name without the contract-name prefix. */
  entrypointName: string;
  /** Target contract instance index for update requests. */
  contractIndex: string;
  /** Target contract instance subindex for update requests. */
  contractSubindex: string;
  /** CCD amount attached to the request in microCCD. */
  amountMicroCcd: string;
  /** Maximum contract execution energy accepted by the dApp. */
  maxContractExecutionEnergy: string;
  /** Base64-encoded versioned module schema used for schema-aware preparation. */
  schemaBase64: string;
  /** JSON value that should match the selected init/update parameter schema. */
  parameterJson: string;
  /** Whether the wallet should simulate the contract request before prompting. */
  validate: boolean;
  /** Latest section-local status message for smart-contract preparation/submission. */
  status: string;
  /** Latest prepared hex-encoded parameter bytes, if available. */
  preparedParameterHex: string;
  /** Schema descriptor paired with the latest prepared parameter bytes. */
  preparedSchema: { base64: string } | null;
  /** Latest submitted transaction hash, if available. */
  lastTransactionHash: string;
}

/**
 * Mutable example-app state rendered in the reference UI.
 */
export interface ExampleAppState {
  /** WebSocket URL used for the local connect server. */
  serverUrl: string;
  /** Genesis hash selected before pairing and bound into the session on approval. */
  networkGenesisHash: string;
  /** Six-digit challenge shown to the user before pairing. */
  challenge: string;
  /** Human-readable status message for the current global app flow. */
  status: string;
  /** Currently selected paired-shell page. */
  currentPage: ExamplePage;
  /** Active paired session context, or `null` before pairing. */
  session: PairedSessionContext | null;
  /** Active account authority, or `null` until `requestAccount` succeeds. */
  accountAuthority: AccountAuthorityState | null;
  /** Mutable Smart Contracts page state. */
  smartContracts: SmartContractsState;
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
  /** Requests wallet-approved smart contract initialization. */
  requestContractInit(params: ContractInitParams): Promise<ContractInitResult>;
  /** Requests wallet-approved smart contract update execution. */
  requestContractUpdate(
    params: ContractUpdateParams,
  ): Promise<ContractUpdateResult>;
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
  /** Updates the target network genesis hash used for pairing. */
  setNetworkGenesisHash(networkGenesisHash: string): void;
  /** Updates the displayed pairing challenge. */
  setChallenge(challenge: string): void;
  /** Replaces the current challenge with a newly generated one. */
  regenerateChallenge(): void;
  /** Switches the active paired-shell page. */
  setCurrentPage(page: ExamplePage): void;
  /** Replaces one or more Smart Contracts page fields. */
  updateSmartContracts(patch: Partial<SmartContractsState>): void;
  /** Requests pairing through the connect client package. */
  pair(): Promise<void>;
  /** Requests account authority for the active paired session. */
  requestAccount(): Promise<void>;
  /** Prepares schema-aware smart-contract parameter bytes using `web-sdk`. */
  prepareSmartContractRequest(): void;
  /** Submits the active Smart Contracts form through the connect client. */
  submitSmartContractRequest(): Promise<void>;
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
  /** Optional smart-contract preparation helpers, primarily useful for tests. */
  smartContractTools?: SmartContractTools;
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

const DEFAULT_OPTIONS = {
  // testnet genesis hash
  initialNetworkGenesisHash:
    "4221332d34e1694168c2a0c0b3fd0f273809612cb13d000d5c2e00e85f50f796",
} satisfies ExampleAppModelOptions;

/**
 * Creates the stateful model backing the example app.
 *
 * @param options - Optional hooks for tests and custom initialization.
 * @returns A model exposing the paired-shell integration flow.
 * @example
 * ```ts
 * const model = createExampleAppModel();
 * await model.pair();
 * await model.requestAccount();
 * model.prepareSmartContractRequest();
 * await model.submitSmartContractRequest();
 * ```
 */
export function createExampleAppModel(
  options: ExampleAppModelOptions = DEFAULT_OPTIONS,
): ExampleAppModel {
  const clientFactory = options.clientFactory ?? defaultClientFactory;
  const challengeGenerator = options.challengeGenerator ?? generateChallenge;
  const smartContractTools =
    options.smartContractTools ?? defaultSmartContractTools;
  const listeners = new Set<(state: ExampleAppState) => void>();

  let state: ExampleAppState = {
    serverUrl: options.initialServerUrl ?? DEFAULT_CONNECT_URL,
    networkGenesisHash: options.initialNetworkGenesisHash ?? "",
    challenge: challengeGenerator(),
    status: "Ready to pair.",
    currentPage: "smart-contracts",
    session: null,
    accountAuthority: null,
    smartContracts: createDefaultSmartContractsState(),
  };
  let client: ConnectClientLike | undefined;

  const emit = (): void => {
    for (const listener of listeners) {
      listener(cloneState(state));
    }
  };

  const update = (patch: Partial<ExampleAppState>): void => {
    state = { ...state, ...patch };
    emit();
  };

  const updateSmartContracts = (patch: Partial<SmartContractsState>): void => {
    state = {
      ...state,
      smartContracts: {
        ...state.smartContracts,
        ...patch,
      },
    };
    emit();
  };

  const closeClient = (): void => {
    client?.close();
    client = undefined;
  };

  const prepareSmartContractRequest = (): PreparedSmartContractParameters | null => {
    try {
      const prepared =
        state.smartContracts.mode === "init"
          ? smartContractTools.prepareInit({
              schemaBase64: state.smartContracts.schemaBase64,
              contractName: state.smartContracts.contractName,
              parameterJson: state.smartContracts.parameterJson,
            })
          : smartContractTools.prepareUpdate({
              schemaBase64: state.smartContracts.schemaBase64,
              contractName: state.smartContracts.contractName,
              entrypointName: state.smartContracts.entrypointName,
              parameterJson: state.smartContracts.parameterJson,
            });
      updateSmartContracts({
        preparedParameterHex: prepared.parameterHex,
        preparedSchema: prepared.schema,
        status: `Prepared ${state.smartContracts.mode} request parameter bytes from schema-aware JSON.`,
        lastTransactionHash: state.smartContracts.lastTransactionHash,
      });
      update({
        status:
          "Smart contract parameters prepared. Review the payload and submit the request when ready.",
      });
      return prepared;
    } catch (error) {
      updateSmartContracts({
        preparedParameterHex: "",
        preparedSchema: null,
        status: formatErrorStatus(error),
        lastTransactionHash: state.smartContracts.lastTransactionHash,
      });
      update({ status: formatErrorStatus(error) });
      return null;
    }
  };

  return {
    getState(): ExampleAppState {
      return cloneState(state);
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

    setCurrentPage(page: ExamplePage): void {
      update({ currentPage: page });
    },

    updateSmartContracts(patch: Partial<SmartContractsState>): void {
      const resetsPreparation =
        patch.mode !== undefined ||
        patch.schemaBase64 !== undefined ||
        patch.contractName !== undefined ||
        patch.entrypointName !== undefined ||
        patch.parameterJson !== undefined;
      updateSmartContracts({
        ...patch,
        ...(resetsPreparation
          ? {
              preparedParameterHex: "",
              preparedSchema: null,
              lastTransactionHash: patch.lastTransactionHash ?? "",
            }
          : {}),
      });
    },

    async pair(): Promise<void> {
      if (!state.networkGenesisHash.trim()) {
        update({ status: "Enter a target network genesis hash first." });
        return;
      }

      closeClient();
      update({
        status: "Connecting and requesting pairing...",
        session: null,
        accountAuthority: null,
        currentPage: "smart-contracts",
        smartContracts: createDefaultSmartContractsState(),
      });
      try {
        client = clientFactory({ url: state.serverUrl });
        await client.connect();
        const result = await client.pair(state.challenge);
        update({
          status:
            "Pairing approved. Session established for the selected network.",
          session: {
            sessionToken: result.sessionToken,
            networkGenesisHash: state.networkGenesisHash,
          },
          accountAuthority: null,
          currentPage: "smart-contracts",
        });
      } catch (error) {
        closeClient();
        update({
          status: formatErrorStatus(error),
          session: null,
          accountAuthority: null,
        });
      }
    },

    async requestAccount(): Promise<void> {
      if (!client || !state.session) {
        update({ status: "No active session. Pair first." });
        return;
      }

      update({
        status:
          "Requesting account authority for the active paired session...",
      });
      try {
        const accountAddress = await client.requestAccount(
          state.session.sessionToken,
          state.session.networkGenesisHash,
        );
        update({
          status: "Account authority approved for the active session.",
          accountAuthority: { accountAddress },
        });
      } catch (error) {
        update({
          status: formatErrorStatus(error),
          accountAuthority: null,
        });
      }
    },

    prepareSmartContractRequest(): void {
      prepareSmartContractRequest();
    },

    async submitSmartContractRequest(): Promise<void> {
      if (!client || !state.session) {
        update({ status: "No active session. Pair first." });
        return;
      }
      if (!state.accountAuthority) {
        update({
          status:
            "Account authority is required before Smart Contracts requests can be submitted.",
        });
        updateSmartContracts({
          status:
            "Request account authority to enable Smart Contracts requests.",
        });
        return;
      }

      const prepared = state.smartContracts.preparedParameterHex
        ? {
            parameterHex: state.smartContracts.preparedParameterHex,
            schema: state.smartContracts.preparedSchema ?? {
              base64: state.smartContracts.schemaBase64,
            },
          }
        : prepareSmartContractRequest();
      if (!prepared) {
        return;
      }

      try {
        if (state.smartContracts.mode === "init") {
          update({ status: "Submitting smart contract init request..." });
          const result = await client.requestContractInit({
            sessionToken: state.session.sessionToken,
            moduleRef: state.smartContracts.moduleRef.trim(),
            initName: state.smartContracts.initName.trim(),
            amountMicroCcd: state.smartContracts.amountMicroCcd.trim(),
            maxContractExecutionEnergy: parseUnsignedInteger(
              state.smartContracts.maxContractExecutionEnergy,
              "maxContractExecutionEnergy",
            ),
            parameterHex: prepared.parameterHex,
            schema: prepared.schema,
            validate: state.smartContracts.validate,
          });
          update({
            status:
              "Smart contract init request approved. The wallet returned a transaction hash.",
          });
          updateSmartContracts({
            status: "Init request submitted through @ccd-wallet/connect-client.",
            lastTransactionHash: result.transactionHash,
          });
          return;
        }

        update({ status: "Submitting smart contract update request..." });
        const receiveName = `${state.smartContracts.contractName.trim()}.${state.smartContracts.entrypointName.trim()}`;
        const result = await client.requestContractUpdate({
          sessionToken: state.session.sessionToken,
          contractAddress: {
            index: parseUnsignedInteger(
              state.smartContracts.contractIndex,
              "contractAddress.index",
            ),
            subindex: parseUnsignedInteger(
              state.smartContracts.contractSubindex,
              "contractAddress.subindex",
            ),
          },
          receiveName,
          amountMicroCcd: state.smartContracts.amountMicroCcd.trim(),
          maxContractExecutionEnergy: parseUnsignedInteger(
            state.smartContracts.maxContractExecutionEnergy,
            "maxContractExecutionEnergy",
          ),
          parameterHex: prepared.parameterHex,
          schema: prepared.schema,
          validate: state.smartContracts.validate,
        });
        update({
          status:
            "Smart contract update request approved. The wallet returned a transaction hash.",
        });
        updateSmartContracts({
          status: "Update request submitted through @ccd-wallet/connect-client.",
          lastTransactionHash: result.transactionHash,
        });
      } catch (error) {
        update({ status: formatErrorStatus(error) });
        updateSmartContracts({ status: formatErrorStatus(error) });
      }
    },

    reset(): void {
      closeClient();
      state = {
        serverUrl: state.serverUrl,
        networkGenesisHash: state.networkGenesisHash,
        challenge: challengeGenerator(),
        status: "Ready to pair.",
        currentPage: "smart-contracts",
        session: null,
        accountAuthority: null,
        smartContracts: createDefaultSmartContractsState(),
      };
      emit();
    },

    subscribe(listener: (state: ExampleAppState) => void): () => void {
      listeners.add(listener);
      listener(cloneState(state));
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

function createDefaultSmartContractsState(): SmartContractsState {
  return {
    mode: "init",
    moduleRef: "",
    contractName: "",
    initName: "init_my_contract",
    entrypointName: "set",
    contractIndex: "0",
    contractSubindex: "0",
    amountMicroCcd: "0",
    maxContractExecutionEnergy: "30000",
    schemaBase64: "",
    parameterJson: "{}",
    validate: true,
    status:
      "Provide a schema, JSON value, and request details to prepare a Smart Contracts payload.",
    preparedParameterHex: "",
    preparedSchema: null,
    lastTransactionHash: "",
  };
}

function cloneState(state: ExampleAppState): ExampleAppState {
  return {
    ...state,
    session: state.session ? { ...state.session } : null,
    accountAuthority: state.accountAuthority
      ? { ...state.accountAuthority }
      : null,
    smartContracts: {
      ...state.smartContracts,
      preparedSchema: state.smartContracts.preparedSchema
        ? { ...state.smartContracts.preparedSchema }
        : null,
    },
  };
}

function defaultClientFactory(options: ConnectClientOptions): ConnectClient {
  return createConnectClient(options);
}

function parseUnsignedInteger(value: string, fieldName: string): number {
  const parsed = Number.parseInt(value.trim(), 10);
  if (!Number.isSafeInteger(parsed) || parsed < 0) {
    throw new Error(`${fieldName} must be a non-negative integer.`);
  }
  return parsed;
}

function formatErrorStatus(error: unknown): string {
  if (error instanceof Error) {
    return `Error: ${error.message}`;
  }
  return "Error: Unknown failure";
}
