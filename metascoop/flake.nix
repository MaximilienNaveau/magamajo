{
  description = "Metascoop Go development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Go toolchain
            go
            gopls # Go language server
            gotools # Additional Go tools
            go-tools # Static analysis tools

            # Build dependencies
            pkg-config

            # Git for cloning repos
            git

            # F-Droid tools
            jdk17
          ];

          shellHook = ''
            echo "🐹 Metascoop Go development environment loaded"
            echo "Go version: $(go version)"
            echo ""
            echo "Available commands:"
            echo "  go build             - Build the binary"
            echo "  go run .             - Run the application"
            echo "  go run . -debug      - Run in debug mode (no fdroid)"
            echo "  go test ./...        - Run tests"
            echo ""
            echo "Usage example:"
            echo "  go run . -ap apps.yaml -rd fdroid/repo -debug"
            echo ""
            echo "Set GITHUB_TOKEN environment variable for GitHub API access"
          '';

          # Environment variables
          CGO_ENABLED = "1";
        };
      }
    );
}
