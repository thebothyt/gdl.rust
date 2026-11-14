# gdl.rust

**Write Geometry Dash levels as code.** `gdl` is a small scripting language and toolchain, written in Rust, that
generates Geometry Dash 2.2 objects and triggers and writes them into your level file. Loops, functions and
variables replace hours of copy-and-paste in the editor.

```
let door = group()
for i in 0..6 {
  add_group(block("brick", 300, 15 + i * 30), door)       // a wall of six bricks
}
pickup(90, 75, 1, 1)                                       // item 1 += 1
count(120, 75, 1, door, 3, false)                         // at 3, deactivate the wall
move(150, 75, door, 0, 180, 1.5, easing = EASE_IN_OUT)     // and slide it away
```

- **45 wrapped 2.2 triggers**: move, rotate, scale, pulse, color, gradient, camera, song/SFX, pickup, count, item edit,
  item compare, timers, collision, random and more. [Full reference](docs/TRIGGERS.md).
- **Named options** for the long tail of trigger settings: `move(0, 0, 1, 0, 60, 2, easing = EASE_OUT)`.
- **92 named blocks and objects** (`block("spike", x, y)`, `rect(B_BASIC, x, y, 4, 3)`). [Block list](docs/BLOCKS.md).
- **Any other trigger or property** via `trigger(id, x, y)` and `set(handle, key, value)`.
- **`.gdlrust` projects**: one plain-text file with your script, the level's objects and its wrapper.
  Convert a `.gdlvl` into a `.gdlrust`, edit, and compile it back. [Format](docs/GDLRUST_FORMAT.md).
- **Live preview editor** that runs in a browser (and on Cloudflare Workers) and redraws as you type.
- **Zero dependencies** in the core. Runs as a CLI and as WebAssembly.

> **Status:** early (0.x). Output is covered by automated tests and cross-checked against community property
> tables, but it has not been exhaustively tested inside the game. **Always keep a backup of your level.**
> Nothing here ever overwrites your original file.

## Install

You need [Rust](https://rustup.rs) (stable).

```sh
git clone https://github.com/thebothyt/gdl.rust
cd gdl.rust
cargo install --path cli        # installs the `gdl` command
gdl --help
```

Or run without installing: `cargo run -p gdl -- <command>`.

## Five-minute tour

**1. Start a project from a level you exported**

```sh
gdl convert MyLevel.gdlvl          # -> MyLevel.gdlrust  (your original is untouched)
```

**2. Open `MyLevel.gdlrust` and write code in its `%%% script` section**

```
%%% script
let g = group()                     // gets a group the level isn't already using
for i in 0..5 { add_group(block("basic", 300 + i * 30, 15), g) }
move(240, 45, g, 0, 90, 1, easing = EASE_IN_OUT)
```

**3. Check what it makes**

```sh
gdl run MyLevel.gdlrust            # prints the new objects and any print() output
```

**4. Compile back into a level**

```sh
gdl compile MyLevel.gdlrust        # -> MyLevel.compiled.gdlvl
```

Prefer to keep your level and script separate? `gdl patch MyLevel.gdlvl script.gdl` appends a script's objects
to a level, and `gdl new MyProject` makes an empty `.gdlrust` you can compile `--into` any level.

## Commands

| command | what it does |
|---|---|
| `gdl run <script.gdl \| project.gdlrust>` | run a script, print its objects |
| `gdl new <name>` | create `<name>.gdlrust` with a starter script |
| `gdl convert <level.gdlvl>` | `.gdlvl` to `.gdlrust` (and `.gdlrust` to `.gdlvl`) |
| `gdl compile <project.gdlrust> [--into level.gdlvl]` | run the script and write a level |
| `gdl patch <level.gdlvl> <script.gdl>` | append a script's objects to a level |
| `gdl inspect <level.gdlvl>` | show how a level file is wrapped and encoded |
| `gdl objects <level.gdlvl>` | list every object with its raw properties |
| `gdl blocks` | list the named block variants |
| `gdl preview` / `gdl base` | the JSON the web editor draws |
| `gdl docs triggers\|blocks` | print the generated references |

Output files are never written over an existing file unless you pass `--force`. Level and project files are
your work, so the tools won't replace them by surprise.

## Live preview editor

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
cd worker && wasm-pack build --target web --out-dir pkg && npx wrangler dev
```

Open the printed URL. The page redraws your script 350 ms after you stop typing: blocks, spikes, slabs, orbs and
pads as shapes, triggers as labelled diamonds with dashed lines to the groups they target. Tap an object to see
its properties and the script line that made it. Load a `.gdlvl` to see its existing objects underneath, or open and
save `.gdlrust` projects directly. Details and deployment: [docs/WORKER.md](docs/WORKER.md).

**Privacy:** the editor and the Worker use no cookies, no localStorage and no KV or database. Files you upload are
processed in memory and returned. Your project lives in the `.gdlrust` file you save.

## Documentation

| | |
|---|---|
| [Getting started](docs/GETTING_STARTED.md) | a longer walkthrough, with troubleshooting |
| [Language guide](docs/LANGUAGE.md) | syntax, builtins, constants, limits |
| [Trigger reference](docs/TRIGGERS.md) | every wrapped trigger, argument and constant (generated) |
| [Block reference](docs/BLOCKS.md) | named blocks and objects (generated) |
| [.gdlrust format](docs/GDLRUST_FORMAT.md) | the project file, byte for byte |
| [Cloudflare Worker](docs/WORKER.md) | running and deploying the web editor |
| [Examples](examples/) | runnable scripts: `blocks.gdl`, `triggers.gdl`, `demo.gdl` |

## Project layout

```
core/      language + codec library: lexer, parser, interpreter, trigger and block tables,
           .gdlvl codec, .gdlrust projects (no dependencies)
cli/       the `gdl` command
worker/    wasm-bindgen wrapper + Cloudflare Worker + editor page
docs/      guides; TRIGGERS.md and BLOCKS.md are generated and checked by tests
examples/  scripts and a small sample level
```

## Contributing

Issues and pull requests are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md). The most useful contributions are
new trigger entries (one table row each, see `core/src/triggers.rs`) and verifying keys against the 2.2 game.

## License and disclaimer

[MIT](LICENSE). This is an unofficial community tool, not affiliated with or endorsed by RobTop Games.
Geometry Dash is a trademark of its owner.
