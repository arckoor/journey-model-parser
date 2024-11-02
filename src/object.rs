use tracing::warn;

use crate::{
    error::ParseError,
    structure::{DataBlock, RenderDataSource, RenderIndexSource, Transform},
};

type Vertices = Vec<[f32; 3]>;
type UVs = Vec<[f32; 2]>;
type Faces = Vec<[u32; 3]>;
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

    pub fn to_ffi(&self) -> (Vec<f32>, Vec<f32>, Vec<u32>, Vec<f32>) {
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
        let data: Vec<f32> = Self::read(
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
        let data: Vec<f32> = Self::read(
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
        let data = Self::read(
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

    fn decode_translation(transform: &Transform) -> Result<Translation, ParseError> {
        let data: Vec<f32> = Self::read(&transform.matrix, "float")?;
        if data.len() != 16 {
            return Err(ParseError::new("Invalid matrix length"));
        }
        let x = data[12];
        let y = data[13];
        let z = data[14];
        Ok([x, y, z])
    }

    fn read<T>(data: &str, data_type: &str) -> Result<Vec<T>, ParseError>
    where
        T: From<u8> + From<u16> + std::str::FromStr,
        <T as std::str::FromStr>::Err: std::fmt::Display,
    {
        if data
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c.is_whitespace())
            && data.split_whitespace().all(|s| {
                s.len() == 2
                    && !((s.len() > 1
                        && s.starts_with("0")
                        && s.chars().all(|c| c.is_ascii_digit()))
                        && (data.contains("e+") || data.contains("e-")))
            })
        {
            return Self::read_hex(data, data_type);
        }
        Self::read_decimal(data)
    }

    fn read_hex<T>(data: &str, data_type: &str) -> Result<Vec<T>, ParseError>
    where
        T: From<u8> + From<u16> + std::str::FromStr,
        <T as std::str::FromStr>::Err: std::fmt::Display,
    {
        let stride = match data_type {
            "uchar" => 1,
            "ushort" | "half" => 2,
            "float" => 4,
            _ => return Err(ParseError::new(&format!("Unknown data type {}", data_type))),
        };

        let bytes = data
            .split_whitespace()
            .map(|s| {
                u8::from_str_radix(s, 16)
                    .map_err(|e| ParseError::new(&format!("Failed to decode hex: {e}")))
            })
            .collect::<Result<Vec<_>, _>>()?;

        bytes
            .chunks(stride)
            .map(|chunk| match (stride, data_type) {
                (1, "uchar") => Ok(T::from(chunk[0])),
                (2, "ushort") => {
                    let arr = <[u8; 2]>::try_from(chunk)
                        .map_err(|_| ParseError::new("Invalid length for u16"))?;
                    Ok(T::from(u16::from_be_bytes(arr)))
                }
                (2, "half") => {
                    let arr = <[u8; 2]>::try_from(chunk)
                        .map_err(|_| ParseError::new("Invalid length for u16"))?;

                    let f = Self::half_to_f32(u16::from_be_bytes(arr));
                    f.to_string()
                        .parse::<T>()
                        .map_err(|_| ParseError::new("Failed to convert half to target type"))
                }
                (4, "float") => {
                    let arr = <[u8; 4]>::try_from(chunk)
                        .map_err(|_| ParseError::new("Invalid length for f32"))?;
                    let f = f32::from_be_bytes(arr);
                    f.to_string()
                        .parse::<T>()
                        .map_err(|_| ParseError::new("Failed to convert f32 to target type"))
                }
                _ => unreachable!(),
            })
            .collect()
    }

    fn read_decimal<T>(data: &str) -> Result<Vec<T>, ParseError>
    where
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::fmt::Display,
    {
        data.split_whitespace()
            .map(|s| {
                s.parse::<T>()
                    .map_err(|e| ParseError::new(&format!("Failed to decode decimal data: {e}")))
            })
            .collect::<Result<Vec<_>, _>>()
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
}
