/**
 * Embedded-schema-aware smart contract helpers for the connect example app.
 *
 * This module keeps `@concordium/web-sdk` usage isolated to the example
 * application so `@ccd-wallet/connect-client` stays focused on transport and
 * protocol concerns.
 */
import { ConcordiumGRPCWebClient } from "@concordium/web-sdk/grpc";
import {
  serializeInitContractParameters,
  serializeUpdateContractParameters,
} from "@concordium/web-sdk/schema";
import {
  ContractAddress,
  ContractName,
  EntrypointName,
  ModuleReference,
  type ContractAddress as ContractAddressNamespace,
} from "@concordium/web-sdk/types";

/**
 * Input needed to prepare init-contract parameters from schema-aware JSON.
 */
export interface SmartContractInitPreparation {
  /** Browser-reachable gRPC-web node endpoint used for module lookup. */
  nodeEndpoint: string;
  /** Module reference whose embedded schema should be used for serialization. */
  moduleRef: string;
  /** Init function name, typically `init_<contractName>`. */
  initName: string;
  /** JSON value matching the init parameter schema. */
  parameterJson: string;
}

/**
 * Input needed to prepare update-contract parameters from schema-aware JSON.
 */
export interface SmartContractUpdatePreparation {
  /** Browser-reachable gRPC-web node endpoint used for instance/module lookup. */
  nodeEndpoint: string;
  /** Contract instance index. */
  contractIndex: string;
  /** Contract instance subindex. */
  contractSubindex: string;
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
  /** Schema descriptor passed back to the wallet for human-readable rendering when available. */
  schema: { base64: string } | null;
  /** Parsed JSON value used for serialization. */
  parameterJson: unknown;
  /** Contract name used during schema-aware serialization. */
  contractName: string;
  /** Module reference from which the embedded schema was derived. */
  moduleRef: string;
}

/**
 * Small abstraction layer for embedded-schema preparation.
 *
 * Tests can inject a fake implementation while the real UI uses the
 * `@concordium/web-sdk`-backed default tools.
 */
export interface SmartContractTools {
  /** Prepares init parameters from node-derived embedded schema and JSON input. */
  prepareInit(
    input: SmartContractInitPreparation,
  ): Promise<PreparedSmartContractParameters>;
  /** Prepares update parameters from node-derived embedded schema and JSON input. */
  prepareUpdate(
    input: SmartContractUpdatePreparation,
  ): Promise<PreparedSmartContractParameters>;
}

/**
 * Default `@concordium/web-sdk`-backed helper implementation.
 */
export const defaultSmartContractTools: SmartContractTools = {
  prepareInit: prepareInitContractParameters,
  prepareUpdate: prepareUpdateContractParameters,
};

/**
 * Prepares init-contract parameters from embedded module schema.
 *
 * @param input - Init preparation input.
 * @returns The prepared connect-request payload pieces.
 * @throws {Error} If node access, module lookup, schema lookup, or JSON parsing fails.
 */
export async function prepareInitContractParameters(
  input: SmartContractInitPreparation,
): Promise<PreparedSmartContractParameters> {
  const nodeClient = createNodeClient(input.nodeEndpoint);
  const moduleReference = ModuleReference.fromHexString(
    normalizeHex(input.moduleRef, "moduleRef"),
  );
  const rawSchema = await nodeClient.getEmbeddedSchema(moduleReference);
  if (!rawSchema) {
    throw new Error(
      "The selected module does not expose an embedded schema. This showcase supports only contracts with embedded schema.",
    );
  }

  const parameterJson = parseJsonValue(input.parameterJson);
  const contractName = contractNameFromInitName(input.initName);
  const serialized = serializeInitContractParameters(
    asContractName(contractName),
    parameterJson,
    rawSchema.buffer,
    rawSchema.type === "unversioned" ? rawSchema.version : undefined,
    true,
  );

  return {
    parameterHex: toHexString(serialized),
    schema: rawSchema.type === "versioned" ? { base64: toBase64(rawSchema.buffer) } : null,
    parameterJson,
    contractName,
    moduleRef: moduleReference.toString(),
  };
}

/**
 * Prepares update-contract parameters from a target instance's embedded module schema.
 *
 * @param input - Update preparation input.
 * @returns The prepared connect-request payload pieces.
 * @throws {Error} If node access, instance lookup, schema lookup, or JSON parsing fails.
 */
