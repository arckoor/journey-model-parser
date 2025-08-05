# journey-model-parser (Windows)

## Programs
Before starting, you will need to install the following:
- [Blender](https://www.blender.org/download/)
- [Python](https://www.python.org/downloads/windows/) (`anaconda` / `miniconda` works as well) See [Preparing Blender](#preparing-blender) for the correct version
- [DirectXTex](https://github.com/microsoft/DirectXTex/releases) (only `texconv.exe`)
- [paint.net](https://www.getpaint.net/download.html)
- [Rust](https://www.rust-lang.org/tools/install) (optional, only if you want to compile yourself)

Additionally you will need `pssgConverter.py`, which you can get [here](https://www.nexusmods.com/greedfall/mods/116?tab=files) (v2.04 is known to work).


Create the following folder structure in the project root:
- `game-data/dmi`
- `game-data/textures`
- `game-data/xml`

## XML files
After extracting the `.ipa` file (rename to `.zip`) copy the contents of `Payload/JourneyLaunch.app/Data/Meshes` to `game-data/xml`. You can safely delete `game-data/xml/Cloth`. Extract all `*.PSSG.gz` files. Copy `pssgConverter.py` to the `xml` folder. Paste the following code into a `.bat` file within the `xml` folder, and run it:
```bat
setlocal enabledelayedexpansion

for %%f in (*.PSSG) do (python pssgConverter.py "%%f")
for %%f in (*.PSSG.xml) do (
	set "filename=%%~nf"
	ren "%%f" "!filename:~0,-5!.xml!"
)
```
You can delete the `.bat` and `.py` files afterwards.

## DMI Files
Navigate to `Payload/JourneyLaunch.app/Data/Scripts`, where you will find 13 folders named `Level_*`. Move these folders to `game-data/dmi`, the relevant file is always `DecorationMeshInstances.lua`.

## Textures
For acquiring textures, a Steam copy of the game is necessary. Copy the contents of `Journey/Data/Textures/Bin` to `game-data/textures`, and delete all `*.dds.D3D11x64` files.
Some files require preprocessing with paint.net. Open them, and save them again as `.dds` in the same location:
- `GoldLightRampGoldLightRamp.dds`
- `Ruin_LightRampRuin_LightRamp.dds`

Copy `texconv.exe` to the `textures` folder and create another `.bat` file there with the following code, and run it.
```bat
for %%f in (*.dds) do (.\texconv.exe "%%f" -ft png)
```
Delete `*.bat` and `texconv.exe` afterward.

## Compiling (optional)
If you got your binaries from GitHub, you can skip this step.

Run `cargo build --release` in the project root.

## Preparing blender
Open Blender, go to the `Scripting` tab in the top bar, and look at the console displayed in the bottom left corner. It should mention a Python version, e.g. "Python 3.11.10" - this is the version of Python you should install. Open a shell where you can access Python and `pip` (Run `python` or `pip` to verify functionality. You may need to add Python to your system's PATH). \
In the shell, run this command: `pip install lupa dataclasses-json`. After `pip` completes the installation, go to Python's install directory, then to `lib/site-packages/`. You should find the following directories:
- `dataclasses_json`
- `lupa`
- `marshmallow`
- `packaging`

If you do not have directories but only `*.py` files with the same name, use these. Copy all of them, then navigate to Blender's install directory, where you will find a `python/lib/site-packages/` folder. Paste your copied files there. \
Verify functionality by running `from lupa import LuaRuntime` and `import dataclasses_json` in the Blender terminal.

## Running the blender importer
Open `blender_import.py` in any editor, or directly within Blender through the `Scripting` tab.
Fill in the values for the constants at the top of the file. `<path>` refers to the absolute prefix to the file / folder, so the actual value would be e.g. `C:\\Users\\arckoor\\journey-model-parser\\game-data\\...`. Be sure to use `\\` instead of `/` as a path separator.
- `LIB_PATH` - `<path>\\target\\release` - if you got your binaries from GitHub, use the path to the folder containing the `.dll` file here.
- `XML_PATH` - `<path>\\game-data\\xml`
- `TEX_PATH` - `<path>\\game-data\\textures`
- `DMI_PATH` - `<path>\\game-data\\dmi\\Level_<level>`

Afterwards, if you haven't yet, import the script in Blender's `Scripting` tab, and press "Run Script".

## Converting `.xml` to `.obj` / `.json`
If you want the `.obj` or `.json` files, run e.g. `journey-model-parser.exe --obj file.xml`. See `--help` for more info.
If you use cargo, run `cargo run --release -- --obj file.xml`.
