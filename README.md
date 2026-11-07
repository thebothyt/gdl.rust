# gdl.rust

A small scripting language, written in Rust, that generates Geometry Dash level
objects and writes them into a `.gdlvl` file. The core has zero dependencies, so the
same code runs as a CLI and inside a Cloudflare Worker (compiled to WASM).

```
core/      language + codec library (lexer, parser, interpreter, base64/gzip, .gdlvl patcher)
cli/       `gdl` command line tool
worker/    wasm-bindgen wrapper + Cloudflare Worker with a mobile-friendly editor page
examples/  demo.gdl
```

## Quick start

```sh
cargo test                                   # run the test suite
cargo run -p gdl -- run examples/demo.gdl    # print the generated objects
cargo run -p gdl -- inspect MyLevel.gdlvl    # decode a level file and show its layout
cargo run -p gdl -- patch MyLevel.gdlvl examples/demo.gdl   # -> MyLevel.patched.gdlvl
```

`patch` never touches your original; it writes `<name>.patched.gdlvl` (or `-o path`).
Keep a backup of your level anyway.

## The language

```
// comments
let x = 10            // variables
x += 5                // = += -= *= /=
let s = "a" + 1       // strings (+ joins), numbers, true / false / nil

if x > 3 && x < 100 { ... } else if x == 0 { ... } else { ... }
for i in 0..10 { ... }              // 0 up to 9
for i in 0..100 step 10 { ... }
for i in 10..0 step -1 { ... }
while x > 0 { x -= 1 }

fn spike_row(x, n) {                // functions, recursion, return
  for i in 0..n { obj(8, x + i * 30, 15) }
}
```

A new line ends an expression, so a line starting with `-` or `(` begins a new statement.
Functions can read and assign top-level variables. PI is predefined.

### Builtins

| call | does |
|---|---|
| `obj(id, x, y)` | create an object, returns its handle |
| `set(handle, key, value)` | set any raw GD property key |
| `group()` / `color()` | next unused group / color channel (start at 1) |
| `add_group(handle, g)` | put an object in group `g` (key 57, dot-joined) |
| `move(x, y, target, dx, dy, secs)` | move trigger (id 901) |
| `alpha(x, y, target, opacity, secs)` | alpha trigger (id 1007) |
| `toggle(x, y, target, on)` | toggle trigger (id 1049) |
| `print(...)` | log text (shown by `run` and the editor page) |
| `floor ceil round abs sqrt sin cos` | 1-argument math |
| `min max pow` | 2-argument math |

Safety limits (for hosting): 3,000,000 statements, 150,000 objects, 100 call depth.

## Cloudflare Worker

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
cd worker
wasm-pack build --target web --out-dir pkg
npx wrangler dev        # then open the printed URL, also works from your phone on LAN
npx wrangler deploy
```

The Worker can't see files on your device: you upload the `.gdlvl` and the script,
and it returns the patched file to download.

## Ideas for next steps (for me lmao)

Lists/arrays, `rand()`, more triggers (pulse, spawn, rotate, follow), grouping helpers like
`obj_in(group, ...)`, colors (`color` channel triggers), an `import` for reusable files, and a
`gdl fmt`.

