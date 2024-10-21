# journey-model-parser

## Compiling the library
Run `cargo build --release` in the project root. \
Depending on your OS this should yield a  `libjourney_model_parser.so` / `journey_modeL_parser.dll`.

## Converting to .obj
Run `cargo run -- path_to_xml.xml`. You can also run with `--release` or run the resulting binary directly with an argument.

## Running the blender importer
Fill in the values for the constants at the top of the file. Use absolute paths to avoid problems with locating a file.
- `LIB_PATH` - the path to your generated `.so` / `.dll`, which is located in `target/release/*.(so|dll)`
- `XML_PATH` - the path to all the converted XMLs
- `TEX_PATH` - the path to all the texture files
- `DMI_PATH` - the path to whatever DecorationMeshInstances.lua file you want to examine. The directory is sufficient, e.g. `Level_Barrens/`

Afterwards, import the script in Blenders "Scripting" tab, and press run.
