use std::io::{Read, Write};

use gpx::Gpx;
use xml::{writer::XmlEvent, EmitterConfig, EventWriter};

pub fn convert(source: &mut impl Read, sink: &mut impl Write) {
    let gpx = gpx::read(source).unwrap();
    let writer = EmitterConfig::new()
        .perform_indent(true)
        .keep_element_names_stack(false)
        .create_writer(sink);
}

trait WriteKML {
    fn write(writer: &mut EventWriter<impl Write>);
}

impl WriteKML for Gpx {
    fn write(writer: &mut EventWriter<impl Write>) {
        let start = XmlEvent::start_element("kml").default_ns("http://www.opengis.net/kml/2.2");
        writer.write(start).unwrap();
    }
}
