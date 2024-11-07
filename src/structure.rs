use std::io::BufReader;
use std::path::Path;
use std::{fmt, fs::File};

use serde::de::{self, MapAccess, Unexpected};
use serde::{de::Visitor, Deserialize, Deserializer};
use serde_xml_rs::from_reader;
use tracing::warn;

use crate::parse::read;

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
    #[serde(rename = "ROOTNODE", default)]
    pub root_node: Option<RootNode>,
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

#[derive(Clone)]
pub struct RootNode {
    pub render_nodes: Vec<RenderNode>,
}

impl<'de> Deserialize<'de> for RootNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RootNodeVisitor;

        impl<'de> Visitor<'de> for RootNodeVisitor {
            type Value = RootNode;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct RootNode")
            }

            fn visit_map<V>(self, mut map: V) -> Result<RootNode, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut render_nodes = Vec::new();

                while let Some(key) = map.next_key::<String>()? {
                    if key == "RENDERNODE" {
                        let node: RenderNode = map.next_value()?;
                        render_nodes.push(node);
                    } else {
                        let _: serde::de::IgnoredAny = map.next_value()?;
                    }
                }

                Ok(RootNode { render_nodes })
            }
        }

        deserializer.deserialize_map(RootNodeVisitor)
    }
}

#[derive(Deserialize, Clone)]
pub struct RenderNode {
    #[serde(rename = "TRANSFORM")]
    pub transform: Transform,
}

#[derive(Clone)]
pub struct Transform {
    matrix: [[f32; 4]; 4],
}

impl<'de> Deserialize<'de> for Transform {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        let data: Vec<f32> = read(&s, "float").map_err(|_| {
            de::Error::invalid_value(Unexpected::Str(&s), &"valid matrix: invalid f32")
        })?;
        if data.len() != 16 {
            return Err(de::Error::invalid_value(
                Unexpected::Str(&s),
                &"valid matrix: invalid length",
            ));
        }

        let matrix: [[f32; 4]; 4] = data
            .chunks_exact(4)
            .map(|chunk| {
                chunk
                    .try_into()
                    .map_err(|_| de::Error::invalid_value(Unexpected::Str(&s), &"valid matrix"))
            })
            .collect::<Result<Vec<[f32; 4]>, _>>()?
            .try_into()
            .map_err(|_| de::Error::invalid_value(Unexpected::Str(&s), &"valid matrix"))?;

        Ok(Transform { matrix })
    }
}

impl std::ops::Index<usize> for Transform {
    type Output = [f32; 4];

    fn index(&self, index: usize) -> &[f32; 4] {
        &self.matrix[index]
    }
}

pub fn parse_xml_file(path: &Path) -> (Vec<DataBlock>, Vec<RenderDataSource>, Option<RootNode>) {
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
        .clone()
        .database
        .libraries
        .into_iter()
        .filter(|l| l.library_type == "SEGMENTSET")
        .flat_map(|l| l.segment_sets)
        .flat_map(|s| s.render_data_sources)
        .filter(|r| r.index_source.index_data.is_some())
        .collect::<Vec<_>>();

    let root_node = pssg_file
        .database
        .libraries
        .into_iter()
        .find_map(|l| l.root_node);

    if source_count != render_index_sources.len() {
        warn!(
            "Expected {} RENDERDATASOURCE entries, found {}",
            source_count,
            render_index_sources.len()
        );
    }

    (data_blocks, render_index_sources, root_node)
}
