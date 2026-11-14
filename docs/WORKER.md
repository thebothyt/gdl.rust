# The web editor and Cloudflare Worker

`worker/` wraps the Rust core in WebAssembly (`wasm-bindgen`) and serves an editor page from a Cloudflare Worker.

## Run it locally

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
cd worker
wasm-pack build --target web --out-dir pkg
npx wrangler dev          # open the printed URL; works from a phone on the same network
```

## Deploy

```sh
cd worker
wasm-pack build --target web --out-dir pkg
npx wrangler deploy
```

If you deploy through Cloudflare's Git integration instead, set the project's root directory to `worker` and use
`wasm-pack build --target web --out-dir pkg` as the build step. Check your project's Builds settings for how it treats
non-production branches (preview builds). The Worker needs the compiled `pkg/` folder, which is not committed.

## What the editor does

- Redraws a preview 350 ms after you stop typing (blocks, spikes, slabs, orbs and pads as shapes, triggers as labelled
  diamonds, dashed lines from a trigger to the objects in its group).
- Tap an object to see its properties and highlight the script line that made it.
- **Level file (.gdlvl)**: shows the level's existing objects underneath; *Patch & download* appends your script.
- **Project (.gdlrust)**: open a project, *Save .gdlrust* to download it, *Compile* to download a level.
  Choosing a `.gdlvl` and then saving makes a project from it.

## Privacy

No cookies, localStorage, sessionStorage, KV, D1 or R2. Uploaded files are processed in memory for the request and
returned. The page warns before you close it with unsaved changes, because nothing is saved for you.

## API

All endpoints are `POST`. Limits: 10 MB per file, 200 KB per script.

| path | body | returns |
|---|---|---|
| `/api/preview` | script text | JSON `{ok, objects, lines, logs}` or `{ok:false, error}` |
| `/api/check` | script text | plain-text report |
| `/api/base` | multipart `file` (.gdlvl) | JSON of the level's objects |
| `/api/inspect` | multipart `file` | layout dump |
| `/api/objects` | multipart `file` | one line per object |
| `/api/patch` | multipart `file` + `script` | patched `.gdlvl` download |
| `/api/open` | `.gdlrust` text | JSON `{ok, name, script, header, objects, has_template}` |
| `/api/save` | multipart `project`, `script`, `name`, optional `file` | `.gdlrust` text |
| `/api/compile` | multipart `project`, `script`, `name`, optional `file` | compiled `.gdlvl` download |
