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
  accountCalls: Array<{ sessionToken: string; networkGenesisHash: string }> = [];
  closeCalls = 0;

  async connect(): Promise<void> {
    this.connected = true;
  }

  async pair(challenge: string) {
    this.pairingCalls.push(challenge);
    return {
      sessionToken: "session-token",
    };
  }

  async requestAccount(sessionToken: string, networkGenesisHash: string) {
    this.accountCalls.push({ sessionToken, networkGenesisHash });
    return "addr";
  }

  close(): void {
    this.closeCalls += 1;
  }
}

test("pairing establishes a session token first", async () => {
  const fakeClient = new FakeClient();
  const model = createExampleAppModel({
    clientFactory: (() => fakeClient) satisfies ConnectClientFactory,
    challengeGenerator: () => "123456",
    initialNetworkGenesisHash: "genesis",
  });

  await model.pair();

  assert.deepEqual(fakeClient.pairingCalls, ["123456"]);
  assert.deepEqual(model.getState(), {
    serverUrl: "ws://127.0.0.1:22771",
    networkGenesisHash: "genesis",
    challenge: "123456",
    status: "Pairing approved. Request an account for the target network.",
    sessionToken: "session-token",
    accountAddress: "",
  });
});

test("requestAccount requests account authority for the target network", async () => {
  const fakeClient = new FakeClient();
  const model = createExampleAppModel({
    clientFactory: (() => fakeClient) satisfies ConnectClientFactory,
    challengeGenerator: () => "123456",
    initialNetworkGenesisHash: "genesis",
  });

  await model.pair();
  await model.requestAccount();

  assert.deepEqual(fakeClient.accountCalls, [
    { sessionToken: "session-token", networkGenesisHash: "genesis" },
  ]);
  assert.equal(model.getState().accountAddress, "addr");
  assert.equal(model.getState().status, "Account approved.");
});

test("reset clears local session state and regenerates the challenge", async () => {
  const fakeClient = new FakeClient();
  let challenges = ["123456", "654321"];
  const model = createExampleAppModel({
    clientFactory: (() => fakeClient) satisfies ConnectClientFactory,
    challengeGenerator: () => challenges.shift() ?? "000000",
    initialNetworkGenesisHash: "genesis",
  });

  await model.pair();
  await model.requestAccount();
  model.reset();

  assert.equal(fakeClient.closeCalls, 1);
  assert.deepEqual(model.getState(), {
    serverUrl: "ws://127.0.0.1:22771",
    networkGenesisHash: "genesis",
    challenge: "654321",
    status: "Ready to pair.",
    sessionToken: "",
    accountAddress: "",
  });
});
