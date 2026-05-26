import test from "node:test";
import assert from "node:assert/strict";

import {
  createExampleAppModel,
  type ConnectClientFactory,
  type ConnectClientLike,
} from "../src/app.ts";
import type {
  ContractInitParams,
  ContractUpdateParams,
} from "@ccd-wallet/connect-client";
import type {
  PreparedSmartContractParameters,
  SmartContractTools,
} from "../src/smart-contracts.ts";

class FakeClient implements ConnectClientLike {
  connected = false;
  pairingCalls: string[] = [];
  accountCalls: Array<{ sessionToken: string; networkGenesisHash: string }> = [];
  initCalls: ContractInitParams[] = [];
  updateCalls: ContractUpdateParams[] = [];
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

  async requestContractInit(params: ContractInitParams) {
    this.initCalls.push(params);
    return { transactionHash: "tx-init" };
  }

  async requestContractUpdate(params: ContractUpdateParams) {
    this.updateCalls.push(params);
    return { transactionHash: "tx-update" };
  }

  close(): void {
    this.closeCalls += 1;
  }
}

class FakeSmartContractTools implements SmartContractTools {
  initInputs: Array<{ schemaBase64: string; contractName: string; parameterJson: string }> = [];
  updateInputs: Array<{
    schemaBase64: string;
    contractName: string;
    entrypointName: string;
    parameterJson: string;
  }> = [];

  prepareInit(input: {
    schemaBase64: string;
    contractName: string;
    parameterJson: string;
  }): PreparedSmartContractParameters {
    this.initInputs.push(input);
    return {
      parameterHex: "deadbeef",
      schema: { base64: input.schemaBase64 },
      parameterJson: JSON.parse(input.parameterJson),
    };
  }

  prepareUpdate(input: {
    schemaBase64: string;
    contractName: string;
    entrypointName: string;
    parameterJson: string;
  }): PreparedSmartContractParameters {
    this.updateInputs.push(input);
    return {
      parameterHex: "c0ffee",
      schema: { base64: input.schemaBase64 },
      parameterJson: JSON.parse(input.parameterJson),
    };
  }
}

test("pairing establishes a paired session shell before account authority exists", async () => {
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
    status: "Pairing approved. Session established for the selected network.",
    currentPage: "smart-contracts",
    session: {
      sessionToken: "session-token",
      networkGenesisHash: "genesis",
    },
    accountAuthority: null,
    smartContracts: {
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
    },
  });
});

test("requestAccount requests account authority for the active paired session", async () => {
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
  assert.deepEqual(model.getState().accountAuthority, { accountAddress: "addr" });
  assert.equal(
    model.getState().status,
    "Account authority approved for the active session.",
  );
});

test("paired shell navigation works while deferred account authority gate remains active", async () => {
  const fakeClient = new FakeClient();
  const model = createExampleAppModel({
    clientFactory: (() => fakeClient) satisfies ConnectClientFactory,
    challengeGenerator: () => "123456",
    initialNetworkGenesisHash: "genesis",
  });

  await model.pair();
  model.setCurrentPage("transactions");
  assert.equal(model.getState().currentPage, "transactions");

  model.setCurrentPage("smart-contracts");
  await model.submitSmartContractRequest();

  assert.equal(model.getState().currentPage, "smart-contracts");
  assert.equal(fakeClient.initCalls.length, 0);
  assert.equal(fakeClient.updateCalls.length, 0);
  assert.equal(
    model.getState().status,
    "Account authority is required before Smart Contracts requests can be submitted.",
  );
  assert.equal(
    model.getState().smartContracts.status,
    "Request account authority to enable Smart Contracts requests.",
  );
});

test("smart-contract init flow uses injected web-sdk helpers and connect-client payload construction", async () => {
  const fakeClient = new FakeClient();
  const fakeTools = new FakeSmartContractTools();
  const model = createExampleAppModel({
    clientFactory: (() => fakeClient) satisfies ConnectClientFactory,
    challengeGenerator: () => "123456",
    initialNetworkGenesisHash: "genesis",
    smartContractTools: fakeTools,
  });

  await model.pair();
  await model.requestAccount();
  model.updateSmartContracts({
    mode: "init",
    contractName: "my_contract",
    initName: "init_my_contract",
    moduleRef: "module-ref",
    schemaBase64: "schema-base64",
    parameterJson: '{"owner":"4Jx"}',
    amountMicroCcd: "0",
    maxContractExecutionEnergy: "30000",
    validate: true,
  });

  model.prepareSmartContractRequest();
  await model.submitSmartContractRequest();

  assert.deepEqual(fakeTools.initInputs, [
    {
      schemaBase64: "schema-base64",
      contractName: "my_contract",
      parameterJson: '{"owner":"4Jx"}',
    },
  ]);
  assert.deepEqual(fakeClient.initCalls, [
    {
      sessionToken: "session-token",
      moduleRef: "module-ref",
      initName: "init_my_contract",
      amountMicroCcd: "0",
      maxContractExecutionEnergy: 30000,
      parameterHex: "deadbeef",
      schema: { base64: "schema-base64" },
      validate: true,
    },
  ]);
  assert.equal(model.getState().smartContracts.lastTransactionHash, "tx-init");
  assert.equal(
    model.getState().smartContracts.status,
    "Init request submitted through @ccd-wallet/connect-client.",
  );
});

test("reset clears paired session state, authority state, and showcase progress", async () => {
  const fakeClient = new FakeClient();
  const fakeTools = new FakeSmartContractTools();
  let challenges = ["123456", "654321"];
  const model = createExampleAppModel({
    clientFactory: (() => fakeClient) satisfies ConnectClientFactory,
    challengeGenerator: () => challenges.shift() ?? "000000",
    initialNetworkGenesisHash: "genesis",
    smartContractTools: fakeTools,
  });

  await model.pair();
  await model.requestAccount();
  model.updateSmartContracts({
    contractName: "my_contract",
    schemaBase64: "schema-base64",
    parameterJson: "{}",
  });
  model.prepareSmartContractRequest();
  model.reset();

  assert.equal(fakeClient.closeCalls, 1);
  assert.deepEqual(model.getState(), {
    serverUrl: "ws://127.0.0.1:22771",
    networkGenesisHash: "genesis",
    challenge: "654321",
    status: "Ready to pair.",
    currentPage: "smart-contracts",
    session: null,
    accountAuthority: null,
    smartContracts: {
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
    },
  });
});
