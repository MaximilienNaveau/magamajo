# Metascoop-RS

Rust implementation of metascoop - a tool to automatically update F-Droid repositories by fetching APKs from GitHub releases.

## Overview

This is a complete rewrite of the original Go-based metascoop in Rust. The conversion was done to:
- Learn Rust
- Eliminate dependency vulnerabilities from the Go version
- Improve type safety and error handling

## Features

- Fetches APK files from GitHub releases
- Updates F-Droid repository metadata automatically
- Extracts screenshots from project repositories
- Generates changelog files from release notes
- Updates README.md with app information table

## Usage

```bash
# With environment variables
export GITHUB_TOKEN=your_token_here
cargo run --release

# With command line arguments
cargo run --release -- \
  --apps-path apps.yaml \
  --repo-dir fdroid/repo \
  --personal-access-token $GITHUB_TOKEN

# Debug mode (skip fdroid commands)
cargo run --release -- --debug
```

## Command Line Options

- `-a, --apps-path <PATH>`: Path to apps.yaml file (default: `apps.yaml`)
- `-r, --repo-dir <PATH>`: Path to fdroid "repo" directory (default: `fdroid/repo`)
- `-p, --personal-access-token <TOKEN>`: GitHub personal access token (can also use `GITHUB_TOKEN` env var)
- `-d, --debug`: Debug mode - won't run the fdroid commands

## Building

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test
```

## Dependencies

- **octocrab**: GitHub API client
- **tokio**: Async runtime
- **serde/serde_yaml/serde_json**: Serialization
- **clap**: Command-line argument parsing
- **anyhow/thiserror**: Error handling
- **tera**: Template engine for README generation
- **reqwest**: HTTP client for downloading assets

## Development Environment

This project uses Nix flakes for reproducible development environments:

```bash
# If using direnv
direnv allow

# Or manually
nix develop
```

## Exit Codes

- `0`: Success with significant changes
- `1`: Error occurred during execution
- `2`: No significant changes detected

## License

See the parent project's LICENSE file.
