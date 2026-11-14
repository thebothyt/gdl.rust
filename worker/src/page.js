// The editor page (HTML + CSS + JS in one string). Kept in its own file so index.js stays small.
// String.raw keeps the backslashes in the page script exactly as written.

export const DEMO = String.raw`let stairs = group()

for i in 0..10 {
  let b = obj(1, 15 + i * 30, 15 + i * 30)
  add_group(b, stairs)
}

move(400, 15, stairs, 0, 90, 1.5)
print("done")
`;

export const PAGE = String.raw`<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>gdl editor</title>
<style>
  :root { color-scheme: light dark; --bg: #fff; --fg: #111; --panel: #f2f2f4; --grid: #00000014; --base: #8a8f99; --mine: #2f7bff; --trig: #ff9f1c; --sel: #e5195f; }
  @media (prefers-color-scheme: dark) { :root { --bg: #15161a; --fg: #eee; --panel: #22242b; --grid: #ffffff14; --base: #7b808c; --mine: #5b9bff; } }
  * { box-sizing: border-box; }
  body { font: 16px/1.4 system-ui, sans-serif; margin: 0; padding: 12px 16px 24px; max-width: 900px; margin-inline: auto; background: var(--bg); color: var(--fg); }
  h1 { font-size: 20px; margin: 4px 0 10px; }
  textarea { width: 100%; height: 220px; font: 14px/1.4 ui-monospace, monospace; padding: 8px; tab-size: 2; background: var(--panel); color: var(--fg); border: 1px solid #8884; border-radius: 6px; }
  button, input[type=file] { font: inherit; margin: 6px 6px 6px 0; }
  button { padding: 8px 14px; }
  .row { display: flex; flex-wrap: wrap; align-items: center; gap: 4px; }
  #view { position: relative; margin: 10px 0; border: 1px solid #8884; border-radius: 6px; overflow: hidden; background: var(--panel); }
  canvas { display: block; width: 100%; height: 340px; touch-action: none; cursor: grab; }
  #legend { font-size: 13px; opacity: .85; margin: 0 0 4px; }
  #legend i { display: inline-block; width: 10px; height: 10px; margin: 0 4px 0 10px; vertical-align: -1px; border-radius: 2px; }
  #zoom { position: absolute; right: 8px; top: 8px; display: flex; flex-direction: column; gap: 4px; }
  #zoom button { margin: 0; padding: 4px 10px; font-size: 18px; line-height: 1; }
  .note { font-size: 13px; opacity: .8; margin: 2px 0 8px; }
  #info { font: 13px/1.4 ui-monospace, monospace; background: var(--panel); border-radius: 6px; padding: 8px 10px; min-height: 2.6em; white-space: pre-wrap; word-break: break-all; }
  pre#out { background: var(--panel); padding: 10px; border-radius: 6px; white-space: pre-wrap; word-break: break-word; min-height: 3em; }
</style></head><body>
<h1>gdl editor</h1>
<textarea id="script" spellcheck="false" autocapitalize="off" autocorrect="off">__DEMO__</textarea>
<div id="status" aria-live="polite" style="font-size:14px;min-height:1.4em"></div>

<div class="row">
  <label>Level file (.gdlvl): <input type="file" id="file"></label>
</div>
<div class="row">
  <label>Project (.gdlrust): <input type="file" id="proj" accept=".gdlrust,text/plain"></label>
  <button id="saveproj">Save .gdlrust</button>
  <button id="compile">Compile &rarr; .gdlvl</button>
</div>
<p class="note">Your work lives in the <b>.gdlrust</b> file you save. This page uses no cookies, localStorage or any other storage, so nothing is kept if you close it without saving.</p>
<div class="row">
  <button id="inspect">Inspect</button>
  <button id="objects">List objects</button>
  <button id="patch">Patch &amp; download</button>
</div>

<p id="legend"><i style="background:var(--base)"></i>already in level<i style="background:var(--mine)"></i>from your script<i style="background:var(--trig)"></i>trigger<i style="background:var(--sel)"></i>selected</p>
<div id="view">
  <canvas id="cv"></canvas>
  <div id="zoom"><button id="zin" aria-label="Zoom in">+</button><button id="zout" aria-label="Zoom out">&minus;</button><button id="fit" aria-label="Fit" style="font-size:13px">fit</button></div>
</div>
<div id="info">Tap an object to see its properties and the script line that made it. Drag to pan, pinch or use +/&minus; to zoom.</div>

<pre id="out">Ready.</pre>

<script>
(function () {
  var $ = function (id) { return document.getElementById(id); };
  var scriptEl = $("script"), fileEl = $("file"), out = $("out"), info = $("info"), status = $("status");
  var cv = $("cv"), ctx = cv.getContext("2d");

  // ---- trigger names (object id -> short label) ----
  var NAMES = { 901: "MOVE", 1346: "ROTATE", 1585: "ANIM", 1006: "PULSE", 1611: "COUNT", 1007: "ALPHA",
    1049: "TOGGLE", 1347: "FOLLOW", 1268: "SPAWN", 1616: "STOP", 1595: "TOUCH", 1811: "ICOUNT", 1812: "DEATH",
    1814: "FOLLOWY", 1815: "COLL", 1817: "PICKUP", 1818: "BGON", 1819: "BGOFF", 1520: "SHAKE", 899: "COLOR",
    1612: "HIDE", 1613: "SHOW", 3619: "ITEMEDIT", 3620: "ITEMCMP", 2903: "GRADIENT", 1913: "ZOOM", 1914: "STATIC",
    1916: "OFFSET", 2015: "CAMROT", 2062: "EDGE", 2925: "CAMMODE", 1934: "SONG", 3602: "SFX", 3614: "TIMER",
    3615: "TIMEEVT", 3617: "TIMECTL", 1912: "RANDOM", 2068: "ADVRAND", 3618: "RESET", 3609: "ICOLL", 2066: "GRAVITY",
    1935: "TIMEWARP", 3606: "BGSPEED", 3600: "END", 2067: "SCALE", 1917: "REVERSE" };

  // ---- shapes for common non-trigger objects (everything else is a square) ----
  var SPIKES = { 8: 1, 39: 1, 103: 1, 144: 1, 145: 1 }, SLABS = { 40: 1, 68: 1, 147: 1 },
      ORBS = { 36: 1, 84: 1, 141: 1 }, PADS = { 35: 1, 67: 1, 140: 1 };

  function show(t) { out.textContent = t; }
  function setStatus(t, bad) { status.textContent = t; status.style.color = bad ? "#e5195f" : ""; }

  // ---- parse "k,v,k,v;k,v..." into drawable objects ----
  function parseObjects(str, source) {
    var res = [];
    var recs = (str || "").split(";");
    for (var i = 0; i < recs.length; i++) {
      var r = recs[i]; if (!r) continue;
      var p = r.split(","), o = { raw: r, src: source, idx: i, line: 0, props: {} };
      for (var j = 0; j + 1 < p.length; j += 2) o.props[p[j]] = p[j + 1];
      o.id = parseInt(o.props["1"], 10); o.x = parseFloat(o.props["2"]); o.y = parseFloat(o.props["3"]);
      if (!isFinite(o.x) || !isFinite(o.y)) continue;
      o.trigger = o.props["36"] === "1" || NAMES[o.id] !== undefined;
      o.groups = o.props["57"] ? o.props["57"].split(".") : [];
      o.target = o.props["51"] || null;
      res.push(o);
    }
    return res;
  }

  var base = [], mine = [], lines = [], selected = null;
  var cam = { x: 0, y: 0, s: 1 };            // world->screen: sx = (x - cam.x) * s ; sy = H - (y - cam.y) * s

  function sizeCanvas() {
    var r = cv.getBoundingClientRect(), d = window.devicePixelRatio || 1;
    cv.width = Math.max(1, Math.round(r.width * d)); cv.height = Math.max(1, Math.round(r.height * d));
    ctx.setTransform(d, 0, 0, d, 0, 0);
  }
  function W() { return cv.getBoundingClientRect().width; }
  function H() { return cv.getBoundingClientRect().height; }
  function toScreen(x, y) { return [(x - cam.x) * cam.s, H() - (y - cam.y) * cam.s]; }
  function toWorld(sx, sy) { return [sx / cam.s + cam.x, (H() - sy) / cam.s + cam.y]; }

  function fit() {
    var all = base.concat(mine);
    if (!all.length) { cam.x = -30; cam.y = -30; cam.s = 1; draw(); return; }
    var x0 = 1e18, y0 = 1e18, x1 = -1e18, y1 = -1e18;
    all.forEach(function (o) { x0 = Math.min(x0, o.x); x1 = Math.max(x1, o.x); y0 = Math.min(y0, o.y); y1 = Math.max(y1, o.y); });
    var pad = 45, w = Math.max(x1 - x0, 60) + pad * 2, h = Math.max(y1 - y0, 60) + pad * 2;
    cam.s = Math.min(W() / w, H() / h);
    cam.x = (x0 + x1) / 2 - W() / cam.s / 2; cam.y = (y0 + y1) / 2 - H() / cam.s / 2;
    draw();
  }

  function css(name) { return getComputedStyle(document.documentElement).getPropertyValue(name).trim(); }

  function drawObj(o, color, isSel) {
    var p = toScreen(o.x, o.y), half = 15 * cam.s, sz = Math.max(2, half * 2 - 1);
    if (p[0] < -sz || p[1] < -sz || p[0] > W() + sz || p[1] > H() + sz) return;
    ctx.fillStyle = color; ctx.strokeStyle = color;
    if (o.trigger) {
      var r = Math.max(3, half * 0.8);
      ctx.beginPath(); ctx.moveTo(p[0], p[1] - r); ctx.lineTo(p[0] + r, p[1]); ctx.lineTo(p[0], p[1] + r); ctx.lineTo(p[0] - r, p[1]); ctx.closePath();
      ctx.globalAlpha = 0.85; ctx.fill(); ctx.globalAlpha = 1;
      if (cam.s > 0.5) { ctx.font = "10px system-ui"; ctx.fillStyle = css("--fg"); ctx.textAlign = "center"; ctx.fillText(NAMES[o.id] || String(o.id), p[0], p[1] - r - 3); }
    } else {
      ctx.globalAlpha = o.src === "base" ? 0.55 : 0.9;
      var rot = (parseFloat(o.props["6"]) || 0) * Math.PI / 180, h2 = sz / 2;
      ctx.save(); ctx.translate(p[0], p[1]); ctx.rotate(rot);
      if (SPIKES[o.id]) { ctx.beginPath(); ctx.moveTo(-h2, h2); ctx.lineTo(0, -h2); ctx.lineTo(h2, h2); ctx.closePath(); ctx.fill(); }
      else if (SLABS[o.id]) ctx.fillRect(-h2, 0, sz, h2);
      else if (ORBS[o.id]) { ctx.beginPath(); ctx.arc(0, 0, h2 * 0.7, 0, 6.2832); ctx.fill(); }
      else if (PADS[o.id]) ctx.fillRect(-h2, h2 * 0.5, sz, h2 * 0.5);
      else ctx.fillRect(-h2, -h2, sz, sz);
      ctx.restore();
      ctx.globalAlpha = 1;
    }
    if (isSel) { ctx.lineWidth = 2; ctx.strokeStyle = css("--sel"); ctx.strokeRect(p[0] - half, p[1] - half, half * 2, half * 2); }
  }

  function draw() {
    var w = W(), h = H(); ctx.clearRect(0, 0, w, h);
    // grid (30-unit cells), only when cells are big enough to read
    var cell = 30 * cam.s;
    if (cell > 6) {
      ctx.strokeStyle = css("--grid"); ctx.lineWidth = 1; ctx.beginPath();
      var gx = Math.floor(cam.x / 30) * 30, gy = Math.floor(cam.y / 30) * 30;
      for (var x = gx; (x - cam.x) * cam.s < w; x += 30) { var sx = Math.round((x - cam.x) * cam.s) + 0.5; ctx.moveTo(sx, 0); ctx.lineTo(sx, h); }
      for (var y = gy; (y - cam.y) * cam.s < h; y += 30) { var sy = Math.round(h - (y - cam.y) * cam.s) + 0.5; ctx.moveTo(0, sy); ctx.lineTo(w, sy); }
      ctx.stroke();
    }
    // ground line (y = 0 is the floor in the editor's coordinates)
    ctx.strokeStyle = css("--fg"); ctx.globalAlpha = 0.4; ctx.beginPath();
    var fy = toScreen(0, 0)[1]; ctx.moveTo(0, fy); ctx.lineTo(w, fy); ctx.stroke(); ctx.globalAlpha = 1;

    var all = base.concat(mine);
    // links: each trigger -> objects in its target group
    var byGroup = {};
    all.forEach(function (o) { o.groups.forEach(function (g) { (byGroup[g] = byGroup[g] || []).push(o); }); });
    ctx.lineWidth = 1; ctx.setLineDash([4, 4]); ctx.strokeStyle = css("--trig");
    all.forEach(function (t) {
      if (!t.trigger || !t.target || !byGroup[t.target]) return;
      var a = toScreen(t.x, t.y), targets = byGroup[t.target], n = Math.min(targets.length, 200);
      ctx.globalAlpha = (t === selected) ? 0.9 : 0.25;
      ctx.beginPath();
      for (var i = 0; i < n; i++) { var b = toScreen(targets[i].x, targets[i].y); ctx.moveTo(a[0], a[1]); ctx.lineTo(b[0], b[1]); }
      ctx.stroke();
    });
    ctx.setLineDash([]); ctx.globalAlpha = 1;

    base.forEach(function (o) { drawObj(o, o.trigger ? css("--trig") : css("--base"), o === selected); });
    mine.forEach(function (o) { drawObj(o, o.trigger ? css("--trig") : css("--mine"), o === selected); });
  }

  // ---- selection ----
  function pick(sx, sy) {
    var w = toWorld(sx, sy), best = null, bd = 1e18, all = mine.concat(base);   // script objects win ties
    all.forEach(function (o) {
      var d = Math.max(Math.abs(o.x - w[0]), Math.abs(o.y - w[1]));
      if (d < bd) { bd = d; best = o; }
    });
    var tol = Math.max(15, 14 / cam.s);
    return bd <= tol ? best : null;
  }

  function describe(o) {
    var s = (o.src === "mine" ? "script" : "level") + " object, id " + o.id + (NAMES[o.id] ? " (" + NAMES[o.id] + " trigger)" : "") +
      " at (" + o.x + ", " + o.y + ")";
    if (o.src === "mine" && o.line) s += "\nmade by script line " + o.line;
    if (o.target) s += "\ntargets group " + o.target + (countGroup(o.target) ? " (" + countGroup(o.target) + " objects)" : " (no objects in it)");
    if (o.groups.length) s += "\nin group(s) " + o.groups.join(", ");
    return s + "\n" + o.raw;
  }
  function countGroup(g) { var n = 0; base.concat(mine).forEach(function (o) { if (o.groups.indexOf(g) >= 0) n++; }); return n; }

  function selectLine(line) {            // highlight a script line in the textarea
    var t = scriptEl.value.split("\n"), start = 0;
    for (var i = 0; i < line - 1 && i < t.length; i++) start += t[i].length + 1;
    var end = start + (t[line - 1] || "").length;
    scriptEl.focus({ preventScroll: true }); scriptEl.setSelectionRange(start, end);
  }

  // ---- pointer input: drag = pan, two fingers = pinch, tap = select ----
  var ptrs = {}, moved = 0, lastDist = 0;
  function pos(e) { var r = cv.getBoundingClientRect(); return [e.clientX - r.left, e.clientY - r.top]; }
  cv.addEventListener("pointerdown", function (e) { cv.setPointerCapture(e.pointerId); ptrs[e.pointerId] = pos(e); moved = 0; lastDist = 0; cv.style.cursor = "grabbing"; });
  cv.addEventListener("pointermove", function (e) {
    if (!ptrs[e.pointerId]) return;
    var prev = ptrs[e.pointerId], cur = pos(e), ids = Object.keys(ptrs);
    if (ids.length === 1) {
      cam.x -= (cur[0] - prev[0]) / cam.s; cam.y += (cur[1] - prev[1]) / cam.s; moved += Math.abs(cur[0] - prev[0]) + Math.abs(cur[1] - prev[1]);
    } else if (ids.length === 2) {
      ptrs[e.pointerId] = cur;
      var a = ptrs[ids[0]], b = ptrs[ids[1]], d = Math.hypot(a[0] - b[0], a[1] - b[1]);
      if (lastDist) zoomAt((a[0] + b[0]) / 2, (a[1] + b[1]) / 2, d / lastDist);
      lastDist = d; moved += 10; draw(); return;
    }
    ptrs[e.pointerId] = cur; draw();
  });
  function endPtr(e) {
    var was = ptrs[e.pointerId]; delete ptrs[e.pointerId]; cv.style.cursor = "grab"; lastDist = 0;
    if (was && moved < 6 && Object.keys(ptrs).length === 0) {
      selected = pick(was[0], was[1]);
      info.textContent = selected ? describe(selected) : "Nothing here.";
      if (selected && selected.src === "mine" && selected.line) selectLine(selected.line);
      draw();
    }
  }
  cv.addEventListener("pointerup", endPtr); cv.addEventListener("pointercancel", endPtr);
  function zoomAt(sx, sy, f) {
    var before = toWorld(sx, sy); cam.s = Math.min(8, Math.max(0.02, cam.s * f));
    var after = toWorld(sx, sy); cam.x += before[0] - after[0]; cam.y += before[1] - after[1];
  }
  cv.addEventListener("wheel", function (e) { e.preventDefault(); var p = pos(e); zoomAt(p[0], p[1], e.deltaY < 0 ? 1.15 : 1 / 1.15); draw(); }, { passive: false });
  $("zin").onclick = function () { zoomAt(W() / 2, H() / 2, 1.4); draw(); };
  $("zout").onclick = function () { zoomAt(W() / 2, H() / 2, 1 / 1.4); draw(); };
  $("fit").onclick = fit;
  window.addEventListener("resize", function () { sizeCanvas(); draw(); });

  // caret in the script -> highlight that line's objects
  function caretLine() { return scriptEl.value.slice(0, scriptEl.selectionStart).split("\n").length; }

  // ---- talking to the Worker ----
  var timer = 0, seq = 0;
  function runPreview(first) {
    var my = ++seq;
    fetch("/api/preview", { method: "POST", body: scriptEl.value }).then(function (r) { return r.json(); }).then(function (j) {
      if (my !== seq) return;
      if (!j.ok) { setStatus(j.error, true); return; }            // keep the last good picture on screen
      mine = parseObjects(j.objects, "mine");
      mine.forEach(function (o, i) { o.line = j.lines[i] || 0; });
      selected = null; info.textContent = "Tap an object to see its properties and the script line that made it.";
      setStatus(mine.length + " object(s) from your script" + (j.logs.length ? "  |  print: " + j.logs.join(" | ") : ""));
      if (first || !base.length) fit(); else draw();
    }).catch(function () { setStatus("couldn't reach the Worker", true); });
  }
  scriptEl.addEventListener("input", function () { clearTimeout(timer); timer = setTimeout(function () { runPreview(false); }, 350); });

  function fileForm(withScript) {
    var f = fileEl.files[0];
    if (!f) { show("Pick a .gdlvl file first."); return null; }
    var fd = new FormData(); fd.append("file", f); if (withScript) fd.append("script", scriptEl.value); return fd;
  }
  fileEl.addEventListener("change", function () {
    var fd = fileForm(false); if (!fd) return;
    fetch("/api/base", { method: "POST", body: fd }).then(function (r) { return r.json(); }).then(function (j) {
      if (!j.ok) { base = []; show("error: " + j.error); draw(); return; }
      base = parseObjects(j.objects, "base"); show("Loaded " + base.length + " object(s) from the level."); fit();
    });
  });
  function textPost(path, withScript) {
    var fd = fileForm(withScript); if (!fd) return;
    fetch(path, { method: "POST", body: fd }).then(function (r) { return r.text(); }).then(show);
  }
  $("inspect").onclick = function () { textPost("/api/inspect", false); };
  $("objects").onclick = function () { textPost("/api/objects", false); };
  $("patch").onclick = function () {
    var fd = fileForm(true); if (!fd) return;
    fetch("/api/patch", { method: "POST", body: fd }).then(function (r) {
      if (!r.ok) return r.text().then(function (t) { show("error: " + t); });
      var name = (fileEl.files[0].name || "level.gdlvl").replace(/\.gdlvl$/, "") + ".patched.gdlvl";
      return r.blob().then(function (b) {
        var a = document.createElement("a"); a.href = URL.createObjectURL(b); a.download = name; a.click(); show("Downloaded " + name);
      });
    });
  };

  // ---- projects (.gdlrust): the file is the only place your work is kept ----
  var projEl = $("proj"), projectText = "", projectName = "project", dirty = false;
  function download(blob, name) {
    var a = document.createElement("a"); a.href = URL.createObjectURL(blob); a.download = name; a.click();
    setTimeout(function () { URL.revokeObjectURL(a.href); }, 1000);
  }
  function projForm() {
    var fd = new FormData();
    fd.append("project", projectText); fd.append("script", scriptEl.value); fd.append("name", projectName);
    if (fileEl.files[0]) fd.append("file", fileEl.files[0]);
    return fd;
  }
  scriptEl.addEventListener("input", function () { dirty = true; });
  window.addEventListener("beforeunload", function (e) { if (dirty) { e.preventDefault(); e.returnValue = ""; } });
  fileEl.addEventListener("change", function () {
    if (!projectText && fileEl.files[0]) projectName = fileEl.files[0].name.replace(/\.gdlvl$/i, "");
  });
  projEl.addEventListener("change", function () {
    var f = projEl.files[0]; if (!f) return;
    f.text().then(function (t) {
      return fetch("/api/open", { method: "POST", body: t }).then(function (r) { return r.json(); }).then(function (j) {
        if (!j.ok) { show("error: " + j.error); return; }
        projectText = t; projectName = (j.name || f.name.replace(/\.gdlrust$/i, "")).replace(/[^\w .-]/g, "") || "project";
        scriptEl.value = j.script; base = parseObjects(j.objects, "base"); dirty = false;
        show("Opened " + f.name + ": " + base.length + " object(s) already in the level, plus your script.");
        runPreview(true);
      });
    });
  });
  $("saveproj").onclick = function () {
    fetch("/api/save", { method: "POST", body: projForm() }).then(function (r) {
      return r.text().then(function (t) {
        if (!r.ok) { show("error: " + t); return; }
        projectText = t; dirty = false;
        download(new Blob([t], { type: "text/plain" }), projectName + ".gdlrust");
        show("Saved " + projectName + ".gdlrust");
      });
    });
  };
  $("compile").onclick = function () {
    fetch("/api/compile", { method: "POST", body: projForm() }).then(function (r) {
      if (!r.ok) return r.text().then(function (t) { show("error: " + t); });
      return r.blob().then(function (b) { download(b, projectName + ".compiled.gdlvl"); show("Compiled -> " + projectName + ".compiled.gdlvl"); });
    });
  };

  sizeCanvas(); runPreview(true);
})();
</script></body></html>`;
