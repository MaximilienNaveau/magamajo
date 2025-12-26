{
  description = "Metascoop Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # rust-overlay provides several benefits over using Rust directly from nixpkgs:
    # 1. Always up-to-date: Gets Rust toolchains directly from the official Rust distribution,
    #    so you get new stable releases immediately (nixpkgs can lag behind by weeks)
    # 2. Version flexibility: Easy to pin to specific Rust versions or switch between stable/beta/nightly
    #    e.g., pkgs.rust-bin.stable."1.75.0".default for a specific version
    # 3. Component management: Cleanly add rust-analyzer, rust-src, clippy as extensions
    # 4. Consistency: Uses the same binaries as rustup, avoiding subtle differences
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
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            cargo
            rustc
            rust-analyzer
            rustfmt
            clippy

            # Dependencies for the project
            pkg-config
            openssl
            git

            # For F-Droid integration
            jdk17
          ];

          shellHook = ''
            echo "Rust development environment loaded"
            echo "Rust version: $(rustc --version)"
            echo "Cargo version: $(cargo --version)"
          '';

          # Environment variables
          RUST_BACKTRACE = "1";
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
        };
      }
    );
}
