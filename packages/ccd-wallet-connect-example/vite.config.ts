import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

/**
 * Vite configuration for the connect example application.
 *
 * The example intentionally uses a minimal React setup so the integration flow
 * stays readable while avoiding manual DOM wiring that distracts from the
 * connect-client usage.
 */
export default defineConfig({
  plugins: [react()],
});
