/**
 * The module catalogue: every lab the hub lists, in the order it lists them,
 * which is the recommended path - population dynamics, then space, then more
 * strategies, then the open sandbox.
 *
 * Separate from `registry.ts` so the contract and the content of the registry
 * change for different reasons. Adding a module means one entry here.
 */
import { HawkDoveModule } from "./hawk-dove/HawkDoveModule";
import { solidModule, type ModuleEntry, type ModuleId } from "./registry";

export const MODULES: readonly ModuleEntry[] = [
  {
    id: "hawk-dove",
    state: "ready",
    startHere: true,
    title: "Hawk and Dove",
    concept: "Mixed equilibrium",
    summary:
      "Two strategies contest one resource. Fighting is costly, so neither takes over: the population settles at a share theory can predict.",
    mount: solidModule(HawkDoveModule),
  },
  {
    id: "spatial",
    state: "planned",
    title: "Space",
    concept: "Network reciprocity",
    summary:
      "The same game played on a lattice, where agents only meet their neighbours. Clusters form, and the outcome leaves the well-mixed prediction behind.",
  },
  {
    id: "rock-paper-scissors",
    state: "planned",
    title: "Rock, Paper, Scissors",
    concept: "Cyclic dominance",
    summary:
      "Three strategies, each beating one and losing to another. Nothing is stable, and the population circles forever.",
  },
  {
    id: "stag-hunt",
    state: "planned",
    title: "Stag Hunt",
    concept: "Equilibrium selection",
    summary:
      "Two equilibria, one better for everyone and one safer for each. Which one evolution finds depends on where it starts.",
  },
  {
    id: "sandbox",
    state: "planned",
    title: "Sandbox",
    concept: "Your own game",
    summary:
      "An editable payoff matrix and every parameter in the lab, for running the experiment nobody wrote a module for.",
  },
];

/**
 * The hub signposts exactly one entry point. Two would leave a newcomer
 * choosing between recommendations, none would leave them choosing blind, and
 * either is a typo away in the list above.
 */
const startHereCount = MODULES.filter(
  (entry) => entry.state === "ready" && entry.startHere === true,
).length;
if (startHereCount !== 1) {
  throw new Error(
    `modules: exactly one module must be marked startHere, found ${startHereCount}`,
  );
}

/** Looks up a module by id. */
export function moduleById(id: ModuleId): ModuleEntry | undefined {
  return MODULES.find((entry) => entry.id === id);
}
