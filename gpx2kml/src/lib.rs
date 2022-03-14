use std::io::{Read, Write};

pub fn convert(source: &mut impl Read, sink: &mut impl Write) {
    let gpx = gpx::read(source).unwrap();
}
