import test from "node:test";
import assert from "node:assert/strict";

import {
  ConnectClient,
  ConnectClientError,
  DEFAULT_CONNECT_URL,
  createConnectClient,
  type WebSocketLike,
} from "../src/index.ts";

class MockWebSocket implements WebSocketLike {
  static instances: MockWebSocket[] = [];
  static readonly OPEN = 1;

  readyState = 0;
  onopen: ((event: unknown) => void) | null = null;
  onmessage: ((event: { data: unknown }) => void) | null = null;
  onerror: ((event: unknown) => void) | null = null;
  onclose: ((event: unknown) => void) | null = null;
  readonly sent: string[] = [];

  readonly url: string;

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  send(data: string): void {
    this.sent.push(data);
  }

  close(): void {
    this.readyState = 3;
    this.onclose?.({ type: "close" });
  }

  open(): void {
    this.readyState = MockWebSocket.OPEN;
    this.onopen?.({ type: "open" });
  }

  receive(value: unknown): void {
    this.onmessage?.({ data: JSON.stringify(value) });
  }
}

function resetMockSockets(): void {
  MockWebSocket.instances = [];
}

test("exports the default connect URL", () => {
  assert.equal(DEFAULT_CONNECT_URL, "ws://127.0.0.1:22771");
});

test("connect opens the configured WebSocket URL", async () => {
  resetMockSockets();
  const client = createConnectClient({
    url: "ws://127.0.0.1:9999",
    WebSocket: MockWebSocket,
  });
  const connected = client.connect();
  const socket = MockWebSocket.instances[0];
  assert.ok(socket);
  assert.equal(socket.url, "ws://127.0.0.1:9999");
  socket.open();
  await connected;
  assert.equal(client.isConnected, true);
});

test("pair sends JSON-RPC pair request and resolves session token", async () => {
  resetMockSockets();
  const client = new ConnectClient({ WebSocket: MockWebSocket });
  const connected = client.connect();
  const socket = MockWebSocket.instances[0];
  assert.ok(socket);
  socket.open();
  await connected;

  const paired = client.pair("123456");
  assert.deepEqual(JSON.parse(socket.sent[0] ?? "{}"), {
    jsonrpc: "2.0",
    id: 1,
    method: "pair",
    params: { challenge: "123456" },
  });

  socket.receive({
    jsonrpc: "2.0",
    id: 1,
    result: {
      sessionToken: "session-token",
    },
  });

  assert.deepEqual(await paired, {
    sessionToken: "session-token",
  });
});

test("requestAccount sends JSON-RPC requestAccount request", async () => {
  resetMockSockets();
  const client = new ConnectClient({ WebSocket: MockWebSocket });
  const connected = client.connect();
  const socket = MockWebSocket.instances[0];
  assert.ok(socket);
  socket.open();
  await connected;

  const accountAddress = client.requestAccount("session-token", "genesis");
  assert.deepEqual(JSON.parse(socket.sent[0] ?? "{}"), {
    jsonrpc: "2.0",
    id: 1,
    method: "requestAccount",
    params: {
      sessionToken: "session-token",
      networkGenesisHash: "genesis",
    },
  });

  socket.receive({
    jsonrpc: "2.0",
    id: 1,
    result: {
      accountAddress: "addr",
    },
  });

  assert.equal(await accountAddress, "addr");
});

test("requestContractInit sends JSON-RPC requestContractInit request", async () => {
  resetMockSockets();
  const client = new ConnectClient({ WebSocket: MockWebSocket });
  const connected = client.connect();
  const socket = MockWebSocket.instances[0];
  assert.ok(socket);
  socket.open();
  await connected;

  const result = client.requestContractInit({
    sessionToken: "session-token",
    moduleRef: "module-ref",
    initName: "init_contract",
    amountMicroCcd: "0",
    maxContractExecutionEnergy: 30000,
    parameterHex: "2a",
    validate: true,
  });
  assert.deepEqual(JSON.parse(socket.sent[0] ?? "{}"), {
    jsonrpc: "2.0",
    id: 1,
    method: "requestContractInit",
    params: {
      sessionToken: "session-token",
      moduleRef: "module-ref",
      initName: "init_contract",
      amountMicroCcd: "0",
      maxContractExecutionEnergy: 30000,
      parameterHex: "2a",
      validate: true,
    },
  });

  socket.receive({
    jsonrpc: "2.0",
    id: 1,
    result: { transactionHash: "tx-init" },
  });

  assert.deepEqual(await result, { transactionHash: "tx-init" });
});

