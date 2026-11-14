# Getting started

## 1. Install

Install [Rust](https://rustup.rs), then:

```sh
git clone https://github.com/thebothyt/gdl.rust
cd gdl.rust
cargo install --path cli
gdl --help
```

## 2. Try the examples

```sh
gdl run examples/blocks.gdl      # prints how many objects it makes, then the level string
gdl run examples/triggers.gdl
```

## 3. Get a level out of the game

You need a `.gdlvl` file. `gdl inspect MyLevel.gdlvl` tells you how it is wrapped; `gdl objects MyLevel.gdlvl` lists what
is in it. This tool reads the same `.gdlvl` layouts the rest of the repo documents; it does not read the game's own save
files directly.

## 4. Make a project

```sh
gdl convert MyLevel.gdlvl         # writes MyLevel.gdlrust
```

Open `MyLevel.gdlrust` in any editor and write your code under `%%% script`. See the [language guide](LANGUAGE.md).

## 5. Run, compile, import

```sh
gdl run MyLevel.gdlrust
gdl compile MyLevel.gdlrust       # writes MyLevel.compiled.gdlvl
```

Import `MyLevel.compiled.gdlvl` the same way you imported the original.

## Troubleshooting

| problem | try |
|---|---|
| `couldn't find the level string` | `gdl inspect` the file; it may not be a `.gdlvl` this tool understands |
| `already exists (use --force...)` | pick another name with `-o`, or `--force` to replace the file |
| `line N: ...` | the message names the line in your script |
| a trigger does nothing in-game | place one by hand, export, run `gdl objects`, and compare its keys with the script's output |
| `block(): unknown block` | run `gdl blocks`, or use `obj(id, x, y)` with any id |
| `.gdlrust ... has no level template` | compile with `--into Level.gdlvl` |
