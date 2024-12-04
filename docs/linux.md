# journey-model-parser (Linux)
This part of the guide assumes you have at least some experience using the shell and various commands. \
As there are countless distributions, you will need to consult your package manager on how to install the tools we will need. If you have access to the [nix](https://nixos.org/download/) package manager, you can use the provided [shell.nix](../shell.nix) file, it sets up everything necessary. The following assumes you use nix; if you don't, refer to `shell.nix` for the commands used.

## Packages
- `blender`
- the Python `lupa` package
- `imagemagick`
- `unzip`
- If you want to compile the library yourself, you will need the `rust` compiler

If you do not want to compile yourself, and instead use the GitHub releases, you will need the following as well:
- `gnupg`
- `python-gnupg`

Additionally you will need `pssgConverter.py`, which you can get [here](https://www.nexusmods.com/greedfall/mods/116?tab=files) (v2.04 is known to work).

## XML files
After extracting the `.ipa` file (rename to `.zip` and use `unzip`), copy the contents of `Payload/JourneyLaunch.app/Data/Meshes` to `game-data/xml`. You can safely delete `game-data/xml/Cloth`. Then, run `convert_xml game-data/xml path/to/pssgConverter.py`. You should be left with just XML files in the `game-data/xml`.

## DMI files
Navigate to `Payload/JourneyLaunch.app/Data/Scripts`, where you will find 13 folders named `Level_*`. Move these folders to `game-data/dmi`, the relevant file is always `DecorationMeshInstances.lua`.

## Textures
For acquiring textures, a Steam copy of the game is necessary. Copy the contents of `Journey/Data/Textures/Bin` to `game-data/textures`, and delete all `*.dds.D3D11x64` files.
Unfortunately, there are some issues with select files, almost no program can convert them. For the following files, you will need to install [paint.net](https://www.getpaint.net/download.html) (on Windows), open the file and save it as `.dds` again. Replace the original files with the ones you re-exported and continue.
The relevant files are:
- `GoldLightRampGoldLightRamp.dds`
- `Ruin_LightRampRuin_LightRamp.dds`

Run `convert_dds game-data/textures`, which should leave you with the `textures` folder full of `.png` files.

## Compiling (optional)
If you got your binaries from GitHub, you can skip this step.

Run `cargo build --release` in the project root. \
Now that you have self-compiled binaries, you can set `VERIFY_SIGNATURE` at the top of the blender python script to `False` and comment out the Exception when prompted.

## Running the blender importer
Open `blender_import.py` in any editor, or directly within Blender through the `Scripting` tab.
Fill in the values for the constants at the top of the file. `<path>` refers to the absolute prefix to the file / folder, so the actual value would be e.g. `/home/arckoor/journey-model-parser/game-data/...`.
- `LIB_PATH` - `<path>/target/release` - if you got your binaries from GitHub, use the path to folder containing both the `.so` and the `.sig` file here.
- `XML_PATH` - `<path>/game-data/xml`
- `TEX_PATH` - `<path>/game-data/textures`
- `DMI_PATH` - `<path>/game-data/dmi/Level_<level>`

Afterwards, if you haven't yet, import the script in Blender's `Scripting` tab, and press "Run Script".

## Converting `.xml` to `.obj`
If you just want `.obj` files, run `cargo run --release -- path/to/file.xml`, or use `target/release/journey-model-parser path/to/file.xml` directly. This functionality is only available if you compiled the project yourself.
