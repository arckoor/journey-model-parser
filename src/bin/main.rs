use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use clap::Parser;

use journey_model_parser::{error::ParseError, initialize_tracing, parse, Options};

#[derive(Parser)]
struct Args {
    /// Output as .json
    #[arg(long, conflicts_with = "obj")]
    json: bool,

    /// Output as .obj
    #[arg(long, default_value_t = false)]
    obj: bool,

    /// Pretty-format the file
    #[arg(
        long,
        requires = "json",
        conflicts_with = "obj",
        default_value_t = false
    )]
    pretty: bool,

    /// Enable parsing the armature, if there is one
    #[arg(
        long,
        requires = "json",
        conflicts_with = "obj",
        default_value_t = false
    )]
    armature: bool,

    /// The file to convert
    #[arg(required = true)]
    file: PathBuf,
}

#[derive(Debug)]
enum FileType {
    Obj,
    Json(bool),
}

fn convert_file(args: Args) -> Result<(), ParseError> {
    let options = Options {
        xml_root: args
            .file
            .parent()
            .ok_or(ParseError::new("Failed to get file path"))?
            .to_path_buf(),
        xml_file: args
            .file
            .file_name()
            .ok_or(ParseError::new("Failed to get file path"))?
            .to_str()
            .ok_or(ParseError::new("Failed to get file path"))?
            .to_string(),
        armature: args.armature,
        armature_full: false,
    };

    let file_type = if args.obj {
        FileType::Obj
    } else {
        FileType::Json(args.pretty)
    };

    let entity = parse(&options)?;

    match file_type {
        FileType::Json(pretty) => {
            let obj_path = options.xml_root.join(format!("{}.json", options.xml_file));
            let mut file =
                File::create(obj_path).map_err(|_| ParseError::new("Failed to create file"))?;
            file.write_all(entity.to_json(pretty).as_bytes())
                .map_err(|_| ParseError::new("Failed to write file"))?;
        }
        FileType::Obj => {
            let objects = entity.to_objs();

            for (i, object) in objects.iter().enumerate() {
                let obj_file = if objects.len() == 1 {
                    format!("{}.obj", options.xml_file)
                } else {
                    format!("{}-{}.obj", options.xml_file, i + 1)
                };
                let obj_path = options.xml_root.join(obj_file);
                let mut file =
                    File::create(obj_path).map_err(|_| ParseError::new("Failed to create file"))?;
                file.write_all(object.as_bytes())
                    .map_err(|_| ParseError::new("Failed to write file"))?;
            }
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize_tracing();
    let args = Args::parse();

    convert_file(args)?;
    Ok(())
}
