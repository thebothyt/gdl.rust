// Cloudflare Worker: serves the visual editor and runs the Rust core (compiled to WASM).
//
//   GET  /             the editor page
//   POST /api/preview  body = script text        -> JSON {ok, objects, lines, logs} or {ok:false, error}
//   POST /api/base     multipart: file           -> JSON of the objects already in the level
//   POST /api/check    body = script text        -> plain-text report
//   POST /api/inspect  multipart: file           -> plain-text layout dump
//   POST /api/objects  multipart: file           -> one line per object (id + raw properties)
//   POST /api/patch    multipart: file + script  -> patched .gdlvl download
//   POST /api/open     body = .gdlrust text      -> JSON {ok, name, script, header, objects}
//   POST /api/save     multipart: project, script, name, [file] -> .gdlrust text
//   POST /api/compile  multipart: project, script, name, [file] -> compiled .gdlvl download
//
// Nothing is stored server-side or in the browser: no cookies, no KV, no storage. Projects live
// in the .gdlrust file the user saves.

import wasmModule from "../pkg/gdl_wasm_bg.wasm";
import {
  initSync, check, patch, inspect, preview, base, objects, open_project, save_project, compile_project,
} from "../pkg/gdl_wasm.js";
import { PAGE, DEMO } from "./page.js";

initSync({ module: wasmModule });

const MAX_FILE = 10 * 1024 * 1024;
const MAX_SCRIPT = 200 * 1024;

const html = PAGE.replace("__DEMO__", DEMO.replace(/&/g, "&amp;").replace(/</g, "&lt;"));

const text = (body, status = 200) =>
  new Response(body, { status, headers: { "content-type": "text/plain; charset=utf-8" } });
const json = (body) =>
  new Response(body, { headers: { "content-type": "application/json; charset=utf-8" } });

async function readFile(form) {
  const file = form.get("file");
  if (!(file instanceof File)) throw new Error("no file uploaded");
  if (file.size > MAX_FILE) throw new Error("file too large (10 MB max)");
  return new Uint8Array(await file.arrayBuffer());
}

// project form fields: project text (may be empty), script, name, optional level file
async function readProjectForm(request) {
  const form = await request.formData();
  const field = (k) => {
    const v = form.get(k);
    return typeof v === "string" ? v : "";
  };
  const script = field("script");
  if (script.length > MAX_SCRIPT) throw new Error("script too large (200 KB max)");
  const project = field("project");
  if (project.length > MAX_FILE) throw new Error("project too large (10 MB max)");
  const f = form.get("file");
  let level;
  if (f instanceof File && f.size > 0) {
    if (f.size > MAX_FILE) throw new Error("file too large (10 MB max)");
    level = new Uint8Array(await f.arrayBuffer());
  }
  const name = field("name").replace(/[^\w .-]/g, "").slice(0, 80) || "project";
  return { project, script, name, level };
}

async function readScript(request) {
  const s = await request.text();
  if (s.length > MAX_SCRIPT) throw new Error("script too large (200 KB max)");
  return s;
}

export default {
  async fetch(request) {
    const { pathname } = new URL(request.url);
    try {
      if (request.method === "GET" && pathname === "/") {
        return new Response(html, { headers: { "content-type": "text/html; charset=utf-8" } });
      }
      if (request.method !== "POST") return text("not found", 404);

      switch (pathname) {
        case "/api/preview":
          return json(preview(await readScript(request)));
        case "/api/check":
          return text(check(await readScript(request)));
        case "/api/base":
          return json(base(await readFile(await request.formData())));
        case "/api/inspect":
          return text(inspect(await readFile(await request.formData())));
        case "/api/objects":
          return text(objects(await readFile(await request.formData())));
        case "/api/patch": {
          const form = await request.formData();
          const script = form.get("script");
          if (typeof script !== "string") throw new Error("no script");
          if (script.length > MAX_SCRIPT) throw new Error("script too large (200 KB max)");
          const out = patch(await readFile(form), script);
          return new Response(out, {
            headers: {
              "content-type": "application/octet-stream",
              "content-disposition": 'attachment; filename="level.patched.gdlvl"',
            },
          });
        }
        case "/api/open": {
          const t = await request.text();
          if (t.length > MAX_FILE) throw new Error("project too large (10 MB max)");
          return json(open_project(t));
        }
        case "/api/save": {
          const f = await readProjectForm(request);
          return text(save_project(f.project, f.script, f.name, f.level));
        }
        case "/api/compile": {
          const f = await readProjectForm(request);
          const out = compile_project(f.project, f.script, f.name, f.level);
          return new Response(out, {
            headers: {
              "content-type": "application/octet-stream",
              "content-disposition": 'attachment; filename="' + f.name + '.compiled.gdlvl"',
            },
          });
        }
        default:
          return text("not found", 404);
      }
    } catch (e) {
      return text(String(e && e.message ? e.message : e), 400);
    }
  },
};
