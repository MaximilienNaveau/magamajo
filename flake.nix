{
  description = "F-Droid repository management with metascoop";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config = {
            allowUnfree = true; # Required for Android SDK
          };
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            (rust-bin.stable.latest.default.override {
              extensions = [
                "rust-src"
                "rust-analyzer"
              ];
            })

            # Build essentials for Rust dependencies
            pkg-config
            openssl
            git

            # F-Droid server with all dependencies
            fdroidserver

            # Android SDK components (required by fdroidserver)
            android-tools # Provides aapt, adb, etc.

            # Java (required by fdroidserver)
            jdk17
          ];

          shellHook = ''
            echo "F-Droid + Rust development environment loaded"
            echo "Rust version: $(rustc --version)"
            echo "Cargo version: $(cargo --version)"
            echo "fdroidserver version: $(fdroid --version 2>/dev/null || echo 'fdroid available')"
          '';
        };
      }
    );
}
