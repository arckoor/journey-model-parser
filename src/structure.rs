use std::io::BufReader;
use std::path::Path;
use std::{fmt, fs::File};

use serde::de::{self, MapAccess, Unexpected};
use serde::Serialize;
use serde::{de::Visitor, Deserialize, Deserializer};
use serde_xml_rs::from_reader;
use tracing::warn;

use crate::error::ParseError;
use crate::parse::read;

#[derive(Deserialize, Clone, Debug)]
#[serde(rename = "PSSGFILE")]
pub struct PssgFile {
    #[serde(rename = "PSSGDATABASE")]
    pub database: PssgDatabase,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PssgDatabase {
    #[serde(rename = "TYPEINFO", default)]
    pub type_info: Vec<TypeInfo>,
    #[serde(rename = "LIBRARY", default)]
    pub libraries: Vec<Library>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TypeInfo {
    #[serde(rename = "typeName")]
    pub type_name: String,
    #[serde(rename = "typeCount")]
    pub type_count: usize,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Library {
    #[serde(rename = "type")]
    pub library_type: String,
    #[serde(rename = "DATABLOCK", default)]
    pub data_blocks: Vec<DataBlock>,
    #[serde(rename = "SEGMENTSET", default)]
    pub segment_sets: Vec<SegmentSet>,
    #[serde(rename = "ANIMATIONCHANNEL", default)]
    pub animation_channels: Vec<AnimationChannel>,
    #[serde(rename = "ANIMATION", default)]
    pub animations: Vec<Animation>,
    #[serde(rename = "ANIMATIONCHANNELDATABLOCK", default)]
    pub animation_channel_data_blocks: Vec<AnimDataBlock>,
    #[serde(rename = "ROOTNODE")]
    pub root_node: Option<Node>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct DataBlock {
    #[serde(rename = "elementCount")]
    pub element_count: usize,
    #[serde(rename = "DATABLOCKSTREAM")]
    pub stream: DataBlockStream,
    #[serde(rename = "DATABLOCKDATA")]
    pub data: Option<DataBlockData>,
    pub id: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct DataBlockStream {
    #[serde(rename = "renderType")]
    pub render_type: String,
    #[serde(rename = "dataType")]
    pub data_type: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct DataBlockData {
    #[serde(rename = "$value")]
    pub text: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct SegmentSet {
    #[serde(rename = "RENDERDATASOURCE", default)]
    pub render_data_sources: Vec<RenderDataSource>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RenderDataSource {
    #[serde(rename = "RENDERINDEXSOURCE")]
    pub index_source: RenderIndexSource,
    #[serde(rename = "RENDERSTREAM", default)]
    pub streams: Vec<RenderStream>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RenderIndexSource {
    pub primitive: String,
    pub count: usize,
    #[serde(rename = "INDEXSOURCEDATA")]
    pub index_data: Option<IndexSourceData>,
    #[serde(rename = "format")]
    pub data_type: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct IndexSourceData {
    #[serde(rename = "$value")]
    pub text: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RenderStream {
    #[serde(rename = "dataBlock")]
    pub data_block: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct AnimationChannel {
    #[serde(rename = "timeBlock")]
    pub time_block: String,
    #[serde(rename = "valueBlock")]
    pub value_block: String,
    pub id: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Animation {
    #[serde(rename = "constantChannelStartTime", default)]
    pub constant_channel_start_time: f32,
    #[serde(rename = "constantChannelEndTime", default)]
    pub constant_channel_end_time: f32,
    #[serde(rename = "CHANNELREF", default)]
    pub channel_refs: Vec<ChannelRef>,
    #[serde(rename = "CONSTANTCHANNEL", default)]
    pub constant_channels: Vec<ConstantChannel>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ChannelRef {
    pub channel: String,
    #[serde(rename = "targetName")]
    pub target_name: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ConstantChannel {
    pub value: String,
    #[serde(rename = "targetName")]
    pub target_name: String,
    #[serde(rename = "keyType")]
    pub key_type: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct AnimDataBlock {
    #[serde(rename = "keyCount")]
    pub key_count: usize,
    #[serde(rename = "keyType")]
    pub key_type: String,
    pub id: String,
    #[serde(rename = "KEYS")]
    pub keys: String,
}

#[derive(Deserialize, Clone, Debug)]
pub enum NodeKind {
    Root(Node),
    Render(Node),
    Joint(Node),
    Node(Node),
}

#[derive(Clone, Debug)]
pub struct Node {
    pub transform: Transform,
    pub nodes: Vec<NodeKind>,
    pub id: String,
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NodeVisitor;

        impl<'de> Visitor<'de> for NodeVisitor {
            type Value = Node;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a node")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut transform = None;
                let mut id = None;
                let mut nodes = Vec::new();
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "TRANSFORM" => {
                            transform = Some(map.next_value()?);
                        }
                        "ROOTNODE" | "RENDERNODE" | "JOINTNODE" | "NODE" => {
                            let node = map.next_value::<Node>()?;
                            match key.as_str() {
                                "ROOTNODE" => nodes.push(NodeKind::Root(node)),
                                "RENDERNODE" => nodes.push(NodeKind::Render(node)),
                                "JOINTNODE" => nodes.push(NodeKind::Joint(node)),
                                "NODE" => nodes.push(NodeKind::Node(node)),
                                _ => unreachable!(),
                            }
                        }
                        "id" => {
                            id = Some(map.next_value()?);
                        }
                        _ => {
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                let transform = transform.ok_or_else(|| de::Error::missing_field("TRANSFORM"))?;
                let id = id.ok_or_else(|| de::Error::missing_field("id"))?;
                Ok(Node {
                    transform,
                    nodes,
                    id,
                })
            }
        }

        deserializer.deserialize_map(NodeVisitor)
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct Transform {
    matrix: [[f32; 4]; 4],
}

pub struct ModelData {
    pub data_blocks: Vec<DataBlock>,
    pub render_index_sources: Vec<RenderDataSource>,
    pub root_node: Option<NodeKind>,
    pub anim_channels: Vec<AnimationChannel>,
    pub animations: Vec<Animation>,
    pub anim_data_blocks: Vec<AnimDataBlock>,
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

impl Default for Transform {
    fn default() -> Self {
        Self {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
}

impl std::ops::Index<usize> for Transform {
    type Output = [f32; 4];

    fn index(&self, index: usize) -> &[f32; 4] {
        &self.matrix[index]
    }
}

pub fn parse_xml_file(path: &Path) -> Result<ModelData, ParseError> {
    let pssg_file: PssgFile = {
        let file = File::open(path).expect("Failed to open file");
        let reader = BufReader::new(file);
        from_reader(reader).map_err(|e| ParseError::new(&format!("{:?}", e)))?
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

    let anim_channels = pssg_file
        .clone()
        .database
        .libraries
        .into_iter()
        .filter(|l| l.library_type == "ANIMATIONCHANNEL")
        .flat_map(|l| l.animation_channels)
        .collect::<Vec<_>>();

    let animations = pssg_file
        .clone()
        .database
        .libraries
        .into_iter()
        .filter(|l| l.library_type == "ANIMATION")
        .flat_map(|l| l.animations)
        .collect::<Vec<_>>();

    let anim_data_blocks = pssg_file
        .clone()
        .database
        .libraries
        .into_iter()
        .filter(|l| l.library_type == "ANIMATIONCHANNELDATABLOCK")
        .flat_map(|l| l.animation_channel_data_blocks)
        .collect::<Vec<_>>();

    let root_node = pssg_file
        .database
        .libraries
        .into_iter()
        .find_map(|l| l.root_node.map(NodeKind::Root));

    if source_count != render_index_sources.len() {
        warn!(
            "Expected {} RENDERDATASOURCE entries, found {}",
            source_count,
            render_index_sources.len()
        );
    }

    Ok(ModelData {
        data_blocks,
        render_index_sources,
        root_node,
        anim_channels,
        animations,
        anim_data_blocks,
    })
}
