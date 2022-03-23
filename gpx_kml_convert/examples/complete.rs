// Copyright 2021, 2022 Viktor Reusch
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

use std::{fs::File, io::stdout, path::Path};

use gpx_kml_convert::convert;

const RESOURCES: &str = "./resources/";

fn main() {
    let mut source =
        File::open(Path::new(RESOURCES).join("complete.gpx")).expect("complete.gpx not found");
    let mut sink = stdout();
    convert(&mut source, &mut sink).unwrap()
}
