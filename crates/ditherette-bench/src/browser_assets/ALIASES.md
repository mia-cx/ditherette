# Runtime file identity

WebKit's WPEBackend names refer to one shared library through source symlinks.
Copying each name into an independent file caused a second library instance and the WS singleton assertion.
All 65 copied runtime/private-library paths had identical bytes and read/execute permissions.
The missing property was file identity, not content.

Snapshots now group source files by resolved device/inode identity.
Each group uses its first sorted relative path as the canonical file.
Other entries carry `alias_of` and become hardlinks to that copied canonical file.
Independent equal-byte files remain independent.
Resnapshotting preserves existing hardlink groups.

The manifest digest includes alias_of but excludes device/inode numbers and storage roots.
Structural validation requires aliases to name an earlier canonical entry with identical bytes, digest, and mode.
Filesystem validation recomputes the groups and rejects split or merged identities.
Final snapshots still reject symlinks. File identity inspection requires Unix, as does the owned browser lease protocol.

## Evidence

Before the implementation changed, a fresh private hardlinked runtime passed newPage and content checks.
The focused snapshot test then failed on separate copied inodes and passed after the fix.
It covers source symlinks, existing hardlinks, independent equal-byte files, resnapshot identity,
digest binding, alias removal, split groups, and accidentally merged groups.

The Rust-created runtime at `target/s20-webkit-alias` passes actual WebKit newPage, setContent, and textContent checks.
Browser PID 639383 exited successfully and Playwright completed cleanup.
The inherited automation warning still appears without the crash.
The coordinator also reports installed-package conformance passing in Chromium, Firefox, and WebKit with this runtime.

The complete bench suite passes 45 top-level tests, with one child-only fixture ignored in the ordinary listing.
Formatting and the separate trusted S18 guard pass. No benchmark measurements ran.
