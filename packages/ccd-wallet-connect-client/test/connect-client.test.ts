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

test("pair sends JSON-RPC pair request and resolves session data", async () => {
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
      context: {
        networkGenesisHash: "genesis",
        accountAddress: "addr",
      },
    },
  });

  assert.deepEqual(await paired, {
    sessionToken: "session-token",
    context: {
      networkGenesisHash: "genesis",
      accountAddress: "addr",
    },
  });
});

test("getSessionContext sends JSON-RPC session.getContext request", async () => {
  resetMockSockets();
  const client = new ConnectClient({ WebSocket: MockWebSocket });
  const connected = client.connect();
  const socket = MockWebSocket.instances[0];
  assert.ok(socket);
  socket.open();
  await connected;

  const context = client.getSessionContext("session-token");
  assert.deepEqual(JSON.parse(socket.sent[0] ?? "{}"), {
    jsonrpc: "2.0",
    id: 1,
    method: "session.getContext",
    params: { sessionToken: "session-token" },
  });

  socket.receive({
    jsonrpc: "2.0",
    id: 1,
    result: {
      networkGenesisHash: "genesis",
      accountAddress: "addr",
    },
  });

  assert.deepEqual(await context, {
    networkGenesisHash: "genesis",
    accountAddress: "addr",
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
