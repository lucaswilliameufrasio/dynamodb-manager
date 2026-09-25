#!/usr/bin/env bash
set -euo pipefail

case "$(uname -m)" in
  x86_64)
    architecture=x86_64
    expected_sha256=a6d71e2b6cd66f8e8d16c37ad164658985e0cf5fcaa950c90a482890cb9d13e0
    ;;
  aarch64|arm64)
    architecture=aarch64
    expected_sha256=1b00524ba8c6b678dc15ef88a5c25ec24def36cdfc7e3abb32ddcd068e8007fe
    ;;
  *)
    printf 'Unsupported AppImage tool architecture: %s\n' "$(uname -m)" >&2
    exit 1
    ;;
esac

install_directory="${APPIMAGETOOL_BIN_DIR:-$HOME/.local/bin}"
temporary_file="$(mktemp)"
trap 'rm -f "$temporary_file"' EXIT

curl --fail --location --silent --show-error \
  "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-${architecture}.AppImage" \
  --output "$temporary_file"
actual_sha256="$(sha256sum "$temporary_file")"
actual_sha256="${actual_sha256%% *}"
if [[ "$actual_sha256" != "$expected_sha256" ]]; then
  printf 'appimagetool checksum mismatch: expected %s, got %s\n' \
    "$expected_sha256" "$actual_sha256" >&2
  exit 1
fi

install -d "$install_directory"
install --mode=0755 "$temporary_file" "$install_directory/appimagetool"
printf 'Installed verified appimagetool (%s) at %s\n' \
  "$architecture" "$install_directory/appimagetool"
