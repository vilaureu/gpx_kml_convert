use std::collections::HashMap;
use std::fmt::Write;
use std::io::{self, Read};

use gpx::{Metadata, Waypoint};
use kml::types::{AltitudeMode, Coord, Geometry, Placemark, Point};
use kml::{types::Element, Kml, KmlDocument, KmlVersion, KmlWriter};

const XML_HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>"#;
const NAMESPACES: &[(&str, &str)] = &[
    ("xmlns", "http://www.opengis.net/kml/2.2"),
    ("xmlns:atom", "http://www.w3.org/2005/Atom"),
];
const DEFAULT_OPEN: &str = "1";

type CoordValue = f64;

pub fn convert(source: impl Read, mut sink: impl io::Write) {
    let gpx = gpx::read(source).unwrap();

    let mut elements = vec![simple_kelem("open", DEFAULT_OPEN)];
    push_metadata(gpx.metadata.unwrap_or_default(), gpx.creator, &mut elements);

    for waypoint in gpx.waypoints {
        elements.push(convert_waypoint(waypoint));
    }

    let document = Kml::Document {
        elements,
        attrs: Default::default(),
    };
    let namespaces = NAMESPACES
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let kml = Kml::<CoordValue>::KmlDocument(KmlDocument {
        version: KmlVersion::V22,
        attrs: namespaces,
        elements: vec![document],
    });

    writeln!(&mut sink, "{XML_HEAD}").unwrap();
    let mut writer = KmlWriter::from_writer(&mut sink);
    writer.write(&kml).unwrap();
    writeln!(&mut sink).unwrap();
}

fn push_metadata(metadata: Metadata, creator: Option<String>, elements: &mut Vec<Kml<CoordValue>>) {
    if let Some(name) = metadata.name {
        elements.push(simple_kelem("name", name));
    }

    let mut children = vec![];
    if let Some(author) = metadata.author {
        let mut name = author.name.unwrap_or_default();
        let mail = author.email.unwrap_or_default();
        if !name.is_empty() && !mail.is_empty() {
            name.push(' ');
        }
        if !mail.is_empty() {
            write!(name, "<{mail}>").unwrap();
        }
        if !name.is_empty() {
            children.push(simple_element("atom:name", name));
        }

        if let Some(link) = author.link {
            children.push(atom_link(link.href));
        }
    }
    if !children.is_empty() {
        elements.push(Kml::Element(Element {
            name: "atom:author".to_string(),
            children,
            ..Default::default()
        }));
    }

    for link in metadata.links {
        elements.push(Kml::Element(atom_link(link.href)));
    }

    let mut description = metadata
        .description
        .map(|mut d| {
            d.push('\n');
            d
        })
        .unwrap_or_default();
    if metadata.time.is_some() || creator.is_some() {
        description.push_str("Created");
        if let Some(time) = metadata.time {
            write!(description, " {}", &time.to_rfc2822()).unwrap();
        }
        if let Some(ref creator) = creator {
            write!(description, " by {}", creator).unwrap();
        }
        description.push('\n');
    }
    if let Some(keywords) = metadata.keywords {
        writeln!(description, "Keywords: {}", keywords).unwrap();
    }
    if let Some(copyright) = metadata
        .copyright
        .filter(|c| c.author.is_some() || c.year.is_some() || c.license.is_some())
    {
        description.push_str("Copyright");
        if let Some(author) = copyright.author {
            write!(description, " {}", author).unwrap();
        }
        if let Some(year) = copyright.year {
            write!(description, " {}", year).unwrap();
        }
        if let Some(license) = copyright.license {
            write!(description, " under {}", license).unwrap();
        }
        description.push('\n');
    }
    if !description.is_empty() {
        elements.push(simple_kelem("description", description));
    }
}

fn convert_waypoint(waypoint: Waypoint) -> Kml<CoordValue> {
    let point = waypoint.point();

    let mut children = vec![];
    for link in waypoint.links {
        children.push(atom_link(link.href));
    }

    let mut description = waypoint
        .description
        .map(|mut d| {
            d.push('\n');
            d
        })
        .unwrap_or_default();
    if let Some(comment) = waypoint.comment {
        writeln!(description, "{}", comment).unwrap();
    }
    if let Some(time) = waypoint.time {
        writeln!(description, "Created {}", time.to_rfc2822()).unwrap();
    }
    if let Some(source) = waypoint.source {
        writeln!(description, "Source: {}", source).unwrap();
    }
    if let Some(typ) = waypoint._type {
        writeln!(description, "Type: {}", typ).unwrap();
    }

    Kml::Placemark(Placemark {
        name: waypoint.name,
        description: Some(description).filter(|d| !d.is_empty()),
        geometry: Some(Geometry::Point(Point {
            coord: Coord {
                x: point.x(),
                y: point.y(),
                z: waypoint.elevation,
            },
            altitude_mode: if waypoint.elevation.is_some() {
                AltitudeMode::Absolute
            } else {
                Default::default()
            },
            ..Default::default()
        })),
        children,
        ..Default::default()
    })
}

fn simple_kelem(name: impl Into<String>, content: impl Into<String>) -> Kml<CoordValue> {
    Kml::Element(simple_element(name, content))
}

fn simple_element(name: impl Into<String>, content: impl Into<String>) -> Element {
    Element {
        name: name.into(),
        content: Some(content.into()),
        ..Default::default()
    }
}

fn atom_link(href: impl Into<String>) -> Element {
    Element {
        name: "atom:link".to_string(),
        attrs: HashMap::from([("href".to_string(), href.into())]),
        ..Default::default()
    }
}
