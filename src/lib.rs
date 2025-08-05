mod armature;
pub mod entity;
pub mod error;
mod ffi;
mod object;
mod parse;
mod structure;

use std::path::PathBuf;
use std::sync::Once;

use entity::Entity;
use error::ParseError;
use serde::Deserialize;
use tracing::info;

static INIT: Once = Once::new();

#[derive(Deserialize, Debug)]
pub struct Options {
    pub xml_root: PathBuf,
    pub xml_file: String,
    pub armature: bool,
    pub armature_full: bool,
}

pub fn initialize_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    });
}

pub fn parse(options: &Options) -> Result<Entity, ParseError> {
    let timer = std::time::Instant::now();
    info!("Parsing file {:?}", options.xml_file);
    let xml_full_path = options.xml_root.join(&options.xml_file);
    let model_data = structure::parse_xml_file(&xml_full_path)?;
    let entity = Entity::new(options, model_data)?;
    info!("Parsing took {:?}", timer.elapsed());
    Ok(entity)
}
