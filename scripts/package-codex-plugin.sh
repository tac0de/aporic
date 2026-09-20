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
    echo "the v0.3.0 plugin package supports macOS arm64 only" >&2
    exit 2
    ;;
esac

if [ -e "$output_dir" ]; then
  echo "output directory already exists: $output_dir" >&2
  exit 2
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)
template_dir="$repo_dir/packaging/codex-plugin"

cargo build --manifest-path "$repo_dir/Cargo.toml" --release --locked
mkdir -p "$output_dir/.codex-plugin" "$output_dir/hooks" "$output_dir/bin"
cp "$template_dir/.codex-plugin/plugin.json" "$output_dir/.codex-plugin/plugin.json"
cp "$repo_dir/target/release/aporic" "$output_dir/bin/aporic"
cp "$template_dir/hooks/hooks.json" "$output_dir/hooks/hooks.json"

echo "$output_dir"
