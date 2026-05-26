/**
 * Schema-aware smart contract helpers for the connect example app.
 *
 * This module keeps `@concordium/web-sdk` usage isolated to the example
 * application so `@ccd-wallet/connect-client` stays focused on transport and
 * protocol concerns.
 */
import {
  serializeInitContractParameters,
  serializeUpdateContractParameters,
} from "@concordium/web-sdk/schema";

/**
 * Input needed to prepare init-contract parameters from schema-aware JSON.
 */
export interface SmartContractInitPreparation {
  /** Base64-encoded versioned module schema. */
  schemaBase64: string;
  /** Contract name used for schema lookup, without the `init_` prefix. */
  contractName: string;
  /** JSON value matching the init parameter schema. */
  parameterJson: string;
}

/**
 * Input needed to prepare update-contract parameters from schema-aware JSON.
 */
export interface SmartContractUpdatePreparation {
  /** Base64-encoded versioned module schema. */
  schemaBase64: string;
  /** Contract name used for schema lookup. */
  contractName: string;
  /** Receive entrypoint name without the contract-name prefix. */
  entrypointName: string;
  /** JSON value matching the receive parameter schema. */
  parameterJson: string;
}

/**
 * Schema-prepared parameter payload ready to send through connect requests.
 */
export interface PreparedSmartContractParameters {
  /** Hex-encoded serialized parameter bytes without a `0x` prefix. */
  parameterHex: string;
  /** Schema descriptor passed back to the wallet for human-readable rendering. */
  schema: { base64: string };
  /** Parsed JSON value used for serialization. */
  parameterJson: unknown;
}

/**
 * Small abstraction layer for schema-aware preparation.
 *
 * Tests can inject a fake implementation while the real UI uses the
 * `@concordium/web-sdk`-backed default tools.
 */
export interface SmartContractTools {
  /** Prepares init parameters from schema-aware JSON input. */
  prepareInit(
    input: SmartContractInitPreparation,
  ): PreparedSmartContractParameters;
  /** Prepares update parameters from schema-aware JSON input. */
  prepareUpdate(
    input: SmartContractUpdatePreparation,
  ): PreparedSmartContractParameters;
}

/**
 * Default `@concordium/web-sdk`-backed helper implementation.
 */
export const defaultSmartContractTools: SmartContractTools = {
  prepareInit: prepareInitContractParameters,
  prepareUpdate: prepareUpdateContractParameters,
};

/**
 * Prepares init-contract parameters from schema-aware JSON input.
 *
 * @param input - Init preparation input.
 * @returns The prepared connect-request payload pieces.
 * @throws {Error} If the schema or JSON input cannot be parsed or serialized.
 */
export function prepareInitContractParameters(
  input: SmartContractInitPreparation,
): PreparedSmartContractParameters {
  const normalizedSchema = normalizeSchemaBase64(input.schemaBase64);
  const schemaBytes = decodeBase64(normalizedSchema);
  const parameterJson = parseJsonValue(input.parameterJson);
  const serialized = serializeInitContractParameters(
    asContractName(input.contractName),
    parameterJson,
    toExactArrayBuffer(schemaBytes),
  );

  return {
    parameterHex: toHexString(serialized),
    schema: { base64: normalizedSchema },
    parameterJson,
  };
}

/**
 * Prepares update-contract parameters from schema-aware JSON input.
 *
 * @param input - Update preparation input.
 * @returns The prepared connect-request payload pieces.
 * @throws {Error} If the schema or JSON input cannot be parsed or serialized.
 */
export function prepareUpdateContractParameters(
  input: SmartContractUpdatePreparation,
): PreparedSmartContractParameters {
  const normalizedSchema = normalizeSchemaBase64(input.schemaBase64);
  const schemaBytes = decodeBase64(normalizedSchema);
  const parameterJson = parseJsonValue(input.parameterJson);
  const serialized = serializeUpdateContractParameters(
    asContractName(input.contractName),
    asEntrypointName(input.entrypointName),
    parameterJson,
    toExactArrayBuffer(schemaBytes),
  );

  return {
    parameterHex: toHexString(serialized),
    schema: { base64: normalizedSchema },
    parameterJson,
  };
}

function parseJsonValue(source: string): unknown {
  if (!source.trim()) {
    throw new Error(
      "Enter a JSON value that matches the selected smart contract schema.",
    );
  }
  return JSON.parse(source);
}

function normalizeSchemaBase64(schemaBase64: string): string {
  const normalized = schemaBase64.replace(/\s+/g, "").trim();
  if (!normalized) {
    throw new Error("Enter a base64-encoded versioned module schema.");
  }
  const padding = normalized.length % 4;
  if (padding === 0) {
    return normalized;
  }
  return `${normalized}${"=".repeat(4 - padding)}`;
}

function decodeBase64(value: string): Uint8Array {
  const decoded = globalThis.atob(value);
  return Uint8Array.from(decoded, (character) => character.charCodeAt(0));
}

function toExactArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  return Uint8Array.from(bytes).buffer;
}

function asContractName(
  value: string,
): Parameters<typeof serializeInitContractParameters>[0] {
  return value.trim() as unknown as Parameters<typeof serializeInitContractParameters>[0];
}

function asEntrypointName(
  value: string,
): Parameters<typeof serializeUpdateContractParameters>[1] {
  return value.trim() as unknown as Parameters<typeof serializeUpdateContractParameters>[1];
}

function toHexString(value: unknown): string {
  if (typeof value === "string") {
    return value.startsWith("0x") ? value.slice(2) : value;
  }
  if (value instanceof Uint8Array) {
    return bytesToHex(value);
  }
  if (value instanceof ArrayBuffer) {
    return bytesToHex(new Uint8Array(value));
  }
  if (ArrayBuffer.isView(value)) {
    return bytesToHex(
      new Uint8Array(value.buffer, value.byteOffset, value.byteLength),
    );
  }
  const stringified = String(value);
  if (/^(0x)?[0-9a-f]+$/i.test(stringified)) {
    return stringified.startsWith("0x") ? stringified.slice(2) : stringified;
  }
  throw new Error("web-sdk returned an unsupported parameter representation.");
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join(
    "",
  );
}
