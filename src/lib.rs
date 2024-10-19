mod ffi;
mod structure;

use std::io::Write;
use std::path::Path;
use std::{fs::File, path::PathBuf};

use structure::{DataBlock, RenderIndexSource};
use tracing::warn;

type Vertices = Vec<[f32; 3]>;
type UVs = Vec<[f32; 2]>;
type Faces = Vec<[u64; 3]>;

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

fn decode_vertices(blocks: &Vec<DataBlock>) -> Vertices {
    let mut vertices = Vec::new();
    for block in blocks {
        if block.stream.render_type == "Vertex" {
            let data = decode(&block.data.text);
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

fn decode_uvs(blocks: &Vec<DataBlock>) -> UVs {
    let mut uvs = Vec::new();
    let blocks = blocks
        .iter()
        .filter(|b| b.stream.render_type == "ST")
        .collect::<Vec<_>>();
    let block = blocks[blocks.len() - 1];

    let data = decode(&block.data.text);
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

fn decode_faces(sources: &Vec<RenderIndexSource>) -> Faces {
    let mut faces = Vec::new();
    for block in sources.iter().filter(|b| b.primitive == "triangles") {
        let data = decode(&block.index_data.text);
        let mut block_faces = Vec::new();
        // fixme why is this the same
        if block.data_type == "ushort" {
            for i in (0..data.len()).step_by(3) {
                block_faces.push([data[i] as u64, data[i + 1] as u64, data[i + 2] as u64]);
            }
        } else if block.data_type == "uchar" {
            for i in (0..data.len()).step_by(3) {
                block_faces.push([data[i] as u64, data[i + 1] as u64, data[i + 2] as u64]);
            }
        }
        if block.count % 3 != 0 {
            warn!("Face count is not a multiple of 3");
        }
        if block.count / 3 != block_faces.len() {
            warn!("Face count does not match face data");
        }
        faces.extend_from_slice(&block_faces);
    }
    faces
}

fn write_obj(vertices: &Vertices, uvs: &UVs, faces: &Faces, mut path: PathBuf) {
    path.set_extension("obj");
    let mut file = File::create(path).expect("Failed to create file");

    writeln!(file, "{}", format!("# Vertices {}", vertices.len())).unwrap();
    for vertex in vertices {
        writeln!(
            file,
            "v  {:.4} {:.4} {:.4}",
            vertex[0], vertex[1], vertex[2]
        )
        .unwrap();
    }

    writeln!(file, "{}", format!("\n# UVs {}", uvs.len())).unwrap();
    for uv in uvs {
        writeln!(file, "vt  {:.4} {:.4}", uv[0], uv[1]).unwrap();
    }

    writeln!(file, "{}", format!("\n# Faces {}", faces.len())).unwrap();
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

pub fn parse(xml_file: &Path) -> (Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<[u64; 3]>) {
    let (data_blocks, render_index_sources) = structure::parse_xml_file(xml_file);

    let vertices = decode_vertices(&data_blocks);
    let uvs = decode_uvs(&data_blocks);
    let faces = decode_faces(&render_index_sources);

    (vertices, uvs, faces)
}

pub fn convert_file(xml_file: &Path) {
    let (vertices, uvs, faces) = parse(xml_file);

    write_obj(&vertices, &uvs, &faces, xml_file.to_path_buf());
}
