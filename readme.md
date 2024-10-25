# journey-model-parser

## How to use this guide
This guide assumes you have some experience with installing programs and interacting with system shells.
All needed steps should either be sufficiently explained, or where necessary you should be able to quickly find a solution through your search engine of choice (i.e. later we will be installing Python, a question that might come up for Windows users is how to add a program to PATH. For instance, searching 'windows add python to PATH' should provide helpful results. Stack Overflow usually has working answers). \
If you cannot figure out a certain step or feel that a section needs improvement, feel free to reach out in the `#modding-hacking` channel of the [Journey Discord](https://discord.gg/TG6GMq4Cyg).

## Getting binaries

### Compiling
Install the [rust compiler](https://www.rust-lang.org/tools/install), either through the official installer or your system's package manager. \
Run `cargo build --release` in the project root. \
Depending on your OS, this should yield a  `libjourney_model_parser.so` (Linux) / `journey_model_parser.dll` (Windows).

### Downloading artifacts
If you are unable to compile the project, binaries with detached signatures are available on GitHub under "Releases".
You should download both the binary and its signature matching your operating system (`.so` for Linux, `.dll` for Windows).

## Converting .xml to .obj
Run `cargo run --release -- path_to_xml.xml`, or use the compiled binary directly. The provided release artifacts cannot be used to convert to `.obj`, and provide only FFI (Foreign Function Interface) capabilities.

## Preparing Blender
For the Python script to work you will need to install `lupa` and `python-gnupg` for Blender's Python environment.
### Python - Linux
For any Linux distribution, please consult your system's package manager, Nix(OS) users can also use the provided `shell.nix`.
### Python - Windows
On Windows, open Blender, go to the `Scripting` tab in the top bar, and look at the console displayed in the bottom left corner. It should mention a Python version, e.g. "Python 3.11.10". Install this version of Python through either the [official installer](https://www.python.org/downloads/windows/) or a distribution like `anaconda` / `miniconda`. After you have installed Python, open a shell where you can access Python and `pip` (Run `python` or `pip` to verify functionality. You may need to add Python to your system's PATH). \
In the shell run this command: `pip install lupa python-gnupg`. After `pip` completes the installation, go to Python's install directory, then to `lib/site-packages/`. You should find `gnupg.py` and a `lupa/` directory. Copy both, then navigate to Blender's install directory, where you will find a `python/lib/site-packages/` folder. Paste your copied files there. \
Verify functionality by running `from lupa import LuaRuntime` and `import gnupg` in the Blender terminal.

### Other packages
For increased security you should also install GnuPG on your system. The Windows installer can be found [here](https://gpg4win.org/index.html), and every Linux distribution should provide this package.

## Why GnuPG and python-gnupg?
The Blender importer automatically verifies the signature of the provided binary to make sure you don't accidentally run malicious code.
This functionality *can* be disabled, **but it is strongly recommended NOT to do so**.

You can further verify the validity of your binary by using the github-cli: `gh attestation verify <file.(so|dll)> -R arckoor/journey-model-parser`. \
Release artifacts will always be a signed version of an actions artifact, and must have a valid attestation.

## Running the blender importer
Fill in the values for the constants at the top of the file. Use absolute paths to avoid issues locating files.
- `LIB_PATH` - the path to the directory containing both the binary file (`.so` / `.dll`) and its signature (`.sig`)
- `XML_PATH` - the path to all the converted XMLs
- `TEX_PATH` - the path to all the texture files
- `DMI_PATH` - the path to whatever `DecorationMeshInstances.lua` file you want to examine. The directory is sufficient, e.g. `Level_Barrens/`

On Windows, be sure to use `\\` (double backslash) as a path separator instead of `/`, e.g. `C:\\Users\\<Username>\\Documents\\`.

Afterwards, import the script in Blender's `Scripting` tab, and press "Run Script".
