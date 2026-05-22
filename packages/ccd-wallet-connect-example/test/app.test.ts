import test from "node:test";
import assert from "node:assert/strict";

import {
  createExampleAppModel,
  type ConnectClientFactory,
  type ConnectClientLike,
} from "../src/app.ts";

class FakeClient implements ConnectClientLike {
  connected = false;
  pairingCalls: string[] = [];
  refreshCalls: string[] = [];
  closeCalls = 0;

  async connect(): Promise<void> {
    this.connected = true;
  }

  async pair(challenge: string) {
    this.pairingCalls.push(challenge);
    return {
      sessionToken: "session-token",
      context: {
        networkGenesisHash: "genesis",
        accountAddress: "addr",
      },
    };
  }

  async getSessionContext(sessionToken: string) {
    this.refreshCalls.push(sessionToken);
    return {
      networkGenesisHash: "genesis-2",
      accountAddress: "addr-2",
    };
  }

  close(): void {
    this.closeCalls += 1;
  }
}

test("pairing uses the client package flow and stores approved session data", async () => {
  const fakeClient = new FakeClient();
  const model = createExampleAppModel({
    clientFactory: (() => fakeClient) satisfies ConnectClientFactory,
    challengeGenerator: () => "123456",
  });

  await model.pair();

  assert.deepEqual(fakeClient.pairingCalls, ["123456"]);
  assert.deepEqual(model.getState(), {
    serverUrl: "ws://127.0.0.1:22771",
    challenge: "123456",
    status: "Pairing approved.",
    sessionToken: "session-token",
    context: {
      networkGenesisHash: "genesis",
      accountAddress: "addr",
    },
  });
});

test("refresh retrieves approved session context again", async () => {
  const fakeClient = new FakeClient();
  const model = createExampleAppModel({
    clientFactory: (() => fakeClient) satisfies ConnectClientFactory,
    challengeGenerator: () => "123456",
  });

  await model.pair();
  await model.refresh();

  assert.deepEqual(fakeClient.refreshCalls, ["session-token"]);
  assert.deepEqual(model.getState().context, {
    networkGenesisHash: "genesis-2",
    accountAddress: "addr-2",
  });
  assert.equal(model.getState().status, "Session context refreshed.");
});

test("reset clears local session state and regenerates the challenge", async () => {
  const fakeClient = new FakeClient();
  let challenges = ["123456", "654321"];
  const model = createExampleAppModel({
    clientFactory: (() => fakeClient) satisfies ConnectClientFactory,
    challengeGenerator: () => challenges.shift() ?? "000000",
  });

  await model.pair();
  model.reset();

  assert.equal(fakeClient.closeCalls, 1);
  assert.deepEqual(model.getState(), {
    serverUrl: "ws://127.0.0.1:22771",
    challenge: "654321",
    status: "Ready to pair.",
    sessionToken: "",
    context: null,
  });
});
