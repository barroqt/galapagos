/**
 * The WASM boundary.
 *
 * The files behind this one are the only place in the app that imports
 * `sim-core/pkg`; everything above it imports from here. That is what makes
 * the questions with one answer - what crosses, what is a copy, what has to be
 * freed - answerable by reading a single directory.
 *
 * What crosses, in short:
 *
 * - **Parameters** go in as plain numbers ({@link HawkDoveParams}), converted
 *   to what wasm-bindgen wants at the boundary and validated by `sim-core`,
 *   which names whatever it rejects.
 * - **Shares** come back as flat generation-major `Float64Array`s
 *   ({@link ShareHistory}), read directly and never copied per frame.
 * - **Runs** are owned objects with a `dispose()`. Nothing in WASM memory is
 *   collected for you.
 */
export { CoreError, DisposedError } from "./errors";
export { ShareHistory } from "./history";
export { assertCoreLoaded, coreVersion, initCore } from "./init";
export { Owned } from "./owned";
export type { HawkDoveParams, TrajectoryParams } from "./params";
export { ReplicatorTrajectory } from "./replicator";
export { WellMixedRun } from "./wellmixed";
