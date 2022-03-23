use wasm_bindgen::{prelude::wasm_bindgen, JsError};

#[wasm_bindgen]
pub fn convert(source: &[u8]) -> Result<Box<[u8]>, JsError> {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    let mut sink = vec![];
    gpx_kml_convert::convert(source, &mut sink)?;
    Ok(sink.into_boxed_slice())
}
