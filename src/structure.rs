use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use serde::Deserialize;
use serde_xml_rs::from_reader;
use tracing::warn;

#[derive(Deserialize, Clone)]
#[serde(rename = "PSSGFILE")]
pub struct PssgFile {
    #[serde(rename = "PSSGDATABASE")]
    pub database: PssgDatabase,
}

#[derive(Deserialize, Clone)]
pub struct PssgDatabase {
    #[serde(rename = "TYPEINFO")]
    pub type_info: Vec<TypeInfo>,
    #[serde(rename = "LIBRARY")]
    pub libraries: Vec<Library>,
}

#[derive(Deserialize, Clone)]
pub struct TypeInfo {
    #[serde(rename = "typeName")]
    pub type_name: String,
    #[serde(rename = "typeCount")]
    pub type_count: usize,
}

#[derive(Deserialize, Clone)]
pub struct Library {
    #[serde(rename = "type")]
    pub library_type: String,
    #[serde(rename = "DATABLOCK", default)]
    pub data_blocks: Vec<DataBlock>,
    #[serde(rename = "SEGMENTSET", default)]
    pub segment_sets: Vec<SegmentSet>,
}

#[derive(Deserialize, Clone)]
pub struct DataBlock {
    #[serde(rename = "elementCount")]
    pub element_count: usize,
    #[serde(rename = "DATABLOCKSTREAM")]
    pub stream: DataBlockStream,
    #[serde(rename = "DATABLOCKDATA")]
    pub data: Option<DataBlockData>,
    #[serde(rename = "id")]
    pub id: String,
}

#[derive(Deserialize, Clone)]
pub struct DataBlockStream {
    #[serde(rename = "renderType")]
    pub render_type: String,
    #[serde(rename = "dataType")]
    pub data_type: String,
}

#[derive(Deserialize, Clone)]
pub struct DataBlockData {
    #[serde(rename = "$value")]
    pub text: String,
}

#[derive(Deserialize, Clone)]
pub struct SegmentSet {
    #[serde(rename = "RENDERDATASOURCE")]
    pub render_data_sources: Vec<RenderDataSource>,
}

#[derive(Deserialize, Clone)]
pub struct RenderDataSource {
    #[serde(rename = "RENDERINDEXSOURCE")]
    pub index_source: RenderIndexSource,
    #[serde(rename = "RENDERSTREAM")]
    pub streams: Vec<RenderStream>,
}

#[derive(Deserialize, Clone)]
pub struct RenderIndexSource {
    #[serde(rename = "primitive")]
    pub primitive: String,
    #[serde(rename = "count")]
    pub count: usize,
    #[serde(rename = "INDEXSOURCEDATA")]
    pub index_data: Option<IndexSourceData>,
    #[serde(rename = "format")]
    pub data_type: String,
}

#[derive(Deserialize, Clone)]
pub struct IndexSourceData {
    #[serde(rename = "$value")]
    pub text: String,
}

#[derive(Deserialize, Clone)]
pub struct RenderStream {
    #[serde(rename = "dataBlock")]
    pub data_block: String,
}

pub fn parse_xml_file(path: &Path) -> (Vec<DataBlock>, Vec<RenderDataSource>) {
    let pssg_file: PssgFile = {
        let file = File::open(path).expect("Failed to open file");
        let reader = BufReader::new(file);
        from_reader(reader).expect("Failed to parse XML")
    };

    let source_count = pssg_file
        .database
        .type_info
        .iter()
        .find(|t| t.type_name == "RENDERDATASOURCE")
        .unwrap_or(&TypeInfo {
            type_name: "RENDERDATASOURCE".to_string(),
            type_count: 0,
        })
        .type_count;

    let data_blocks = pssg_file
        .clone()
        .database
        .libraries
        .into_iter()
        .filter(|l| l.library_type == "RENDERINTERFACEBOUND")
        .flat_map(|l| l.data_blocks)
        .filter(|d| d.data.is_some())
        .collect();

    let render_index_sources = pssg_file
        .database
        .libraries
        .into_iter()
        .filter(|l| l.library_type == "SEGMENTSET")
        .flat_map(|l| l.segment_sets)
        .flat_map(|s| s.render_data_sources)
        .filter(|r| r.index_source.index_data.is_some())
        .collect::<Vec<_>>();

    if source_count != render_index_sources.len() {
        warn!(
            "Expected {} RENDERDATASOURCE entries, found {}",
            source_count,
            render_index_sources.len()
        );
    }

    (data_blocks, render_index_sources)
}
