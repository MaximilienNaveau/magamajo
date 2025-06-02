{ pkgs ? import (fetchTarball "https://channels.nixos.org/nixos-unstable/nixexprs.tar.xz") {} }:

pkgs.mkShell {
  buildInputs = [
    pkgs.go_1_24
  ];

  shellHook = ''
    echo "Nix shell with Go 1.24 is ready!"
  '';
}
