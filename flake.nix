{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    systems.url = "github:nix-systems/default";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.systems.follows = "systems";
    };
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  } @ inputs:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        blender-with-packages = pkgs.blender-hip.withPackages (
          ps:
            with ps; [
              dataclasses-json
              flake8
              lupa
            ]
        );

        blender-unwrapped = pkgs.stdenv.mkDerivation {
          name = "blender-unwrapped";
          buildCommand = ''
            mkdir -p $out/bin
            ln -s ${blender-with-packages}/bin/blender-wrapped $out/bin/blender
          '';
        };

        mkScript = name: text: (pkgs.writeShellScriptBin name text);

        shellScripts = [
          (mkScript "verify" ''
            gpg --verify "$1.sig" "$1" && gh attestation verify "$1" -R arckoor/journey-model-parser
          '')

          (mkScript "sign" ''
            gpg --default-key 42F041970716D5F9A42468CE50E6472D9E5513E8 --output "$1.sig" --detach-sign "$1"
          '')

          (mkScript "convert_xml" ''
            if [ -z "$1" ] || [ -z "$2" ]; then
              echo "Usage: convert_xml <folder> <script.py>"
              return 1
            fi

            for file in "$1"/*.gz; do
              filename=''${file%.gz}
              gunzip "$file"
              python "$2" -i0 "$filename"
              rm "$filename.pschema"
              rm "$filename"
              mv "$filename.xml" "''${filename%.*}.xml"
            done
          '')

          (mkScript "convert_dds" ''
            if [ -z "$1" ]; then
              echo "Usage: convert_dds <folder>"
              return 1
            fi

            for file in "$1"/*.dds; do
              filename=''${file%.dds}
              echo "Converting $file to $filename.png"
              magick "$file" "$filename.png"
              rm "$file"
            done
          '')
          (mkScript "format" "black -l 120 .")
        ];
      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs;
            [
              rust-bin.stable.latest.default
              cargo-edit

              black
              blender-unwrapped
              gh
              gnupg
              gzip
              imagemagick

              unzip
            ]
            ++ shellScripts;

          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

          shellHook = ''
            export PYTHONPATH=$(python -c "import site; print(site.getsitepackages()[0])")

            mkdir -p ./game-data/xml
            mkdir -p ./game-data/textures
            mkdir -p ./game-data/dmi
          '';
        };
      }
    );
}
