mod ffi;
mod structure;

use std::io::Write;
use std::path::Path;
use std::sync::Once;
use std::{fs::File, path::PathBuf};

use tracing::{info, warn};

use structure::{DataBlock, RenderDataSource};

type Vertices = Vec<[f32; 3]>;
type UVs = Vec<[f32; 2]>;
type Faces = Vec<[u64; 3]>;

static INIT: Once = Once::new();

fn initialize_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    });
}

fn decode(data: &str) -> Vec<u32> {
    if data
        .chars()
        .any(|c| !c.is_ascii_digit() && !c.is_whitespace())
    {
        data.split_whitespace()
            .flat_map(|s| {
                (0..s.len()).step_by(2).map(|i| {
                    u8::from_str_radix(&s[i..i + 2], 16).expect("Failed to decode hex") as u32
                })
            })
            .collect()
    } else {
        data.split_whitespace()
            .map(|s| s.parse::<u32>().expect("Failed to parse decimal"))
            .collect()
    }
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

fn decode_vertices(blocks: &[DataBlock]) -> Vertices {
    let mut vertices = Vec::new();
    for block in blocks {
        if block.stream.render_type == "Vertex" {
            let data = decode(&block.data.as_ref().unwrap().text);
            let mut block_vertices = Vec::new();
            let mut i = 0;
            while i < data.len() {
                let x = f32::from_be_bytes([
                    data[i] as u8,
                    data[i + 1] as u8,
                    data[i + 2] as u8,
                    data[i + 3] as u8,
                ]);
                let y = f32::from_be_bytes([
                    data[i + 4] as u8,
                    data[i + 5] as u8,
                    data[i + 6] as u8,
                    data[i + 7] as u8,
                ]);
                let z = f32::from_be_bytes([
                    data[i + 8] as u8,
                    data[i + 9] as u8,
                    data[i + 10] as u8,
                    data[i + 11] as u8,
                ]);
                block_vertices.push([x, y, z]);
                i += 12;
            }
            assert_eq!(block.element_count, block_vertices.len());
            vertices.extend_from_slice(&block_vertices);
        }
    }
    vertices
}

fn decode_uvs(block: &DataBlock) -> UVs {
    let mut uvs = Vec::new();
    let data = decode(&block.data.as_ref().unwrap().text);
    let mut i = 0;
    while i < data.len() {
        let u = half_to_f32(u16::from_be_bytes([data[i] as u8, data[i + 1] as u8]));
        let v = half_to_f32(u16::from_be_bytes([data[i + 2] as u8, data[i + 3] as u8]));
        uvs.push([u, v]);
        if block.stream.data_type == "half2" {
            i += 4;
        } else if block.stream.data_type == "half4" {
            i += 8;
        }
    }
    assert_eq!(block.element_count, uvs.len());
    uvs
}

fn decode_faces(sources: &[RenderDataSource], blocks: &[DataBlock]) -> (Faces, UVs) {
    let mut faces = Vec::new();
    let mut uvs = Vec::new();
    let uv_blocks: Vec<&DataBlock> = blocks
        .iter()
        .filter(|b| b.stream.render_type == "ST")
        .collect();
    for source in sources
        .iter()
        .filter(|b| b.index_source.primitive == "triangles")
    {
        let index_source = &source.index_source;
        if !["uchar", "ushort"].contains(&index_source.data_type.as_str()) {
            warn!("Unknown data type {} for faces", index_source.data_type);
        }
        let data = decode(&index_source.index_data.as_ref().unwrap().text);
        let mut source_faces = Vec::new();

        for i in (0..data.len()).step_by(3) {
            source_faces.push([data[i] as u64, data[i + 1] as u64, data[i + 2] as u64]);
        }
        if index_source.count % 3 != 0 {
            warn!("Face count is not a multiple of 3");
        }
        if index_source.count / 3 != source_faces.len() {
            warn!("Face count does not match face data");
        }
        faces.extend_from_slice(&source_faces);
        for render_stream in &source.streams {
            if let Some(block) = uv_blocks
                .iter()
                .find(|b| b.id == render_stream.data_block.replace("#", ""))
            {
                uvs.extend_from_slice(&decode_uvs(block));
                break;
            }
        }
    }
    (faces, uvs)
}

fn write_obj(vertices: &Vertices, uvs: &UVs, faces: &Faces, mut path: PathBuf) {
    path.set_extension("obj");
    let mut file = File::create(path).expect("Failed to create file");

    writeln!(file, "# Vertices {}", vertices.len()).unwrap();
    for vertex in vertices {
        writeln!(
            file,
            "v  {:.4} {:.4} {:.4}",
            vertex[0], vertex[1], vertex[2]
        )
        .unwrap();
    }

    writeln!(file, "\n# UVs {}", uvs.len()).unwrap();
    for uv in uvs {
        writeln!(file, "vt  {:.4} {:.4}", uv[0], uv[1]).unwrap();
    }

    writeln!(file, "\n# Faces {}", faces.len()).unwrap();
    for face in faces {
        writeln!(
            file,
            "f  {}/{} {}/{} {}/{}",
            face[0] + 1,
            face[0] + 1,
            face[1] + 1,
            face[1] + 1,
            face[2] + 1,
            face[2] + 1,
        )
        .unwrap();
    }
}

pub fn parse(xml_file: &Path) -> (Vertices, UVs, Faces) {
    initialize_tracing();
    info!("Parsing file {:?}", xml_file);
    let (data_blocks, render_index_sources) = structure::parse_xml_file(xml_file);

    let vertices = decode_vertices(&data_blocks);
    let (faces, uvs) = decode_faces(&render_index_sources, &data_blocks);

    (vertices, uvs, faces)
}

pub fn convert_file(xml_file: &Path) {
    let (vertices, uvs, faces) = parse(xml_file);

    write_obj(&vertices, &uvs, &faces, xml_file.to_path_buf());
}
