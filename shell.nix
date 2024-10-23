{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  buildInputs = [
    pkgs.blender-hip
    pkgs.gh
    pkgs.gnupg
    (pkgs.python311.withPackages (ps: [ ps.lupa ps.python-gnupg ]))
  ];

  shellHook = ''
    export PYTHONPATH=$(python -c "import site; print(site.getsitepackages()[0])")
    verify() {
      gh attestation verify "$1" -R arckoor/journey-model-parser
    }
    sign() {
      gpg --default-key 42F041970716D5F9A42468CE50E6472D9E5513E8 --output "$1.sig" --detach-sign "$1"
    }
  '';
}
