{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  buildInputs = [
    pkgs.blender-hip
    (pkgs.python311.withPackages (ps: [ ps.lupa ]))
  ];

  shellHook = ''
    export PYTHONPATH=$(python -c "import site; print(site.getsitepackages()[0])")
  '';
}
