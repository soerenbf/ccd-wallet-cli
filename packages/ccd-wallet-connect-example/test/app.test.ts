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
  initInputs: Array<{
    nodeEndpoint: string;
    moduleRef: string;
    initName: string;
    parameterJson: string;
  }> = [];
  updateInputs: Array<{
    nodeEndpoint: string;
    contractIndex: string;
    contractSubindex: string;
    entrypointName: string;
    parameterJson: string;
  }> = [];
  nextError: unknown = null;

  async prepareInit(input: {
    nodeEndpoint: string;
    moduleRef: string;
    initName: string;
    parameterJson: string;
  }): Promise<PreparedSmartContractParameters> {
    if (this.nextError !== null) {
      throw this.nextError;
    }
    this.initInputs.push(input);
    return {
      parameterHex: "deadbeef",
      schema: { base64: "embedded-schema-base64" },
      parameterJson: JSON.parse(input.parameterJson),
      contractName: "my_contract",
      moduleRef: input.moduleRef,
    };
  }

  async prepareUpdate(input: {
    nodeEndpoint: string;
    contractIndex: string;
    contractSubindex: string;
    entrypointName: string;
    parameterJson: string;
  }): Promise<PreparedSmartContractParameters> {
    if (this.nextError !== null) {
      throw this.nextError;
    }
    this.updateInputs.push(input);
    return {
      parameterHex: "c0ffee",
      schema: { base64: "embedded-update-schema" },
      parameterJson: JSON.parse(input.parameterJson),
      contractName: "weather",
      moduleRef:
        "44434352ddba724930d6b1b09cd58bd1fba6ad9714cf519566d5fe72d80da0d1",
    };
  }
}