test("requestContractUpdate sends JSON-RPC requestContractUpdate request", async () => {
  resetMockSockets();
  const client = new ConnectClient({ WebSocket: MockWebSocket });
  const connected = client.connect();
  const socket = MockWebSocket.instances[0];
  assert.ok(socket);
  socket.open();
  await connected;

  const result = client.requestContractUpdate({
    sessionToken: "session-token",
    contractAddress: { index: 42, subindex: 0 },
    receiveName: "contract.receive",
    amountMicroCcd: "1",
    maxContractExecutionEnergy: 30000,
    parameterHex: "2a",
  });
  assert.deepEqual(JSON.parse(socket.sent[0] ?? "{}"), {
    jsonrpc: "2.0",
    id: 1,
    method: "requestContractUpdate",
    params: {
      sessionToken: "session-token",
      contractAddress: { index: 42, subindex: 0 },
      receiveName: "contract.receive",
      amountMicroCcd: "1",
      maxContractExecutionEnergy: 30000,
      parameterHex: "2a",
    },
  });

  socket.receive({
    jsonrpc: "2.0",
    id: 1,
    result: { transactionHash: "tx-update" },
  });

  assert.deepEqual(await result, { transactionHash: "tx-update" });
});

test("contract request server errors reject with ConnectClientError", async () => {
  resetMockSockets();
  const client = new ConnectClient({ WebSocket: MockWebSocket });
  const connected = client.connect();
  const socket = MockWebSocket.instances[0];
  assert.ok(socket);
  socket.open();
  await connected;

  const result = client.requestContractUpdate({
    sessionToken: "session-token",
    contractAddress: { index: 42, subindex: 0 },
    receiveName: "contract.receive",
    amountMicroCcd: "1",
    maxContractExecutionEnergy: 30000,
    parameterHex: "2a",
  });
  socket.receive({
    jsonrpc: "2.0",
    id: 1,
    error: { code: -32004, message: "contract update declined by user" },
  });

  await assert.rejects(result, (error: unknown) => {
    assert.equal(error instanceof ConnectClientError, true);
    assert.equal(
      (error as ConnectClientError).message,
      "contract update declined by user",
    );
    assert.equal((error as ConnectClientError).code, -32004);
    return true;
  });
});

test("JSON-RPC error responses reject with ConnectClientError", async () => {
  resetMockSockets();
  const client = new ConnectClient({ WebSocket: MockWebSocket });
  const connected = client.connect();
  const socket = MockWebSocket.instances[0];
  assert.ok(socket);
  socket.open();
  await connected;

  const paired = client.pair("123456");
  socket.receive({
    jsonrpc: "2.0",
    id: 1,
    error: { code: -32000, message: "rejected by user" },
  });

  await assert.rejects(paired, (error: unknown) => {
    assert.equal(error instanceof ConnectClientError, true);
    assert.equal((error as ConnectClientError).message, "rejected by user");
    assert.equal((error as ConnectClientError).code, -32000);
    return true;
  });
});

test("close rejects pending requests", async () => {
  resetMockSockets();
  const client = new ConnectClient({ WebSocket: MockWebSocket });
  const connected = client.connect();
  const socket = MockWebSocket.instances[0];
  assert.ok(socket);
  socket.open();
  await connected;

  const paired = client.pair("123456");
  client.close();

  await assert.rejects(paired, /WebSocket connection closed/);
  assert.equal(client.isConnected, false);
});
