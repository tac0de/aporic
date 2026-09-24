#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "usage: package-aporic-codex.sh ABSOLUTE_DESTINATION" >&2
  exit 64
fi

destination=$1
case "$destination" in
  /*) ;;
  *) echo "destination must be absolute" >&2; exit 64 ;;
esac

if [ -e "$destination" ]; then
  echo "destination already exists" >&2
  exit 1
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)

cargo build --manifest-path "$repo_root/Cargo.toml" --locked --release -p aporic-cli --bin aporicctl
mkdir -p "$destination/bin"
cp -R "$repo_root/packaging/aporic-codex/." "$destination/"
cp "$repo_root/target/release/aporicctl" "$destination/bin/aporicctl"
chmod 755 "$destination/bin/aporicctl" "$destination/scripts/run-hook.sh"
