mod entity;
mod error;
mod ffi;
mod object;
mod parse;
mod structure;

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Once;

use entity::Entity;
use error::ParseError;
use tracing::info;

use object::Object;

static INIT: Once = Once::new();

fn initialize_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    });
}

fn write_obj(object: &Object, path: &Path) {
    let mut file = File::create(path).expect("Failed to create file");
    file.write_all(object.to_obj().as_bytes())
        .expect("Failed to write to file");
}

pub fn parse(xml_file: &Path) -> Result<Entity, ParseError> {
    initialize_tracing();
    info!("Parsing file {:?}", xml_file);
    let (data_blocks, render_index_sources, root_node) = structure::parse_xml_file(xml_file);
    Entity::new(data_blocks, render_index_sources, root_node)
}

pub fn convert_file(xml_file: &Path) {
    let entity = parse(xml_file);

    if let Err(e) = entity {
        info!("Failed to parse file: {:?}", e);
        return;
    }

    let entity = entity.unwrap();

    let path = xml_file.file_stem().unwrap().to_string_lossy().to_string();

    for (i, object) in entity.objects.iter().enumerate() {
        let obj_file = if entity.objects.len() == 1 {
            format!("{}.obj", path)
        } else {
            format!("{}-{}.obj", path, i + 1)
        };
        let obj_path = xml_file.with_file_name(obj_file);
        write_obj(object, &obj_path);
    }
}
