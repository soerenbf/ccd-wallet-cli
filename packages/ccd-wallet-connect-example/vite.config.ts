import path from "node:path";
import { fileURLToPath } from "node:url";

import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const dirname = path.dirname(fileURLToPath(import.meta.url));

/**
 * Vite configuration for the connect example application.
 *
 * The example intentionally uses a minimal React setup so the integration flow
 * stays readable while avoiding manual DOM wiring that distracts from the
 * connect-client usage.
 */
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@ccd-wallet/connect-client": path.resolve(
        dirname,
        "../ccd-wallet-connect-client/src/index.ts",
      ),
    },
  },
});
