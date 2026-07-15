// Scaffold entry point: load the WASM core, step the placeholder Sim once per
// animation frame, and display the tick count. Proves the full pipeline
// (Rust -> wasm-bindgen -> Vite -> browser) before any real feature lands.
import init, { Sim, core_version } from "../../sim-core/pkg/sim_core";

async function main(): Promise<void> {
  await init();

  const status = document.getElementById("status")!;
  const footer = document.getElementById("core-version")!;
  footer.textContent = core_version();

  const sim = new Sim();
  function frame(): void {
    const tick = sim.step();
    status.textContent = `WASM pipeline OK — tick ${tick}`;
    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
}

main().catch((err) => {
  document.getElementById("status")!.textContent = `Failed to load WASM: ${err}`;
});
