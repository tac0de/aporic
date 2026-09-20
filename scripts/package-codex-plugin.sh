#!/bin/sh
set -eu

if [ "$#" -lt 2 ] || [ "$#" -gt 3 ]; then
  echo "usage: $0 OUTPUT_DIR ABSOLUTE_WORKSPACE [SCOPE]" >&2
  exit 2
fi

output_dir=$1
workspace=$2
scope=${3:-aporic}

case "$workspace" in
  /*) ;;
  *)
    echo "workspace must be an absolute path" >&2
    exit 2
    ;;
esac

if [ ! -d "$workspace" ]; then
  echo "workspace does not exist: $workspace" >&2
  exit 2
fi
workspace=$(CDPATH= cd -- "$workspace" && pwd -P)
if printf '%s' "$workspace" | LC_ALL=C grep -q '[\\"[:cntrl:]]'; then
  echo "workspace contains characters that cannot be rendered safely" >&2
  exit 2
fi

if ! printf '%s' "$scope" | grep -Eq '^[A-Za-z0-9_][A-Za-z0-9._-]*$'; then
  echo "scope must start with a letter, digit, or underscore and use only letters, digits, dot, underscore, or hyphen" >&2
  exit 2
fi
if [ "$(printf '%s' "$scope" | wc -c | tr -d ' ')" -gt 256 ]; then
  echo "scope exceeds 256 UTF-8 bytes" >&2
  exit 2
fi

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) ;;
  *)
    echo "the v0.2.0 plugin package supports macOS arm64 only" >&2
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
mkdir -p "$output_dir/.codex-plugin" "$output_dir/hooks" "$output_dir/bin" "$output_dir/config"
cp "$template_dir/.codex-plugin/plugin.json" "$output_dir/.codex-plugin/plugin.json"
cp "$repo_dir/target/release/aporic" "$output_dir/bin/aporic"
cp "$template_dir/policy.json" "$output_dir/config/policy.json"

APORIC_PACKAGE_WORKSPACE=$workspace APORIC_PACKAGE_SCOPE=$scope \
  perl -0pe '
    BEGIN {
      $workspace = $ENV{APORIC_PACKAGE_WORKSPACE};
      $workspace =~ s/([\$`])/\\\\$1/g;
    }
    s/__(APORIC_WORKSPACE|APORIC_SCOPE)__/
      $1 eq "APORIC_WORKSPACE" ? $workspace : $ENV{APORIC_PACKAGE_SCOPE}
    /gex;
  ' "$template_dir/hooks/hooks.json.template" > "$output_dir/hooks/hooks.json"

echo "$output_dir"
