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

if [ -n "$(git -C "$repo_root" status --porcelain)" ]; then
  echo "Aporic source must be clean before packaging" >&2
  exit 1
fi
aporic_commit=$(git -C "$repo_root" rev-parse --verify HEAD)
if [ -z "$(git -C "$repo_root" branch -r --contains "$aporic_commit")" ]; then
  echo "Aporic HEAD must be present on a fetched remote branch before packaging" >&2
  exit 1
fi

cargo build --manifest-path "$repo_root/Cargo.toml" --locked --release -p aporic-cli --bin aporicctl
mkdir -p "$destination/bin"
cp -R "$repo_root/packaging/aporic-codex/." "$destination/"
cp "$repo_root/target/release/aporicctl" "$destination/bin/aporicctl"
chmod 755 "$destination/bin/aporicctl" "$destination/scripts/run-hook.sh"
adapter_sha256=$(shasum -a 256 "$destination/bin/aporicctl" | awk '{print $1}')
printf '{"schema_version":1,"aporic_commit":"%s","adapter_sha256":"%s"}\n' \
  "$aporic_commit" "$adapter_sha256" > "$destination/aporic-build.json"
