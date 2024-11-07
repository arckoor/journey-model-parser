use crate::{
    error::ParseError,
    object::Object,
    structure::{DataBlock, RenderDataSource, RootNode},
};

type Translation = [f32; 3];

pub struct Entity {
    pub objects: Vec<Object>,
    pub translation: Translation,
}

impl Entity {
    pub fn new(
        data_blocks: Vec<DataBlock>,
        render_data_sources: Vec<RenderDataSource>,
        root_node: Option<RootNode>,
    ) -> Result<Self, ParseError> {
        let mut objects = Vec::new();
        for source in render_data_sources {
            objects.push(Object::new(&data_blocks, source)?);
        }

        let translation = Self::get_translation(&root_node)?;

        Ok(Self {
            objects,
            translation,
        })
    }

    fn get_translation(root_node: &Option<RootNode>) -> Result<Translation, ParseError> {
        if let Some(root_node) = root_node {
            if let Some(render_node) = root_node.render_nodes.first() {
                let transform = &render_node.transform;
                let translation = [transform[3][0], transform[3][1], transform[3][2]];
                return Ok(translation);
            }
        }
        Ok([0.0, 0.0, 0.0])
    }
}