test("pairing establishes a paired session shell with node lookup context before account authority exists", async () => {
  const fakeClient = new FakeClient();
  const model = createExampleAppModel({
    clientFactory: (() => fakeClient) satisfies ConnectClientFactory,
    challengeGenerator: () => "123456",
    initialNetworkGenesisHash: "genesis",
    initialNodeEndpoint: "http://127.0.0.1:20000",
  });

  await model.pair();

  assert.deepEqual(fakeClient.pairingCalls, ["123456"]);
  assert.deepEqual(model.getState(), {
    serverUrl: "ws://127.0.0.1:22771",
    networkGenesisHash: "genesis",
    nodeEndpoint: "http://127.0.0.1:20000",
    challenge: "123456",
    status:
      "Pairing approved. Session established for the selected network and node context.",
    currentPage: "smart-contracts",
    session: {
      sessionToken: "session-token",
      networkGenesisHash: "genesis",
      nodeEndpoint: "http://127.0.0.1:20000",
    },
    accountAuthority: null,
    smartContracts: {
      mode: "init",
      moduleRef: "",
      initName: "init_my_contract",
      entrypointName: "set",
      contractIndex: "0",
      contractSubindex: "0",
      amountMicroCcd: "0",
      maxContractExecutionEnergy: "30000",
      parameterJson: "{}",
      validate: true,
      status:
        "Provide Smart Contracts request details and JSON input. The app will derive embedded schema automatically from the module or target contract instance.",
      preparedParameterHex: "",
      preparedSchema: null,
      preparedModuleRef: "",
      preparedContractName: "",
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
    initialNodeEndpoint: "http://127.0.0.1:20000",
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
    initialNodeEndpoint: "http://127.0.0.1:20000",
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

test("smart-contract init flow derives embedded schema from the referenced module", async () => {
  const fakeClient = new FakeClient();
  const fakeTools = new FakeSmartContractTools();
  const model = createExampleAppModel({
    clientFactory: (() => fakeClient) satisfies ConnectClientFactory,
    challengeGenerator: () => "123456",
    initialNetworkGenesisHash: "genesis",
    initialNodeEndpoint: "http://127.0.0.1:20000",
    smartContractTools: fakeTools,
  });

  await model.pair();
  await model.requestAccount();
  model.updateSmartContracts({
    mode: "init",
    initName: "init_my_contract",
    moduleRef: "module-ref",
    parameterJson: '{"owner":"4Jx"}',
    amountMicroCcd: "0",
    maxContractExecutionEnergy: "30000",
    validate: true,
  });

  await model.prepareSmartContractRequest();
  await model.submitSmartContractRequest();

  assert.deepEqual(fakeTools.initInputs, [
    {
      nodeEndpoint: "http://127.0.0.1:20000",
      moduleRef: "module-ref",
      initName: "init_my_contract",
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
      schema: { base64: "embedded-schema-base64" },
      validate: true,
    },
  ]);
  assert.equal(
    model.getState().smartContracts.preparedModuleRef,
    "module-ref",
  );
  assert.equal(
    model.getState().smartContracts.preparedContractName,
    "my_contract",
  );
  assert.equal(model.getState().smartContracts.lastTransactionHash, "tx-init");
});

test("smart-contract update flow derives embedded schema from the target instance module", async () => {
  const fakeClient = new FakeClient();
  const fakeTools = new FakeSmartContractTools();
  const model = createExampleAppModel({
    clientFactory: (() => fakeClient) satisfies ConnectClientFactory,
    challengeGenerator: () => "123456",
    initialNetworkGenesisHash: "genesis",
    initialNodeEndpoint: "http://127.0.0.1:20000",
    smartContractTools: fakeTools,
  });

  await model.pair();
  await model.requestAccount();
  model.updateSmartContracts({
    mode: "update",
    contractIndex: "42",
    contractSubindex: "0",
    entrypointName: "set",
    parameterJson: '{"value":17}',
    amountMicroCcd: "1",
    maxContractExecutionEnergy: "30000",
    validate: false,
  });

  await model.prepareSmartContractRequest();
  await model.submitSmartContractRequest();

  assert.deepEqual(fakeTools.updateInputs, [
    {
      nodeEndpoint: "http://127.0.0.1:20000",
      contractIndex: "42",
      contractSubindex: "0",
      entrypointName: "set",
      parameterJson: '{"value":17}',
    },
  ]);
  assert.deepEqual(fakeClient.updateCalls, [
    {
      sessionToken: "session-token",
      contractAddress: { index: 42, subindex: 0 },
      receiveName: "weather.set",
      amountMicroCcd: "1",
      maxContractExecutionEnergy: 30000,
      parameterHex: "c0ffee",
      schema: { base64: "embedded-update-schema" },
      validate: false,
    },
  ]);
  assert.equal(
    model.getState().smartContracts.preparedContractName,
    "weather",
  );
});

test("string-like schema preparation errors are surfaced in app status", async () => {
  const fakeClient = new FakeClient();
  const fakeTools = new FakeSmartContractTools();
  fakeTools.nextError = { message: "Schema mismatch for weather.vote parameter" };
  const model = createExampleAppModel({
    clientFactory: (() => fakeClient) satisfies ConnectClientFactory,
    challengeGenerator: () => "123456",
    initialNetworkGenesisHash: "genesis",
    initialNodeEndpoint: "http://127.0.0.1:20000",
    smartContractTools: fakeTools,
  });

  await model.pair();
  await model.requestAccount();
  model.updateSmartContracts({
    mode: "update",
    contractIndex: "1999",
    contractSubindex: "0",
    entrypointName: "vote",
    parameterJson: '{"vote_length":8}',
  });

  await model.prepareSmartContractRequest();

  assert.equal(
    model.getState().status,
    "Error: Schema mismatch for weather.vote parameter",
  );
  assert.equal(
    model.getState().smartContracts.status,
    "Error: Schema mismatch for weather.vote parameter",
  );
});

test("reset clears paired session state, authority state, and embedded-schema preparation state", async () => {
  const fakeClient = new FakeClient();
  const fakeTools = new FakeSmartContractTools();
  let challenges = ["123456", "654321"];
  const model = createExampleAppModel({
    clientFactory: (() => fakeClient) satisfies ConnectClientFactory,
    challengeGenerator: () => challenges.shift() ?? "000000",
    initialNetworkGenesisHash: "genesis",
    initialNodeEndpoint: "http://127.0.0.1:20000",
    smartContractTools: fakeTools,
  });

  await model.pair();
  await model.requestAccount();
  model.updateSmartContracts({
    moduleRef: "module-ref",
    parameterJson: "{}",
  });
  await model.prepareSmartContractRequest();
  model.reset();

  assert.equal(fakeClient.closeCalls, 1);
  assert.deepEqual(model.getState(), {
    serverUrl: "ws://127.0.0.1:22771",
    networkGenesisHash: "genesis",
    nodeEndpoint: "http://127.0.0.1:20000",
    challenge: "654321",
    status: "Ready to pair.",
    currentPage: "smart-contracts",
    session: null,
    accountAuthority: null,
    smartContracts: {
      mode: "init",
      moduleRef: "",
      initName: "init_my_contract",
      entrypointName: "set",
      contractIndex: "0",
      contractSubindex: "0",
      amountMicroCcd: "0",
      maxContractExecutionEnergy: "30000",
      parameterJson: "{}",
      validate: true,
      status:
        "Provide Smart Contracts request details and JSON input. The app will derive embedded schema automatically from the module or target contract instance.",
      preparedParameterHex: "",
      preparedSchema: null,
      preparedModuleRef: "",
      preparedContractName: "",
      lastTransactionHash: "",
    },
  });
});
