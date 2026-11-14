use wasm_bindgen::prelude::*;

/// Run a script and return a short report (never throws): "ok: N object(s)" plus prints,
/// or "error: line N: ...".
#[wasm_bindgen]
pub fn check(script: &str) -> String {
    match gdl_core::run(script) {
        Ok(o) => {
            let mut s = format!("ok: {} object(s)\n", o.objects.len());
            for l in &o.logs {
                s.push_str(&format!("print: {l}\n"));
            }
            s
        }
        Err(e) => format!("error: {e}"),
    }
}

/// Run the script and add its objects to the uploaded .gdlvl; returns the new file bytes.
#[wasm_bindgen]
pub fn patch(file: &[u8], script: &str) -> Result<Vec<u8>, JsValue> {
    let out = gdl_core::run(script).map_err(|e| JsValue::from_str(&e))?;
    gdl_core::codec::patch_level_file(file, &out.objects_string()).map_err(|e| JsValue::from_str(&e))
}

/// Decode a .gdlvl and describe its layout.
#[wasm_bindgen]
pub fn inspect(file: &[u8]) -> Result<String, JsValue> {
    gdl_core::codec::inspect(file).map_err(|e| JsValue::from_str(&e))
}

/// Run a script for the visual preview. Always returns JSON: {"ok":true,"objects":"...","lines":[...],"logs":[...]}
/// or {"ok":false,"error":"line N: ..."}.
#[wasm_bindgen]
pub fn preview(script: &str) -> String {
    gdl_core::preview_json(script)
}

/// Objects already in an uploaded .gdlvl, as JSON, for drawing under the script's objects.
#[wasm_bindgen]
pub fn base(file: &[u8]) -> String {
    gdl_core::base_json(file)
}

/// One object per line (id + raw properties) for looking up IDs and property keys.
#[wasm_bindgen]
pub fn objects(file: &[u8]) -> Result<String, JsValue> {
    gdl_core::codec::list_objects(file).map_err(|e| JsValue::from_str(&e))
}

/// Open a .gdlrust: JSON with its script, header and the objects already in the level.
#[wasm_bindgen]
pub fn open_project(text: &str) -> String {
    gdl_core::project_open_json(text)
}

/// The .gdlrust text for the editor's current state. `project` is the text the page already
/// holds (empty for none); `level` is an uploaded .gdlvl, used to start a new project from it.
#[wasm_bindgen]
pub fn save_project(project: &str, script: &str, name: &str, level: Option<Vec<u8>>) -> Result<String, JsValue> {
    gdl_core::project::Project::for_editor(project, script, name, level.as_deref())
        .and_then(|p| p.to_text())
        .map_err(|e| JsValue::from_str(&e))
}

/// Compile the editor's current state to .gdlvl bytes.
#[wasm_bindgen]
pub fn compile_project(project: &str, script: &str, name: &str, level: Option<Vec<u8>>) -> Result<Vec<u8>, JsValue> {
    gdl_core::project::Project::for_editor(project, script, name, level.as_deref())
        .and_then(|p| p.compile_for_editor(level.as_deref()))
        .map(|c| c.bytes)
        .map_err(|e| JsValue::from_str(&e))
}
