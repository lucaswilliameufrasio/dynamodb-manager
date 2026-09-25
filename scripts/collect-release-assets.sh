#!/usr/bin/env bash
set -euo pipefail

platform="$1"
architecture="$2"
tag="$3"
version="${tag#v}"
output_directory="dist/release-assets"

mkdir -p "$output_directory"

copy_one() {
  local extension="$1"
  local output_name="$2"
  local matches=()
  while IFS= read -r -d '' file; do
    matches+=("$file")
  done < <(find dist -type f -name "*.$extension" ! -path "$output_directory/*" -print0)

  if [[ "${#matches[@]}" -ne 1 ]]; then
    printf 'Expected one .%s package under dist, found %s\n' \
      "$extension" "${#matches[@]}" >&2
    exit 1
  fi
  cp "${matches[0]}" "$output_directory/$output_name"
}

case "$platform" in
  linux)
    copy_one AppImage "DynamoDB-Manager-linux-${architecture}-${tag}.AppImage"
    ;;
  macos)
    copy_one dmg "DynamoDB-Manager-macos-${architecture}-${tag}.dmg"
    copy_one zip "DynamoDB-Manager-macos-${architecture}-${tag}.app.zip"
    ;;
  *)
    printf 'Unsupported release platform: %s\n' "$platform" >&2
    exit 1
    ;;
esac

printf 'Prepared release assets in %s for %s (%s)\n' \
  "$output_directory" "$tag" "$architecture"
