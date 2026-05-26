/**
 * React UI for the connect example app.
 *
 * This component renders an unpaired pairing screen and a paired showcase shell
 * with navigation between capability areas.
 */
import { useEffect, useMemo, useState } from "react";

import { createExampleAppModel } from "./app.js";

/**
 * Root React component for the connect example application.
 *
 * @returns The rendered example application.
 * @example
 * ```tsx
 * <ExampleApp />
 * ```
 */
export function ExampleApp() {
  const model = useMemo(() => createExampleAppModel(), []);
  const [state, setState] = useState(model.getState());

  useEffect(() => model.subscribe(setState), [model]);

  return (
    <main className="example-app">
      <header className="hero panel">
        <p className="eyebrow">ccd-wallet connect example</p>
        <h1>Paired-session API showcase</h1>
        <p className="lead">
          Pair first to establish trusted browser-session context for one
          network. Request account authority later only when a capability needs
          it.
        </p>
      </header>

      {!state.session ? (
        <section className="panel pairing-screen">
          <h2>Pair with Wallet</h2>
          <p className="muted">
            This screen establishes a paired session only. The node endpoint is
            also captured here so the Smart Contracts page can derive embedded
            schema automatically.
          </p>

          <label>
            <span>Connect server URL</span>
            <input
              type="text"
              value={state.serverUrl}
              onChange={(event) => model.setServerUrl(event.target.value)}
            />
          </label>

          <label>
            <span>Target network genesis hash</span>
            <input
              type="text"
              value={state.networkGenesisHash}
              onChange={(event) =>
                model.setNetworkGenesisHash(event.target.value)
              }
            />
          </label>

          <label>
            <span>Browser-reachable node endpoint (gRPC-web)</span>
            <input
              type="text"
              value={state.nodeEndpoint}
              onChange={(event) => model.setNodeEndpoint(event.target.value)}
            />
          </label>

          <label>
            <span>Pairing challenge (enter this in the wallet prompt)</span>
            <div className="challenge-row">
              <input
                type="text"
                maxLength={6}
                inputMode="numeric"
                value={state.challenge}
                onChange={(event) => model.setChallenge(event.target.value)}
              />
              <button
                type="button"
                className="secondary-button"
                onClick={() => model.regenerateChallenge()}
              >
                Regenerate
              </button>
            </div>
          </label>

          <div className="button-row">
            <button type="button" onClick={() => void model.pair()}>
              Pair with Wallet
            </button>
            <button
              type="button"
              className="secondary-button"
              onClick={() => model.reset()}
            >
              Reset
            </button>
          </div>
        </section>
      ) : (
        <div className="paired-shell">
          <section className="panel session-context">
            <div className="shell-header-row">
              <div>
                <h2>Paired Session</h2>
                <p className="muted">
                  Network trust is already bound. Account authority is optional
                  until a feature asks for it.
                </p>
              </div>
              <div className="button-row compact">
                <button
                  type="button"
                  className="secondary-button"
                  onClick={() => void model.requestAccount()}
                >
                  {state.accountAuthority
                    ? "Refresh Account Authority"
                    : "Request Account Authority"}
                </button>
                <button
                  type="button"
                  className="secondary-button"
                  onClick={() => model.reset()}
                >
                  Reset
                </button>
              </div>
            </div>

            <div className="context-grid">
              <InfoCard
                title="Session token"
                value={state.session.sessionToken}
                testId="session-token"
              />
              <InfoCard
                title="Bound network genesis hash"
                value={state.session.networkGenesisHash}
                testId="network-genesis-hash"
              />
              <InfoCard
                title="Node endpoint"
                value={state.session.nodeEndpoint}
                testId="node-endpoint"
              />
              <InfoCard
                title="Account authority"
                value={state.accountAuthority?.accountAddress ?? "Not granted yet"}
                testId="account-authority"
              />
            </div>
          </section>

          <section className="panel shell-nav-panel">
            <nav className="shell-nav" aria-label="API showcase sections">
              <NavButton
                label="Smart Contracts"
                active={state.currentPage === "smart-contracts"}
                onClick={() => model.setCurrentPage("smart-contracts")}
              />
              <NavButton
                label="Transactions"
                active={state.currentPage === "transactions"}
                onClick={() => model.setCurrentPage("transactions")}
              />
              <NavButton
                label="Chain Updates"
                active={state.currentPage === "chain-updates"}
                onClick={() => model.setCurrentPage("chain-updates")}
              />
            </nav>
          </section>

          {state.currentPage === "smart-contracts" ? (
            <section className="panel capability-panel">
              <div className="section-header">
                <div>
                  <h2>Smart Contracts</h2>
                  <p className="muted">
                    This page derives embedded schema from the referenced module
                    or target contract instance with
                    <code>@concordium/web-sdk</code> and then submits the request
                    through <code>@ccd-wallet/connect-client</code>.
                  </p>
                </div>
              </div>

              {!state.accountAuthority ? (
                <div className="authority-gate">
                  <h3>Account authority required</h3>
                  <p>
                    Smart contract init and update requests require previously
                    granted session account authority.
                  </p>
                  <button type="button" onClick={() => void model.requestAccount()}>
                    Request Account Authority
                  </button>
                </div>
              ) : (
                <>
                  <div
                    className="mode-toggle"
                    role="tablist"
                    aria-label="Smart contract request type"
                  >
                    <button
                      type="button"
                      className={
                        state.smartContracts.mode === "init"
                          ? "nav-button active"
                          : "nav-button"
                      }
                      onClick={() => model.updateSmartContracts({ mode: "init" })}
                    >
                      Contract Init
                    </button>
                    <button
                      type="button"
                      className={
                        state.smartContracts.mode === "update"
                          ? "nav-button active"
                          : "nav-button"
                      }
                      onClick={() =>
                        model.updateSmartContracts({ mode: "update" })
                      }
                    >
                      Contract Update
                    </button>
                  </div>

                  <div className="form-grid two-columns">
                    {state.smartContracts.mode === "init" ? (
                      <>
                        <label>
                          <span>Module reference</span>
                          <input
                            type="text"
                            value={state.smartContracts.moduleRef}
                            onChange={(event) =>
                              model.updateSmartContracts({
                                moduleRef: event.target.value,
                              })
                            }
                          />
                        </label>
                        <label>
                          <span>Init name</span>
                          <input
                            type="text"
                            value={state.smartContracts.initName}
                            onChange={(event) =>
                              model.updateSmartContracts({
                                initName: event.target.value,
                              })
                            }
                          />
                        </label>
                      </>
                    ) : (
                      <>
                        <label>
                          <span>Contract index</span>
                          <input
                            type="text"
                            inputMode="numeric"
                            value={state.smartContracts.contractIndex}
                            onChange={(event) =>
                              model.updateSmartContracts({
                                contractIndex: event.target.value,
                              })
                            }
                          />
                        </label>
                        <label>
                          <span>Contract subindex</span>
                          <input
                            type="text"
                            inputMode="numeric"
                            value={state.smartContracts.contractSubindex}
                            onChange={(event) =>
                              model.updateSmartContracts({
                                contractSubindex: event.target.value,
                              })
                            }
                          />
                        </label>
                        <label>
                          <span>Entrypoint name</span>
                          <input
                            type="text"
                            value={state.smartContracts.entrypointName}
                            onChange={(event) =>
                              model.updateSmartContracts({
                                entrypointName: event.target.value,
                              })
                            }
                          />
                        </label>
                      </>
                    )}

                    <label>
                      <span>Amount (microCCD)</span>
                      <input
                        type="text"
                        inputMode="numeric"
                        value={state.smartContracts.amountMicroCcd}
                        onChange={(event) =>
                          model.updateSmartContracts({
                            amountMicroCcd: event.target.value,
                          })
                        }
                      />
                    </label>

                    <label>
                      <span>Max contract execution energy</span>
                      <input
                        type="text"
                        inputMode="numeric"
                        value={state.smartContracts.maxContractExecutionEnergy}
                        onChange={(event) =>
                          model.updateSmartContracts({
                            maxContractExecutionEnergy: event.target.value,
                          })
                        }
                      />
                    </label>
                  </div>

                  <label>
                    <span>Parameter JSON</span>
                    <textarea
                      rows={8}
                      value={state.smartContracts.parameterJson}
                      onChange={(event) =>
                        model.updateSmartContracts({
                          parameterJson: event.target.value,
                        })
                      }
                    />
                  </label>

                  <label className="checkbox-row">
                    <input
                      type="checkbox"
                      checked={state.smartContracts.validate}
                      onChange={(event) =>
                        model.updateSmartContracts({
                          validate: event.target.checked,
                        })
                      }
                    />
                    <span>Request wallet-side simulation before prompting</span>
                  </label>

                  <div className="button-row">
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => void model.prepareSmartContractRequest()}
                    >
                      Derive Embedded Schema
                    </button>
                    <button
                      type="button"
                      onClick={() => void model.submitSmartContractRequest()}
                    >
                      Submit {state.smartContracts.mode === "init" ? "Init" : "Update"} Request
                    </button>
                  </div>

                  <div className="output-grid">
                    <InfoCard
                      title="Prepared parameter hex"
                      value={state.smartContracts.preparedParameterHex || "Not prepared yet"}
                      testId="prepared-parameter-hex"
                    />
                    <InfoCard
                      title="Prepared module reference"
                      value={state.smartContracts.preparedModuleRef || "Not resolved yet"}
                      testId="prepared-module-ref"
                    />
                    <InfoCard
                      title="Prepared contract name"
                      value={state.smartContracts.preparedContractName || "Not resolved yet"}
                      testId="prepared-contract-name"
                    />
                    <InfoCard
                      title="Last transaction hash"
                      value={state.smartContracts.lastTransactionHash || "No request submitted yet"}
                      testId="smart-contract-tx-hash"
                    />
                  </div>

                  <section className="nested-panel">
                    <h3>Smart Contracts status</h3>
                    <p data-field="smart-contract-status">
                      {state.smartContracts.status}
                    </p>
                  </section>
                </>
              )}
            </section>
          ) : state.currentPage === "transactions" ? (
            <PlaceholderPage
              title="Transactions"
              body="This placeholder establishes the future showcase section. Transaction APIs will follow the same paired-session shell and capability-specific authority model."
            />
          ) : (
            <PlaceholderPage
              title="Chain Updates"
              body="This placeholder reserves navigation space for governance and chain-update workflows without implementing those API areas yet."
            />
          )}
        </div>
      )}

      <section className="panel output">
        <h2>Application status</h2>
        <p data-field="status">{state.status}</p>
      </section>
    </main>
  );
}

interface InfoCardProps {
  title: string;
  value: string;
  testId: string;
}

function InfoCard({ title, value, testId }: InfoCardProps) {
  return (
    <div className="info-card">
      <h3>{title}</h3>
      <pre data-testid={testId}>{value}</pre>
    </div>
  );
}

interface NavButtonProps {
  label: string;
  active: boolean;
  onClick: () => void;
}

function NavButton({ label, active, onClick }: NavButtonProps) {
  return (
    <button
      type="button"
      className={active ? "nav-button active" : "nav-button"}
      onClick={onClick}
    >
      {label}
    </button>
  );
}

interface PlaceholderPageProps {
  title: string;
  body: string;
}

function PlaceholderPage({ title, body }: PlaceholderPageProps) {
  return (
    <section className="panel capability-panel placeholder-page">
      <h2>{title}</h2>
      <p>{body}</p>
    </section>
  );
}
