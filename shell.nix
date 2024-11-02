{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  buildInputs = with pkgs; [
    cargo
    clippy
    gcc
    rustc
    rustfmt

    blender-hip
    gh
    gnupg
    gzip
    imagemagick
    (python311.withPackages (ps: with ps; [ flake8 lupa python-gnupg ]))
    unzip
  ];

  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

  shellHook = ''
    alias cclean="cargo clean";
    alias cbuild="cargo build";
    alias cfmt="cargo fmt";
    alias cclippy="cargo clippy -- -D warnings";

    mkdir -p ./game-data/xml
    mkdir -p ./game-data/textures
    mkdir -p ./game-data/dmi

    export PYTHONPATH=$(python -c "import site; print(site.getsitepackages()[0])")
    verify() {
      gh attestation verify "$1" -R arckoor/journey-model-parser
    }
    sign() {
      gpg --default-key 42F041970716D5F9A42468CE50E6472D9E5513E8 --output "$1.sig" --detach-sign "$1"
    }
    convert_xml() {
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
    }
    convert_dds() {
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
    }
  '';
}
