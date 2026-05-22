/**
 * Browser entrypoint for the connect example app.
 *
 * This module mounts the React-based integration reference into the root
 * document node and intentionally keeps bootstrapping logic minimal.
 */
import React from "react";
import ReactDOM from "react-dom/client";

import "./style.css";

import { ConnectClientError } from "@ccd-wallet/connect-client";

import { ExampleApp } from "./ExampleApp.js";

const root = document.querySelector("#app");
if (!(root instanceof HTMLElement)) {
  throw new ConnectClientError("Missing #app root element for connect example app");
}

ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <ExampleApp />
  </React.StrictMode>,
);
