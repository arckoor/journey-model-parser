mod error;
mod ffi;
mod object;
mod structure;

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Once;

use tracing::{error, info};

use object::ParsedObject;

static INIT: Once = Once::new();

fn initialize_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    });
}

fn write_obj(object: &ParsedObject, path: &Path) {
    let mut file = File::create(path).expect("Failed to create file");
    file.write_all(object.to_obj().as_bytes())
        .expect("Failed to write to file");
}

pub fn parse(xml_file: &Path) -> Vec<ParsedObject> {
    initialize_tracing();
    info!("Parsing file {:?}", xml_file);
    let (data_blocks, render_index_sources) = structure::parse_xml_file(xml_file);
    let mut objects = Vec::new();
    for source in render_index_sources {
        match ParsedObject::new(&data_blocks, source) {
            Ok(object) => objects.push(object),
            Err(e) => {
                error!("Failed to parse model data for {:?}: {}", xml_file, e);
                return Vec::new();
            }
        }
    }
    objects
}

pub fn convert_file(xml_file: &Path) {
    let objects = parse(xml_file);

    let path = xml_file.file_stem().unwrap().to_str().unwrap();

    for (i, object) in objects.iter().enumerate() {
        let obj_file = if objects.len() == 1 {
            format!("{}.obj", path)
        } else {
            format!("{}-{}.obj", path, i + 1)
        };
        let obj_path = xml_file.with_file_name(obj_file);
        write_obj(object, &obj_path);
    }
}
