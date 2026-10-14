# Agent loop fixture

The project is `../projects/shop`. Each folder here is a prepared patch: copy
its files over the project to apply it.

- `harder/` makes `applyDiscounts` harder to follow (deep nesting).
- `fix/` replaces the nesting with a lookup object.

Both run locally: `scripts/agent-loop.sh` runs the loop by hand, and
`cargo test` runs it automatically through `tests/agent_loop.rs`.
