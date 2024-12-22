use serde::Serialize;

use crate::{
    animation::Animation,
    armature::Armature,
    error::ParseError,
    object::Object,
    structure::{ModelData, NodeKind, Transform},
    Options,
};

#[derive(Serialize, Debug)]
pub struct Entity {
    pub objects: Vec<Object>,
    pub transform: Transform,
    pub armature: Option<Armature>,
    pub animations: Vec<Animation>,
}

impl Entity {
    pub fn new(options: &Options, model_data: ModelData) -> Result<Self, ParseError> {
        let mut objects = Vec::new();
        for source in model_data.render_index_sources {
            objects.push(Object::new(&model_data.data_blocks, source)?);
        }

        let transform = Self::get_transform(&model_data.root_node);

        let armature = if let Some(NodeKind::Root(root_node)) = model_data.root_node {
            Armature::new(options, &NodeKind::Root(root_node))?
        } else {
            None
        };

        let mut animations = Vec::new();
        for anim in model_data.animations {
            animations.push(Animation::new(
                &model_data.anim_channels,
                anim,
                &model_data.anim_data_blocks,
                "base".to_string(),
            )?);
        }

        Ok(Self {
            objects,
            transform,
            armature,
            animations,
        })
    }

    fn get_transform(root_node: &Option<NodeKind>) -> Transform {
        if let Some(NodeKind::Root(root_node)) = root_node {
            for node in &root_node.nodes {
                if let NodeKind::Render(render_node) = node {
                    return render_node.transform.clone();
                }
            }
        }
        Transform::default()
    }

    pub fn add_animations(
        &mut self,
        model_data: ModelData,
        name: String,
    ) -> Result<(), ParseError> {
        for anim in model_data.animations {
            self.animations.push(Animation::new(
                &model_data.anim_channels,
                anim,
                &model_data.anim_data_blocks,
                name.clone(),
            )?);
        }

        Ok(())
    }
}
