# journey-model-parser

## How to use this guide
There are different guides available for both Linux and Windows.
If you cannot figure out a certain step or feel that a section needs improvement, feel free to reach out in the `#modding-hacking` channel of the [Journey Discord](https://discord.gg/TG6GMq4Cyg).

## Preparation

Clone this repo using `git clone https://github.com/arckoor/journey-model-parser.git`.

If you plan on compiling the rust library yourself, you can skip the next step.

### Downloading binaries
Binaries with detached signatures are available on GitHub under "Releases".
Download both the binary and its signature (`.sig`) matching your operating system (`.so` for Linux, `.dll` for Windows).

### Getting the game files
You will need to source the iOS version of the game (`Journey.ipa`). You can get this file by converting it from your own installation, or by finding it online.

You also need the Steam version for textures.

After you have acquired both, continue with the guide matching your operating system:
- [Linux](./linux.md)
- [Windows](./windows.md)
