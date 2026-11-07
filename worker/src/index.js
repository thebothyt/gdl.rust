// Cloudflare Worker: serves a small editor page and runs the Rust core (compiled to WASM).
//
//   GET  /            the editor page
//   POST /api/check   body = script text            -> plain-text report
//   POST /api/patch   multipart: file + script      -> patched .gdlvl download
//   POST /api/inspect multipart: file               -> plain-text layout dump

import wasmModule from "../pkg/gdl_wasm_bg.wasm";
import { initSync, check, patch, inspect } from "../pkg/gdl_wasm.js";

initSync({ module: wasmModule });

const MAX_FILE = 10 * 1024 * 1024;

const DEMO = `let stairs = group()

for i in 0..10 {
  let b = obj(1, 15 + i * 30, 15 + i * 30)
  add_group(b, stairs)
}

move(400, 15, stairs, 0, 90, 1.5)
print("done")
`;

const PAGE = `<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>gdl editor</title>
<style>
  :root { color-scheme: light dark; }
  body { font: 16px/1.4 system-ui, sans-serif; margin: 0 auto; padding: 16px; max-width: 760px; }
  textarea { width: 100%; box-sizing: border-box; height: 280px; font: 14px/1.4 ui-monospace, monospace; padding: 8px; tab-size: 2; }
  button, input[type=file] { font: inherit; margin: 6px 6px 6px 0; }
  button { padding: 8px 14px; }
  pre { background: rgba(127,127,127,.15); padding: 10px; border-radius: 6px; white-space: pre-wrap; word-break: break-word; min-height: 3em; }
</style></head><body>
<h1>gdl editor</h1>
<textarea id="script" spellcheck="false" autocapitalize="off" autocorrect="off">__DEMO__</textarea>
<div><button id="check">Check script</button></div>
<div>
  <label>Level file (.gdlvl): <input type="file" id="file"></label><br>
  <button id="inspect">Inspect file</button>
  <button id="patch">Patch &amp; download</button>
</div>
<pre id="out">Ready.</pre>
<script>
  var out = document.getElementById("out");
  var scriptEl = document.getElementById("script");
  var fileEl = document.getElementById("file");
  function show(t) { out.textContent = t; }
  function form(withScript) {
    var f = fileEl.files[0];
    if (!f) { show("Pick a .gdlvl file first."); return null; }
    var fd = new FormData();
    fd.append("file", f);
    if (withScript) fd.append("script", scriptEl.value);
    return fd;
  }
  document.getElementById("check").onclick = function () {
    fetch("/api/check", { method: "POST", body: scriptEl.value })
      .then(function (r) { return r.text(); }).then(show);
  };
  document.getElementById("inspect").onclick = function () {
    var fd = form(false); if (!fd) return;
    fetch("/api/inspect", { method: "POST", body: fd })
      .then(function (r) { return r.text(); }).then(show);
  };
  document.getElementById("patch").onclick = function () {
    var fd = form(true); if (!fd) return;
    fetch("/api/patch", { method: "POST", body: fd }).then(function (r) {
      if (!r.ok) return r.text().then(function (t) { show("error: " + t); });
      var name = (fileEl.files[0].name || "level.gdlvl").replace(/\\.gdlvl$/, "") + ".patched.gdlvl";
      return r.blob().then(function (b) {
        var a = document.createElement("a");
        a.href = URL.createObjectURL(b); a.download = name; a.click();
        show("Downloaded " + name);
      });
    });
  };
</script></body></html>`.replace("__DEMO__", DEMO.replace(/&/g, "&amp;").replace(/</g, "&lt;"));

const text = (body, status = 200) =>
  new Response(body, { status, headers: { "content-type": "text/plain; charset=utf-8" } });

async function readFile(form) {
  const file = form.get("file");
  if (!(file instanceof File)) throw new Error("no file uploaded");
  if (file.size > MAX_FILE) throw new Error("file too large (10 MB max)");
  return new Uint8Array(await file.arrayBuffer());
}

export default {
  async fetch(request) {
    const { pathname } = new URL(request.url);
    try {
      if (request.method === "GET" && pathname === "/") {
        return new Response(PAGE, { headers: { "content-type": "text/html; charset=utf-8" } });
      }
      if (request.method === "POST" && pathname === "/api/check") {
        return text(check(await request.text()));
      }
      if (request.method === "POST" && pathname === "/api/inspect") {
        return text(inspect(await readFile(await request.formData())));
      }
      if (request.method === "POST" && pathname === "/api/patch") {
        const form = await request.formData();
        const script = form.get("script");
        if (typeof script !== "string") throw new Error("no script");
        const out = patch(await readFile(form), script);
        return new Response(out, {
          headers: {
            "content-type": "application/octet-stream",
            "content-disposition": 'attachment; filename="level.patched.gdlvl"',
          },
        });
      }
      return text("not found", 404);
    } catch (e) {
      return text(String(e && e.message ? e.message : e), 400);
    }
  },
};
