use serde::Serialize;
use tracing::warn;

use crate::{
    error::ParseError,
    parse::read,
    structure::{self, AnimDataBlock, AnimationChannel},
};

pub type TranslationOrScale = [f32; 3];
/// WXYZ quaternion
pub type Quaternion = [f32; 4];

#[derive(Serialize, Debug)]
pub struct NodeAnimation {
    pub target: String,
    pub time: Vec<f32>,
    pub translation: Option<Vec<TranslationOrScale>>,
    pub rotation: Option<Vec<Quaternion>>,
    pub scale: Option<Vec<TranslationOrScale>>,
}

#[derive(Serialize, Debug)]
pub struct ConstantNodeAnimation {
    pub target: String,
    pub translation: Option<TranslationOrScale>,
    pub rotation: Option<Quaternion>,
    pub scale: Option<TranslationOrScale>,
}

#[derive(Serialize, Debug)]
pub struct Animation {
    pub name: String,
    pub node_animations: Vec<NodeAnimation>,
    pub constant_node_animations: Vec<ConstantNodeAnimation>,
    pub constant_start_time: f32,
    pub constant_end_time: f32,
}

impl Animation {
    pub fn new(
        channels: &[AnimationChannel],
        anim: structure::Animation,
        data_blocks: &[AnimDataBlock],
        name: String,
    ) -> Result<Self, ParseError> {
        let mut node_animations = Vec::new();
        let mut constant_node_animations = Vec::new();

        for channel in channels {
            let v_block = data_blocks
                .iter()
                .find(|block| block.id == channel.value_block.replace("#", ""));
            let t_block = data_blocks
                .iter()
                .find(|block| block.id == channel.time_block.replace("#", ""));
            let mut time = None;
            let mut translation = None;
            let mut rotation = None;
            let mut scale = None;
            if let (Some(value_block), Some(time_block)) = (v_block, t_block) {
                match value_block.key_type.as_str() {
                    // "Translation" => {
                    //     translation = Some(Self::decode_translation_scale(value_block)?);
                    // }
                    "Rotation" => {
                        rotation = Some(Self::decode_rotation(value_block)?);
                    }
                    "Scale" => {
                        scale = Some(Self::decode_translation_scale(value_block)?);
                    }
                    _ => {} // unreachable!(),
                }
                if time.is_none() && time_block.key_type == "Time" {
                    time = Some(Self::decode_time(time_block)?)
                }
            } else {
                warn!("No data block found for channel {:?}", channel.id);
            }

            if time.is_none() {
                warn!("No time data found for channel {:?}", channel.id);
            } else {
                if translation.is_none() && rotation.is_none() && scale.is_none() {
                    continue;
                }

                let target = Self::find_target(channel.id.clone(), &anim);

                node_animations.push(NodeAnimation {
                    target,
                    time: time.unwrap(),
                    translation,
                    rotation,
                    scale,
                });
            }
        }

        for channel in &anim.constant_channels {
            let mut translation = None;
            let mut rotation = None;
            let mut scale = None;
            let value = read(&channel.value, "float")?;
            match channel.key_type.as_str() {
                // "Translation" => {
                //     translation = Some(Self::get_translation_or_scale(&value));
                // }
                // "Rotation" => {
                //     rotation = Some(Self::get_quaternion(&value));
                // }
                "Scale" => {
                    scale = Some(Self::get_translation_or_scale(&value));
                }
                _ => {} //unreachable!(),
            }

            if translation.is_none() && rotation.is_none() && scale.is_none() {
                continue;
            }

            let target = Self::find_target(channel.target_name.clone(), &anim);
            constant_node_animations.push(ConstantNodeAnimation {
                target,
                translation,
                rotation,
                scale,
            })
        }

        Ok(Self {
            name,
            node_animations,
            constant_node_animations,
            constant_start_time: anim.constant_channel_start_time,
            constant_end_time: anim.constant_channel_end_time,
        })
    }

    fn find_target(target: String, anim: &structure::Animation) -> String {
        let mut target = target;
        if let Some(ref_) = anim
            .channel_refs
            .iter()
            .find(|r| r.channel.replace("#", "") == target)
        {
            target = ref_.target_name.clone();
        }

        target
    }

    fn decode_translation_scale(
        data_block: &AnimDataBlock,
    ) -> Result<Vec<TranslationOrScale>, ParseError> {
        let data = read(&data_block.keys, "float")?;
        let rotations = data
            .chunks(3)
            .map(Self::get_translation_or_scale)
            .collect::<Vec<_>>();

        if data_block.key_count != rotations.len() {
            return Err(ParseError::new(&format!(
                "Translation / Scale count {} does not match key count {}",
                rotations.len(),
                data_block.key_count
            )));
        }

        Ok(rotations)
    }

    fn decode_rotation(data_block: &AnimDataBlock) -> Result<Vec<Quaternion>, ParseError> {
        let data = read(&data_block.keys, "float")?;
        let rotations = data.chunks(4).map(Self::get_quaternion).collect::<Vec<_>>();

        if data_block.key_count != rotations.len() {
            return Err(ParseError::new(&format!(
                "Rotation count {} does not match key count {}",
                rotations.len(),
                data_block.key_count
            )));
        }

        Ok(rotations)
    }

    fn decode_time(data_block: &AnimDataBlock) -> Result<Vec<f32>, ParseError> {
        let data = read(&data_block.keys, "float")?;
        if data_block.key_count != data.len() {
            warn!(
                "Time count {} does not match key count {}",
                data.len(),
                data_block.key_count
            );
        }

        Ok(data)
    }

    fn get_translation_or_scale(c: &[f32]) -> TranslationOrScale {
        [c[0], c[1], c[2]]
    }

    fn get_quaternion(c: &[f32]) -> Quaternion {
        [c[3], c[0], c[1], c[2]]
    }
}
