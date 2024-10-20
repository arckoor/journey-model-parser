use std::env;
use std::path::Path;

use journey_model_parser::convert_file;

fn main() {
    let args: Vec<String> = env::args().collect();
    let xml_file = Path::new(&args[1]);

    assert!(xml_file.exists(), "File does not exist");

    convert_file(xml_file);
}
