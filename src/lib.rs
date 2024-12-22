mod armature;
mod entity;
mod error;
mod ffi;
mod object;
mod parse;
mod structure;

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Once;

use entity::Entity;
use error::ParseError;
use serde::Deserialize;
use tracing::info;

use object::Object;

static INIT: Once = Once::new();

#[derive(Deserialize, Debug)]
pub struct Options {
    pub xml_root: PathBuf,
    pub xml_file: String,
    pub armature_full: bool,
}

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

pub fn parse(options: &Options) -> Result<Entity, ParseError> {
    initialize_tracing();
    let timer = std::time::Instant::now();
    info!("Parsing file {:?}", options.xml_file);
    let xml_full_path = options.xml_root.join(&options.xml_file);
    let model_data = structure::parse_xml_file(&xml_full_path)?;
    let entity = Entity::new(options, model_data)?;
    info!("Parsing took {:?}", timer.elapsed());
    Ok(entity)
}

pub fn convert_file(options: &Options) {
    let entity = parse(options);

    if let Err(e) = entity {
        info!("Failed to parse file: {:?}", e);
        return;
    }

    let entity = entity.unwrap();

    for (i, object) in entity.objects.iter().enumerate() {
        let obj_file = if entity.objects.len() == 1 {
            format!("{}.obj", options.xml_file)
        } else {
            format!("{}-{}.obj", options.xml_file, i + 1)
        };
        let obj_path = options.xml_root.join(obj_file);
        write_obj(object, &obj_path);
    }
}
