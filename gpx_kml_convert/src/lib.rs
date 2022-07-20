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

//! Library for converting from [GPX](https://www.topografix.com/gpx.asp) to
//! [KML](https://developers.google.com/kml).
//!
//! It reads in GPX waypoints, routes, and tours and converts them to KML for
//! visualization.
//!
//! See [`convert`] for information on how to use this library.

use std::collections::HashMap;
use std::fmt::Write;
use std::io::{self, Read};

use gpx::{errors::GpxError, Link, Metadata, Route, Track, TrackSegment, Waypoint};
use kml::types::{AltitudeMode, Coord, Geometry, LineString, MultiGeometry, Placemark, Point};
use kml::{types::Element, Kml, KmlDocument, KmlVersion, KmlWriter};
use thiserror::Error;

/// This line needs to be prepended to the KML output.
const XML_HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>"#;
/// Namespace attributes for the `<kml>` tag.
const NAMESPACES: &[(&str, &str)] = &[
    ("xmlns", "http://www.opengis.net/kml/2.2"),
    ("xmlns:atom", "http://www.w3.org/2005/Atom"),
];
/// Default value for the open attribute of the main KML _Document_.
const DEFAULT_OPEN: &str = "1";
/// Default value for tessellating lines in KML.
const DEFAULT_TESSELLATE: bool = true;

/// Use double precision for coordinate values.
type CoordValue = f64;

/// Error returned from the [`convert`] function.
#[derive(Error, Debug)]
pub enum Error {
    /// GPX reading failed.
    #[error("reading GPX failed: {0}")]
    Gpx(#[from] GpxError),
    /// KML writing failed.
    #[error("writing KML failed: {0}")]
    Kml(#[from] kml::Error),
}

/// Read a GPX file and write a KML file.
///
/// A complete GPX file is read from `source`. The converted data is written as
/// a complete KML file to `sink`.
///
/// If an error occurs, the function returns immediately. The `source` and
/// `sink` might have been modified in this case.
///
/// # Example
/// ```
/// # use gpx_kml_convert::convert;
/// #
/// let source = r#"
/// <?xml version="1.0" encoding="UTF-8"?>
/// <gpx xmlns="http://www.topografix.com/GPX/1/1" version="1.1">
///     <wpt lat="48.858222" lon="2.2945"><name>Eiffel Tower</name></wpt>
/// </gpx>
/// "#;
/// let mut sink = vec![];
///
/// convert(source.as_bytes(), &mut sink).expect("conversion failed");
///
/// let kml = String::from_utf8(sink).expect("KML data is not valid UTF-8");
/// assert!(kml.contains("<kml"));
/// assert!(kml.contains("2.2945"));
/// assert!(kml.contains("48.858222"));
/// assert!(kml.contains("Eiffel Tower"));
/// ```
pub fn convert(source: impl Read, mut sink: impl io::Write) -> Result<(), Error> {
    let gpx = gpx::read(source)?;

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
    writer.write(&kml)?;
    writeln!(&mut sink).unwrap();

    Ok(())
}

/// Convert the GPX `metadata` and `creator` to KML.
///
/// The converted data is pushed to `elements`.
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
    let time = metadata.time.and_then(|t| t.format().ok());
    if time.is_some() || creator.is_some() {
        description.push_str("Created");
        if let Some(time) = time {
            write!(description, " {}", time).unwrap();
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

/// Convert a GPX `waypoint`.
///
/// This marks a single point. It is converted to a KML _Point_.
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
        time: waypoint.time.and_then(|t| t.format().ok()),
        source: waypoint.source,
        typ: waypoint._type,
        geometry,
    })
}

/// Convert a GPX `route`.
///
/// This is a continuous tour of GPX waypoints. It is converted to a KML
/// _LineString_.
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

/// Convert a GPX `track`.
///
/// This is a structure containing multiple continuous segments of GPX
/// waypoints. It is converted to a KML _MultiGeometry_. Each segment is
/// converted with [`convert_segment`].
fn convert_track(track: Track) -> Kml {
    let geometries = track.segments.into_iter().map(convert_segment).collect();

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

/// Convert a single track `segment` to a KML _LineString_.
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

/// Argument for the [`create_placemark`] function.
struct PlacemarkArgs {
    name: Option<String>,
    links: Vec<Link>,
    description: Option<String>,
    comment: Option<String>,
    time: Option<String>,
    source: Option<String>,
    /// _type_ attribute in GPX.
    typ: Option<String>,
    geometry: Geometry,
}

/// Create a KML _Placemark_, which describes displayed geometry.
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
        writeln!(description, "Created {}", time).unwrap();
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

/// Create a simple KML element with `name` and `content`.
fn simple_kelem(name: impl Into<String>, content: impl Into<String>) -> Kml<CoordValue> {
    Kml::Element(simple_element(name, content))
}

/// Create a simple KML element with `name` and `content`.
fn simple_element(name: impl Into<String>, content: impl Into<String>) -> Element {
    Element {
        name: name.into(),
        content: Some(content.into()),
        ..Default::default()
    }
}

/// Create a link referencing `href` following the
/// [Atom schema](https://www.w3.org/2005/Atom).
fn atom_link(href: impl Into<String>) -> Element {
    Element {
        name: "atom:link".to_string(),
        attrs: HashMap::from([("href".to_string(), href.into())]),
        ..Default::default()
    }
}
