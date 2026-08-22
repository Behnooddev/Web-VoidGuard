# Releasing VoidGuard

Releases are prepared automatically and **always require a manual
publish step** — nothing goes live without you clicking the button.

## How it works

1. **Cut a tag** (either push a tag, or trigger manually):

   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

   or, without pushing a tag yourself: go to **Actions → Prepare
   Release → Run workflow** and enter the tag name (it must already
   exist as a git tag).

2. **`.github/workflows/release.yml` runs automatically:**
   - `notes` job walks commits since the previous tag and generates
     categorized, clean release notes (Features / Fixes / Security /
     Performance / Docs / Refactoring / Tests / Chores / Other,
     plus a Breaking Changes section) — see
     `.github/scripts/generate-release-notes.mjs`. It reads
     [Conventional Commit](https://www.conventionalcommits.org/)
     prefixes (`feat:`, `fix:`, `security:`, etc.); anything that
     doesn't match a known prefix still shows up under "Other
     changes" rather than being dropped.
   - `build-windows` job builds the MSI/NSIS installers via
     `tauri-apps/tauri-action`.
   - `finalize-notes` job writes the generated notes onto the release
     and makes sure a **draft** release exists even if the Windows
     build failed, so you can see what happened either way.

3. **Nothing is published yet.** Go to the repo's **Releases** page —
   you'll see a draft with the generated notes and (if the build
   succeeded) the installers attached.

4. **Review and edit** the draft directly in the GitHub UI if
   anything needs adjusting — the generated notes are a clean
   starting point, not gospel.

5. **Click "Publish release."** That's the approval step. Only then
   does it become visible as a real release / show up in the
   repo's release feed / trigger release notifications.

## Commit message convention (for clean auto-generated notes)

Use Conventional Commit prefixes so commits land in the right
section automatically:

```
feat: add DNS validation to the network adapter view
fix: correct restart polling for stopped services
security: remove an unnecessary elevated handle in ports.rs
docs: update the port-control wiki page
chore: bump windows crate to 0.55
```

Add `!` after the type (`feat!:`) or a `BREAKING CHANGE:` footer for
anything that needs to show up under "Breaking changes."

## Testing the notes generator locally

```bash
node .github/scripts/generate-release-notes.mjs v0.2.0 --upto=HEAD
```

This prints what the notes would look like without needing to push a
tag or run CI.

## If the Windows build fails

The draft release is still created (with notes but no installers).
Fix the build, re-run the `build-windows` job (or the whole workflow)
from the Actions tab — `tauri-action` will update the same draft
rather than creating a duplicate.
