// Copyright 2022 Viktor Reusch
//
// This file is part of gpx_kml_convert.
//
// gpx_kml_convert is free software: you can redistribute it and/or modify it
// under the terms of the GNU Affero General Public License as published by the
// Free Software Foundation, either version 3 of the License, or (at your
// option) any later version.
//
// gpx_kml_convert is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
// FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License
// for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with gpx_kml_convert. If not, see <https://www.gnu.org/licenses/>.

//! This is a WASM wrapper for `gpx_kml_convert`.

use wasm_bindgen::{prelude::wasm_bindgen, JsError};

/// This wraps `gpx_kml_convert::convert` for interfacing with JS.
#[wasm_bindgen]
pub fn convert(source: &[u8]) -> Result<Box<[u8]>, JsError> {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    let mut sink = vec![];
    gpx_kml_convert::convert(source, &mut sink)?;
    Ok(sink.into_boxed_slice())
}
