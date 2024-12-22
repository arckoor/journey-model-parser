use std::env;
use std::path::Path;

use journey_model_parser::{convert_file, Options};

fn main() {
    let args: Vec<String> = env::args().collect();
    let xml_full_path = Path::new(&args[1]);

    assert!(xml_full_path.exists(), "File does not exist");

    let options = Options {
        xml_root: xml_full_path.parent().unwrap().to_path_buf(),
        xml_file: xml_full_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string(),
        armature_full: false,
    };

    convert_file(&options);
}
