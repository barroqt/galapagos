import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

export default defineConfig({
  // Solid's JSX compiles to direct DOM instructions, so the plugin has to see
  // every .tsx file. Nothing else in the pipeline transforms JSX.
  plugins: [solid()],
  server: {
    // Bind to all interfaces so the sandbox port can be published to the host.
    host: true,
    fs: {
      // The wasm-pack output lives outside the Vite root, in ../sim-core/pkg.
      allow: [".."],
    },
  },
});
