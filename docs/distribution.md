# Desktop distribution

The app currently ships as macOS and Linux desktop packages:

| Platform | Architectures | Release files |
|---|---|---|
| macOS | Apple Silicon (`arm64`), Intel (`x86_64`) | DMG and zipped `.app` |
| Linux | `x86_64` | AppImage |

## Package locally

Install Fastforge and package for the host OS:

```sh
make dist-install
make package-dist
```

Or choose a target explicitly:

```sh
make package-linux  # Linux; installs a checksum-verified appimagetool
make package-macos  # macOS; installs appdmg through npm
```

Packages are written under `dist/`. Linux packaging needs Docker-independent
system build tools for Flutter/GTK, `locate`, FUSE-compatible AppImage runtime
support, and network access to fetch the pinned AppImage tool. The Mac package
targets must run on macOS; Xcode cannot cross-build macOS applications from
Linux.

## Release a version

1. Open **Actions → Prepare Release** on `main` and enter a semantic version,
   for example `1.1.0`.
2. Review and merge the generated PR. It updates `pubspec.yaml` and
   `CHANGELOG.md`; Flutter's build number comes from the Actions run number.
3. Push the matching tag, for example `v1.1.0`.
4. **Desktop Packages** builds native release bundles on each target runner,
   attaches the macOS DMG/app ZIP and Linux AppImages to the GitHub Release, and
   publishes `SHA256SUMS.txt`.

Repository administrators must allow GitHub Actions to create pull requests
for the Prepare Release workflow.

The release workflow pins Flutter through `.fvmrc`, Rust through `mise.toml`,
Fastforge at 0.6.12, and the AppImage tooling by checksum. macOS packages are
ad-hoc signed, not Developer ID-signed or notarized; first-open Gatekeeper
guidance is in [macOS signing](macos-signing.md).

Windows and mobile packages are not included because this repository currently
ships and configures macOS/Linux desktop runners only. Linux ARM64 is omitted:
the pinned stable Flutter SDK does not provide a Linux ARM64 host SDK for the
native desktop build used here.