export async function prepareUpdateContractParameters(
  input: SmartContractUpdatePreparation,
): Promise<PreparedSmartContractParameters> {
  const nodeClient = createNodeClient(input.nodeEndpoint);
  const contractAddress = ContractAddress.create(
    parseUnsignedBigInt(input.contractIndex, "contractIndex"),
    parseUnsignedBigInt(input.contractSubindex, "contractSubindex"),
  );
  const instanceInfo = await nodeClient.getInstanceInfo(contractAddress);
  const rawSchema = await nodeClient.getEmbeddedSchema(instanceInfo.sourceModule);
  if (!rawSchema) {
    throw new Error(
      "The target contract module does not expose an embedded schema. This showcase supports only contracts with embedded schema.",
    );
  }

  const parameterJson = parseJsonValue(input.parameterJson);
  const contractName = contractNameFromInitName(instanceInfo.name.toString());
  const serialized = serializeUpdateContractParameters(
    asContractName(contractName),
    asEntrypointName(input.entrypointName),
    parameterJson,
    rawSchema.buffer,
    rawSchema.type === "unversioned" ? rawSchema.version : undefined,
    true,
  );

  return {
    parameterHex: toHexString(serialized),
    schema: rawSchema.type === "versioned" ? { base64: toBase64(rawSchema.buffer) } : null,
    parameterJson,
    contractName,
    moduleRef: instanceInfo.sourceModule.toString(),
  };
}

interface NodeClientLike {
  getEmbeddedSchema(
    moduleRef: Parameters<ConcordiumGRPCWebClient["getEmbeddedSchema"]>[0],
  ): ReturnType<ConcordiumGRPCWebClient["getEmbeddedSchema"]>;
  getInstanceInfo(
    contractAddress: ContractAddressNamespace.Type,
  ): ReturnType<ConcordiumGRPCWebClient["getInstanceInfo"]>;
}

function createNodeClient(nodeEndpoint: string): NodeClientLike {
  const endpoint = parseNodeEndpoint(nodeEndpoint);
  return new ConcordiumGRPCWebClient(endpoint.address, endpoint.port);
}

function parseNodeEndpoint(nodeEndpoint: string): {
  address: string;
  port: number;
} {
  let url: URL;
  try {
    url = new URL(nodeEndpoint.trim());
  } catch {
    throw new Error(
      "Enter a valid browser-reachable gRPC-web node endpoint, for example http://127.0.0.1:20000.",
    );
  }

  if (!(url.protocol === "http:" || url.protocol === "https:")) {
    throw new Error("The node endpoint must use http:// or https://.");
  }
  if (!url.hostname) {
    throw new Error("The node endpoint must include a hostname.");
  }
  if (url.pathname && url.pathname !== "/") {
    throw new Error("The node endpoint must not include a path.");
  }
  if (url.search || url.hash) {
    throw new Error("The node endpoint must not include query or hash parts.");
  }

  return {
    address: `${url.protocol}//${url.hostname}`,
    port: url.port
      ? Number.parseInt(url.port, 10)
      : url.protocol === "https:"
        ? 443
        : 80,
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

function normalizeHex(value: string, fieldName: string): string {
  const normalized = value.trim().replace(/^0x/i, "");
  if (!/^[0-9a-f]+$/i.test(normalized)) {
    throw new Error(`${fieldName} must be a hex string.`);
  }
  return normalized;
}

function contractNameFromInitName(initName: string): string {
  const normalized = initName.trim();
  if (!normalized) {
    throw new Error("Enter an init name or target contract first.");
  }
  return normalized.startsWith("init_") ? normalized.slice(5) : normalized;
}

function parseUnsignedBigInt(value: string, fieldName: string): bigint {
  const normalized = value.trim();
  if (!/^\d+$/.test(normalized)) {
    throw new Error(`${fieldName} must be a non-negative integer.`);
  }
  return BigInt(normalized);
}

function toBase64(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return globalThis.btoa(binary);
}

function asContractName(
  value: string,
): Parameters<typeof serializeInitContractParameters>[0] {
  const normalized = value.trim();
  if (!normalized) {
    throw new Error("Contract name must not be empty.");
  }
  return ContractName.fromString(normalized);
}

function asEntrypointName(
  value: string,
): Parameters<typeof serializeUpdateContractParameters>[1] {
  const normalized = value.trim();
  if (!normalized) {
    throw new Error("Entrypoint name must not be empty.");
  }
  return EntrypointName.fromString(normalized);
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
