# macOS preview implementation

Supported target layouts: Apple Silicon and Intel Claude.app with Contents/Resources/ion-dist.asar. Older directory-only layouts stop explicitly. Mirror remains Wangnov/claude-app-mirror, including SHA256 verification.

## Behavior

- Discover ~/Applications/Claude.app and /Applications/Claude.app; select a real application bundle manually. Read Info.plist version/identifier and Mach-O architecture. Show live original/managed process status.
- Store assistant settings, receipts, logs and managed backups in ~/Library/Application Support/ClaudeCnAssistant. Do not write Claude conversation/configuration files.
- Download the universal mirror DMG; mount read-only in a private temporary directory. Check mirror SHA256 again after mounting, bundle identity, compatible CPU, full Developer ID signature and Gatekeeper assessment. Reject downgrades and ad-hoc packages as originals.
- Ask before stopping Claude. Install to ~/Applications/Claude.app without sudo. A previous user application moves to ~/Applications/.ClaudeCnAssistant-backup-UUID/Claude.app. Existing system /Applications installation is retained. Select the updated user application afterward.
- Persist installation target/staging/backup paths before rename. On interruption, recheck the actual target; if missing, validate and restore the managed old bundle. Never claim success from launching an installer or from a saved target version alone.
- Clone the original with ditto to preserve executable permissions and framework links. Accept only bundle-internal links, never traverse links during validation/signing. Patch all Electron fuse tables in Electron Framework, including both universal slices. Add catalogs to ion-dist.asar and keep app.asar unchanged.
- Sign Mach-O files, then nested frameworks/apps, then outer bundle. Preserve source entitlements except team-bound identity/keychain groups, add disable-library-validation for local signing. Verify the complete signature. The localized copy uses ad-hoc signing; macOS privacy/keychain access can require authorization again.
- Keep mutable launcher scripts and receipt outside Claude.app, so log/script updates do not invalidate bundle signatures. Check main executable, framework, ASAR hashes and signature before launch/recovery.
- Launch through Node.js 22+ inspector injection, verify exact process executable path, close inspector after injection. Node is currently an external prerequisite; Finder launches also check /opt/homebrew/bin/node and /usr/local/bin/node.
- Backups/restoration reuse the existing journaled managed-directory switch. Restore only with Claude stopped, preserve replaced copies and reject corrupt receipts/resources or out-of-bundle links.

## Build and validation

GitHub Actions `macOS validation and preview` builds Apple Silicon and Intel artifacts on native macOS runners. Artifacts are tar.gz inside Actions artifact archives to preserve execute bits and symlinks. These are unsigned assistant previews, not notarized production releases.

On a Mac with Node 22+, Rust and Xcode command-line tools:

```sh
npm ci
npm run check
node --test tests/update-flow.test.cjs
cargo test -p assistant-core --locked
npm run desktop:build -- --bundles app --config '{"bundle":{"active":true}}'
```

Still requiring real Claude validation: mirror DMG Developer ID/Gatekeeper checks, first localized launch, UI translation coverage, permissions and keychain behavior, update with running original/copy, real backup restore after OS interruption. Automated fixture signing tests do not replace these checks. Do not describe this preview as production verified until these have passed.
