use tracing::warn;

use crate::{
    error::ParseError,
    parse::read,
    structure::{DataBlock, RenderDataSource, RenderIndexSource},
};

type Vertices = Vec<[f32; 3]>;
type UVs = Vec<[f32; 2]>;
type Faces = Vec<[u32; 3]>;

pub struct Object {
    pub vertices: Vertices,
    pub uvs: UVs,
    pub faces: Faces,
}

impl Object {
    pub fn new(blocks: &[DataBlock], sources: RenderDataSource) -> Result<Self, ParseError> {
        let (vertices, uvs, faces) = Self::decode(blocks, sources)?;
        Ok(Self {
            vertices,
            uvs,
            faces,
        })
    }

    pub fn to_obj(&self) -> String {
        let mut obj = String::new();
        obj.push_str(&format!("# Vertices: {}\n", self.vertices.len()));
        for vertex in &self.vertices {
            obj.push_str(&format!("v {} {} {}\n", vertex[0], vertex[1], vertex[2]));
        }

        obj.push_str(&format!("\n# UVs: {}\n", self.uvs.len()));
        for uv in &self.uvs {
            obj.push_str(&format!("vt {} {}\n", uv[0], uv[1]));
        }

        obj.push_str(&format!("\n# Faces: {}\n", self.faces.len()));
        for face in &self.faces {
            obj.push_str(&format!(
                "f {} {} {}\n",
                face[0] + 1,
                face[1] + 1,
                face[2] + 1
            ));
        }
        obj
    }

    fn decode(
        blocks: &[DataBlock],
        source: RenderDataSource,
    ) -> Result<(Vertices, UVs, Faces), ParseError> {
        let mut vertices = Vec::new();
        let mut uvs = Vec::new();
        let mut faces = Vec::new();

        for render_stream in &source.streams {
            let block_id = render_stream.data_block.replace("#", "");
            if let Some(block) = blocks.iter().find(|b| b.id == block_id) {
                match block.stream.render_type.as_str() {
                    "Vertex" | "SkinnableVertex" => {
                        vertices.extend(Self::decode_vertices(block)?);
                    }
                    "ST" => {
                        uvs.extend(Self::decode_uvs(block)?);
                    }
                    _ => {}
                }
            }
        }

        faces.extend(Self::decode_faces(&source.index_source)?);

        Ok((vertices, uvs, faces))
    }

    fn decode_vertices(block: &DataBlock) -> Result<Vertices, ParseError> {
        let data: Vec<f32> = read(
            &block
                .data
                .as_ref()
                .ok_or_else(|| ParseError::new("Missing vertex block data"))?
                .text,
            "float",
        )?;

        let vertices: Vec<[f32; 3]> = data.chunks(3).map(|c| [c[0], c[1], c[2]]).collect();
        if block.element_count != vertices.len() {
            return Err(ParseError::new("Vertex count does not match element count"));
        }
        Ok(vertices)
    }

    fn decode_uvs(block: &DataBlock) -> Result<UVs, ParseError> {
        let data: Vec<f32> = read(
            &block
                .data
                .as_ref()
                .ok_or_else(|| ParseError::new("Missing uv block data"))?
                .text,
            "half",
        )?;

        let uvs: Vec<[f32; 2]> = if block.stream.data_type == "half2" {
            data.chunks(2).map(|c| [c[0], c[1]]).collect()
        } else {
            data.chunks(4).map(|c| [c[0], c[1]]).collect()
        };

        if block.element_count != uvs.len() {
            return Err(ParseError::new(&format!(
                "UV count {} does not match element count {}",
                uvs.len(),
                block.element_count
            )));
        }
        Ok(uvs)
    }

    fn decode_faces(source: &RenderIndexSource) -> Result<Faces, ParseError> {
        let data = read(
            &source
                .index_data
                .as_ref()
                .ok_or_else(|| ParseError::new("Missing faces block data"))?
                .text,
            &source.data_type,
        )?;
        let mut faces = Vec::new();

        for i in (0..data.len()).step_by(3) {
            faces.push([data[i], data[i + 1], data[i + 2]]);
        }
        if source.count % 3 != 0 {
            warn!(
                "{}",
                format!("Face count ({}) is not a multiple of 3", source.count)
            );
        }
        if source.count / 3 != faces.len() {
            warn!(
                "{}",
                format!(
                    "Face count ({}) does not match face data ({})",
                    source.count / 3,
                    faces.len()
                )
            );
        }
        Ok(faces)
    }
}
