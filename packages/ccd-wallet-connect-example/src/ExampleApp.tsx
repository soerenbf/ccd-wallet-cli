/**
 * React UI for the connect example app.
 *
 * This component keeps the rendered integration flow small and explicit while
 * delegating business logic to the example app model.
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
      <h1>ccd-wallet connect example</h1>
      <p className="lead">
        A minimal integration reference for pairing and session context retrieval.
      </p>
      <p className="lead secondary">
        When pairing, copy or type the challenge shown here into the wallet prompt.
      </p>

      <section className="panel">
        <label>
          <span>Connect server URL</span>
          <input
            type="text"
            value={state.serverUrl}
            onChange={(event) => model.setServerUrl(event.target.value)}
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
            <button type="button" onClick={() => model.regenerateChallenge()}>
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
            disabled={!state.sessionToken}
            onClick={() => void model.refresh()}
          >
            Refresh Session Context
          </button>
          <button type="button" onClick={() => model.reset()}>
            Reset
          </button>
        </div>
      </section>

      <section className="panel output">
        <h2>Status</h2>
        <p>{state.status}</p>

        <h2>Session token</h2>
        <pre>{state.sessionToken || "—"}</pre>

        <h2>Network genesis hash</h2>
        <pre>{state.context?.networkGenesisHash ?? "—"}</pre>

        <h2>Account address</h2>
        <pre>{state.context?.accountAddress ?? "—"}</pre>
      </section>
    </main>
  );
}
