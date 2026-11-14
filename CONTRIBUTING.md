# Contributing

Thanks for helping. The project has no dependencies, so setup is just `cargo test`.

## Adding or fixing a trigger

Triggers are one row each in `core/src/triggers.rs`: script name, object id, arguments with their property keys.
After editing:

```sh
cargo run -p gdl -- docs triggers > docs/TRIGGERS.md
cargo test
```

A test fails if `docs/TRIGGERS.md` is stale, and another calls every trigger with its required arguments. Please say in
your pull request where you checked the keys (the game itself, `gdl objects` on an exported level, or a community
table). Blocks work the same way in `core/src/blocks.rs` (`gdl docs blocks > docs/BLOCKS.md`).

## Checklist

- `cargo test` passes.
- New behaviour has a test; new syntax or builtins are described in `docs/LANGUAGE.md`.
- Example scripts in `examples/` still run (a test covers this).
- The core stays dependency-free.
- Nothing writes to browser storage, and files are never overwritten without `--force`.

## Reporting bugs

Include the script, the command you ran and the error. If a level misbehaves in-game, include the output of
`gdl objects` for the objects involved.
