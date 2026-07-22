# Frontend decisions

One entry per `[DECISION]` task in [../ISSUES.md](../ISSUES.md). Each records
what was chosen, what it was judged against, and what would have to change for
the decision to be revisited. Decisions are appended, never rewritten.

---

## 2b.1 - UI framework and charting library

**Decided:** Solid.js for the UI layer, a hand-rolled canvas 2D chart layer for
every chart.

The judgement case was the one from Issue 2b, not a feature checklist: a growing
line read by stride from a `Float64Array` at 60fps, with no allocation in the
frame callback, sitting next to the WebGL grid canvas that Issue 3b introduces -
and carrying everything up to Issue 7 (batch runner tables, editable payoff
matrix, presets).

### UI framework: Solid.js

Fine-grained signals with no virtual DOM. A live readout that changes 60 times a
second writes one text node; there is no tree to diff and no per-frame garbage,
which is the same constraint 2b.5 and 2b.6 impose on the driver and the chart.
Canvases stay uncontrolled - a `ref` handed to imperative render code - so the
3b WebGL renderer drops in without fighting the framework for ownership of the
element. Solid is plain TypeScript and JSX, so `tsc` remains the only
type-checker, and the runtime is small enough (~7kb) to be one dependency rather
than a platform.

Rejected:

- **Vanilla TS** - zero dependencies and it matches today's scaffold, but the
  cost lands later rather than never: by Issues 5b and 7 the hand-rolled state
  plumbing for tables, matrix editing and presets is the largest source of
  incidental bugs in the app.
- **React** - the broadest ecosystem, but every live readout wants an escape
  hatch from rendering to stay cheap at 60fps, which means the fast paths all
  live outside the framework's model rather than inside it.
- **Svelte 5** - comparable reactivity, heavier toolchain: `.svelte` files,
  `svelte-check` next to `tsc`, and a language layer that is not TypeScript.

### Charts: hand-rolled canvas 2D

The chart layer is ours. It reads the flat generation-major history by stride
with no copy, pins the y axis to `[0, 1]`, and draws the glow and the palette
from `styles/tokens.css` by construction rather than by overriding a library's
defaults. It is also the layer the later charts share: the ternary simplex plot
(4b) and the trait-distribution heatmap (6b) are custom work in any library, so
a library would be carried for exactly one of the three charts.

Rejected:

- **uPlot** - fast and small, but it wants column-major `AlignedData`
  (`x[]`, `y1[]`, `y2[]`). Feeding it the interleaved history means
  de-interleaving into fresh arrays every frame, which is precisely the
  allocation 2b.6 forbids, and it offers nothing for the simplex or the heatmap.
- **D3 / Observable Plot** - excellent scales and axes, but SVG re-binding per
  frame is the wrong shape for a growing 60fps line. D3 stays available as a
  scale/axis helper if the simplex geometry in 4b turns out to want it; that
  would be a local dependency, not this decision reversed.

### Revisit if

The line chart stops being the demanding case - a module needs many series at
once, or interactive zoom/brush over a long history. Those are where a real
charting library earns its keep, and none of Issues 2b to 7 asks for them.
