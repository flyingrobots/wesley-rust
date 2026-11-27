#!/bin/sh
# Checks that the published crate carries what a redistribution must carry.
#
# Apache-2.0 requires LICENSE and NOTICE with every redistribution, and the
# manifest names a README. Cargo only packages files under the crate's own
# directory, so the crate keeps copies of the repository's LICENSE and NOTICE;
# this fails when a copy drifts from the original or a file is missing from
# the .crate that `cargo package` would build.
set -eu

crate="${1:-wesley-rust-syntax}"
root="$(cd "$(dirname "$0")/.." && pwd)"
dir="$root/crates/$crate"

status=0
for file in LICENSE NOTICE; do
  if ! cmp -s "$root/$file" "$dir/$file"; then
    echo "error: crates/$crate/$file is missing or differs from the repository's $file" >&2
    status=1
  fi
done

readme="$(cargo metadata --format-version 1 --no-deps --locked --manifest-path "$root/Cargo.toml" |
  jq -r --arg crate "$crate" '.packages[] | select(.name == $crate) | .readme // empty')"
if [ -z "$readme" ]; then
  echo "error: $crate names no readme in its manifest" >&2
  exit 1
fi
readme="${readme#"$dir"/}"

list="$(cargo package --list --locked --allow-dirty -p "$crate" --manifest-path "$root/Cargo.toml")"
for file in "$readme" LICENSE NOTICE; do
  if ! printf '%s\n' "$list" | grep -Fxq "$file"; then
    echo "error: the $crate package does not contain $file" >&2
    status=1
  fi
done

exit "$status"
