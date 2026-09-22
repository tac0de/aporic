#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "usage: $0 OUTPUT_DIR" >&2
  exit 2
fi

output_dir=$1

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) ;;
  *)
    echo "the v0.7.0 plugin package supports macOS arm64 only" >&2
    exit 2
    ;;
esac

if ! command -v cargo >/dev/null 2>&1 || ! command -v rustc >/dev/null 2>&1; then
  echo "cargo and rustc must be available in PATH to build the Aporic plugin package" >&2
  if [ -x /opt/homebrew/opt/rustup/bin/cargo ] && [ -x /opt/homebrew/opt/rustup/bin/rustc ]; then
    echo 'Homebrew rustup detected; add export PATH="/opt/homebrew/opt/rustup/bin:$PATH" to ~/.zprofile, start a new shell, and retry' >&2
  else
    echo "install Rust 1.89 or newer with rustup, start a new shell, and retry" >&2
  fi
  exit 2
fi

if [ -e "$output_dir" ]; then
  echo "output directory already exists: $output_dir" >&2
  exit 2
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)
template_dir="$repo_dir/packaging/codex-plugin"

cargo build --manifest-path "$repo_dir/Cargo.toml" --release --locked
cargo build --manifest-path "$repo_dir/Cargo.toml" --package aporic-structural-analyzer --target wasm32-unknown-unknown --release --locked
mkdir -p "$output_dir/.codex-plugin" "$output_dir/hooks" "$output_dir/bin" "$output_dir/analyzers"
cp "$template_dir/.codex-plugin/plugin.json" "$output_dir/.codex-plugin/plugin.json"
cp "$template_dir/.mcp.json" "$output_dir/.mcp.json"
cp "$repo_dir/target/release/aporic" "$output_dir/bin/aporic"
cp "$template_dir/hooks/hooks.json" "$output_dir/hooks/hooks.json"
cp "$repo_dir/target/wasm32-unknown-unknown/release/aporic_structural_analyzer.wasm" "$output_dir/analyzers/structural.wasm"

echo "$output_dir"
