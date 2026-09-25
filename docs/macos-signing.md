# macOS downloads and signing

The release workflow packages the app as a DMG and a zipped `.app`. Builds use
the Xcode project's ad-hoc signing identity. This is not a Developer ID
signature and the app is not notarized, so Gatekeeper may ask you to approve it
the first time it opens.

If you trust the release, Control-click **DynamoDB Manager.app** and choose
**Open**. If macOS still blocks it, remove quarantine from that downloaded app
bundle only:

```sh
xattr -dr com.apple.quarantine "/Applications/DynamoDB Manager.app"
open "/Applications/DynamoDB Manager.app"
```

## Developer ID signing and notarization

To remove the first-launch Gatekeeper warning, the release must be signed with
an Apple Developer ID Application certificate and notarized. The workflow does
not require signing secrets today. Do not commit certificates, private keys, or
passwords.

To enable notarization later, store a protected `.p12`, its password, Apple
Team ID, Apple ID (or App Store Connect API key) as GitHub Actions secrets; sign
the `.app` before packaging, submit it with `xcrun notarytool`, wait for an
accepted result, staple the app and DMG, and verify them with `codesign` and
`spctl` on a macOS runner.
