use tracing::warn;

use crate::{
    error::ParseError,
    structure::{DataBlock, RenderDataSource, RenderIndexSource, Transform},
};

type Vertices = Vec<[f32; 3]>;
type UVs = Vec<[f32; 2]>;
type Faces = Vec<[u64; 3]>;
type Translation = [f32; 3];

pub struct ParsedObject {
    vertices: Vertices,
    uvs: UVs,
    faces: Faces,
    translation: Translation,
}

impl ParsedObject {
    pub fn new(
        blocks: &[DataBlock],
        sources: RenderDataSource,
        transform: &Transform,
    ) -> Result<Self, ParseError> {
        let (vertices, uvs, faces) = Self::decode(blocks, sources)?;
        let translation = Self::decode_translation(transform)?;
        Ok(Self {
            vertices,
            uvs,
            faces,
            translation,
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

    pub fn to_ffi(&self) -> (Vec<f32>, Vec<f32>, Vec<u64>, Vec<f32>) {
        (
            self.vertices.iter().copied().flatten().collect(),
            self.uvs.iter().copied().flatten().collect(),
            self.faces.iter().copied().flatten().collect(),
            self.translation.to_vec(),
        )
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
        let data = Self::read(
            &block
                .data
                .as_ref()
                .ok_or_else(|| ParseError::new("Missing vertex block data"))?
                .text,
        )?;
        let mut vertices = Vec::new();
        let mut i = 0;
        while i < data.len() {
            let x = Self::bytes_to_f32(&data[i..i + 4]);
            let y = Self::bytes_to_f32(&data[i + 4..i + 8]);
            let z = Self::bytes_to_f32(&data[i + 8..i + 12]);
            vertices.push([x, y, z]);
            i += 12;
        }
        if block.element_count != vertices.len() {
            return Err(ParseError::new("Vertex count does not match element count"));
        }
        Ok(vertices)
    }

    fn decode_uvs(block: &DataBlock) -> Result<UVs, ParseError> {
        let mut uvs = Vec::new();
        let data = Self::read(
            &block
                .data
                .as_ref()
                .ok_or_else(|| ParseError::new("Missing uv block data"))?
                .text,
        )?;
        let mut i = 0;
        while i < data.len() {
            let u = Self::half_to_f32(u16::from_be_bytes([data[i] as u8, data[i + 1] as u8]));
            let v = Self::half_to_f32(u16::from_be_bytes([data[i + 2] as u8, data[i + 3] as u8]));
            uvs.push([u, v]);
            if block.stream.data_type == "half2" {
                i += 4;
            } else if block.stream.data_type == "half4" {
                i += 8;
            }
        }
        if block.element_count != uvs.len() {
            return Err(ParseError::new("UV count does not match element count"));
        }
        Ok(uvs)
    }

    fn decode_faces(source: &RenderIndexSource) -> Result<Faces, ParseError> {
        if !["uchar", "ushort"].contains(&source.data_type.as_str()) {
            warn!("Unknown data type {} for faces", source.data_type);
        }
        let data = Self::read(
            &source
                .index_data
                .as_ref()
                .ok_or_else(|| ParseError::new("Missing faces block data"))?
                .text,
        )?;
        let mut faces = Vec::new();

        for i in (0..data.len()).step_by(3) {
            faces.push([data[i] as u64, data[i + 1] as u64, data[i + 2] as u64]);
        }
        if source.count % 3 != 0 {
            warn!("Face count is not a multiple of 3");
        }
        if source.count / 3 != faces.len() {
            warn!("Face count does not match face data");
        }
        Ok(faces)
    }

    fn decode_translation(transform: &Transform) -> Result<Translation, ParseError> {
        let data = transform
            .matrix
            .split_whitespace()
            .map(|s| {
                s.parse::<f32>()
                    .map_err(|e| ParseError::new(&format!("Failed to decode matrix: {e}")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if data.len() != 16 {
            return Err(ParseError::new("Invalid matrix length"));
        }
        let x = data[12];
        let y = data[13];
        let z = data[14];
        Ok([x, y, z])
    }

    fn read(data: &str) -> Result<Vec<u32>, ParseError> {
        let v = if data
            .chars()
            .any(|c| !c.is_ascii_digit() && !c.is_whitespace())
        {
            data.split_whitespace()
                .flat_map(|s| {
                    (0..s.len()).step_by(2).map(|i| {
                        u8::from_str_radix(&s[i..i + 2], 16)
                            .map_err(|e| ParseError::new(&format!("Failed to decode hex: {e}")))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|v| v as u32)
                .collect()
        } else {
            data.split_whitespace()
                .map(|s| {
                    s.parse::<u32>()
                        .map_err(|e| ParseError::new(&format!("Failed to decode decimal: {e}")))
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        Ok(v)
    }

    fn half_to_f32(half: u16) -> f32 {
        let sign = (half >> 15) & 0x1;
        let exponent = (half >> 10) & 0x1f;
        let mantissa = half & 0x3ff;

        if exponent == 0 && mantissa == 0 {
            return 0.0;
        }

        let exp = (exponent as i32) - 15 + 127;
        let bits = ((sign as u32) << 31) | ((exp as u32) << 23) | ((mantissa as u32) << 13);

        f32::from_bits(bits)
    }

    fn bytes_to_f32(bytes: &[u32]) -> f32 {
        f32::from_be_bytes([
            bytes[0] as u8,
            bytes[1] as u8,
            bytes[2] as u8,
            bytes[3] as u8,
        ])
    }
}
