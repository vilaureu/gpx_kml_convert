use std::{fs::File, io::stdout, path::Path};

use gpx2kml::convert;

const RESOURCES: &str = "./resources/";

fn main() {
    let mut source =
        File::open(Path::new(RESOURCES).join("complete.gpx")).expect("complete.gpx not found");
    let mut sink = stdout();
    convert(&mut source, &mut sink).unwrap()
}
