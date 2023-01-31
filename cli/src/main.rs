// Copyright 2023 Viktor Reusch
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

//! This is a very simply command-line interface for the GPX-to-KML converter.

use std::{
    io::{stdin, stdout},
    process::ExitCode,
};

use gpx_kml_convert::convert;

/// Currently, this simply converts from STDIN to STDOUT.
fn main() -> ExitCode {
    match convert(&mut stdin(), &mut stdout()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Conversion failed with: {err:?}");
            ExitCode::FAILURE
        }
    }
}
