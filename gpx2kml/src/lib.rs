use std::collections::HashMap;
use std::fmt::Write;
use std::io::{self, Read};

use chrono::{DateTime, Utc};
use gpx::{Link, Metadata, Route, Track, TrackSegment, Waypoint};
use kml::types::{AltitudeMode, Coord, Geometry, LineString, MultiGeometry, Placemark, Point};
use kml::{types::Element, Kml, KmlDocument, KmlVersion, KmlWriter};

const XML_HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>"#;
const NAMESPACES: &[(&str, &str)] = &[
    ("xmlns", "http://www.opengis.net/kml/2.2"),
    ("xmlns:atom", "http://www.w3.org/2005/Atom"),
];
const DEFAULT_OPEN: &str = "1";
const DEFAULT_TESSELLATE: bool = true;

type CoordValue = f64;

pub fn convert(source: impl Read, mut sink: impl io::Write) {
    let gpx = gpx::read(source).unwrap();

    let mut elements = vec![simple_kelem("open", DEFAULT_OPEN)];
    push_metadata(gpx.metadata.unwrap_or_default(), gpx.creator, &mut elements);

    for waypoint in gpx.waypoints {
        elements.push(convert_waypoint(waypoint));
    }

    for route in gpx.routes {
        elements.push(convert_route(route));
    }

    for track in gpx.tracks {
        elements.push(convert_track(track));
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
    let geometry = Geometry::Point(Point {
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
    });

    create_placemark(PlacemarkArgs {
        name: waypoint.name,
        links: waypoint.links,
        description: waypoint.description,
        comment: waypoint.comment,
        time: waypoint.time,
        source: waypoint.source,
        typ: waypoint._type,
        geometry,
    })
}

fn convert_route(route: Route) -> Kml<CoordValue> {
    let mut elevation_avail = false;
    let mut coords = vec![];
    for waypoint in route.points {
        let point = waypoint.point();
        coords.push(Coord {
            x: point.x(),
            y: point.y(),
            z: waypoint.elevation,
        });
        elevation_avail |= waypoint.elevation.is_some();
    }

    let geometry = Geometry::LineString(LineString {
        tessellate: DEFAULT_TESSELLATE,
        altitude_mode: if elevation_avail {
            AltitudeMode::Absolute
        } else {
            Default::default()
        },
        coords,
        ..Default::default()
    });

    create_placemark(PlacemarkArgs {
        name: route.name,
        links: route.links,
        description: route.description,
        comment: route.comment,
        time: None,
        source: route.source,
        typ: route._type,
        geometry,
    })
}

fn convert_track(track: Track) -> Kml {
    let geometries = track
        .segments
        .into_iter()
        .map(|s| convert_segment(s))
        .collect();

    create_placemark(PlacemarkArgs {
        name: track.name,
        links: track.links,
        description: track.description,
        comment: track.comment,
        time: None,
        source: track.source,
        typ: track._type,
        geometry: Geometry::MultiGeometry(MultiGeometry {
            geometries,
            ..Default::default()
        }),
    })
}

fn convert_segment(segment: TrackSegment) -> Geometry {
    let mut elevation_avail = false;
    let mut coords = vec![];
    for waypoint in segment.points {
        let point = waypoint.point();
        coords.push(Coord {
            x: point.x(),
            y: point.y(),
            z: waypoint.elevation,
        });
        elevation_avail |= waypoint.elevation.is_some();
    }

    Geometry::LineString(LineString {
        tessellate: DEFAULT_TESSELLATE,
        altitude_mode: if elevation_avail {
            AltitudeMode::Absolute
        } else {
            Default::default()
        },
        coords,
        ..Default::default()
    })
}

struct PlacemarkArgs {
    name: Option<String>,
    links: Vec<Link>,
    description: Option<String>,
    comment: Option<String>,
    time: Option<DateTime<Utc>>,
    source: Option<String>,
    typ: Option<String>,
    geometry: Geometry,
}

fn create_placemark(args: PlacemarkArgs) -> Kml<CoordValue> {
    let mut children = vec![];
    for link in args.links {
        children.push(atom_link(link.href));
    }

    let mut description = args
        .description
        .map(|mut d| {
            d.push('\n');
            d
        })
        .unwrap_or_default();
    if let Some(comment) = args.comment {
        writeln!(description, "{}", comment).unwrap();
    }
    if let Some(time) = args.time {
        writeln!(description, "Created {}", time.to_rfc2822()).unwrap();
    }
    if let Some(source) = args.source {
        writeln!(description, "Source: {}", source).unwrap();
    }
    if let Some(typ) = args.typ {
        writeln!(description, "Type: {}", typ).unwrap();
    }

    Kml::Placemark(Placemark {
        name: args.name,
        description: Some(description).filter(|d| !d.is_empty()),
        geometry: Some(args.geometry),
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
