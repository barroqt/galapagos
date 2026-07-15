import { defineConfig } from "vite";

export default defineConfig({
  server: {
    // Bind to all interfaces so the sandbox port can be published to the host.
    host: true,
    fs: {
      // The wasm-pack output lives outside the Vite root, in ../sim-core/pkg.
      allow: [".."],
    },
  },
});
