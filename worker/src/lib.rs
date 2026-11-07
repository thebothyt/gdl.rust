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
