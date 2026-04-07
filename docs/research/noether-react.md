# NoetherReact

*Content-addressed reactive UI — stages as components.*

The full design document lives in the repository:
[`noether-research/noether-react/DESIGN.md`](https://github.com/alpibrusl/noether/blob/main/noether-research/noether-react/DESIGN.md)

## Core thesis

`UI = f(stage_graph(state))`

React components are functions from state to UI. NoetherReact makes those functions content-addressed, type-checked Noether stages — composable, cached, and AI-discoverable.

## Key ideas

- **Component = Stage** — every component has a `StageId`; same ID = same render, guaranteed by content addressing.
- **Auto-memoization** — referential equality of stage IDs replaces `useMemo`/`React.memo` entirely.
- **Type-checked trees** — the composition engine validates that parent and child component types align at build time.
- **AI-composable UI** — `noether compose "a settings page with dark mode toggle"` can assemble UI graphs the same way it assembles data pipelines.

See the [Cloud Registry research](cloud-registry.md) for how a public stage registry enables sharing UI components as content-addressed artifacts.
