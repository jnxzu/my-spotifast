# Local Windows source updates

`launch-local.ps1` is the startup boundary for this workstation's configured
Spotifast J shortcuts and Spotify links. It reads `[package].version` from the
local checkout and asks the installed executable for `--version`.

Only a strictly greater repository version runs a build. Equal and lower
versions skip immediately, even if source files changed. The checkout is assumed
current: this path never fetches, pulls, checks GitHub releases or downloads a
replacement application. Cargo uses the configured compiler and cached
dependencies offline, with its own `target/local-install` build directory.
Rustup's automatic toolchain installation is disabled too; missing build
prerequisites leave the installed copy usable and are reported in the build log.

A per-installation mutex combines simultaneous launches. If the app is already
running, an available update waits for a later launch. A successful build must
report the requested version, and the manifest must still have that version.
The launcher verifies copied bytes, replaces the executable atomically, updates
the existing Windows version record and keeps the previous executable for
rollback. Failed builds and stale output leave the installed copy available.

`install-local-updater.ps1` consumes a machine-local JSON configuration with the
repository, executable, Windows PowerShell, build script, Visual Studio and Rust
environment paths. It saves that configuration under LocalAppData, backs up
changed shortcuts and protocol commands, and preserves the application icon.
Only shortcuts and protocol commands already pointing to this installed copy
are changed. Setup disables its automatic GitHub update checks; other
preferences and protected sign-ins are untouched. The VBScript bootstrap hides
the build console and falls back to the installed app if the checkout is absent.

The state directory contains `update.log`, the last `build.log`, previous
installation files and original shortcut/protocol backups. No build command,
compiler path or machine-specific path belongs in the committed app settings.
Direct command-line use of `spotifast.exe` remains available.

Verification uses Windows PowerShell 5.1: `test-local-updater.ps1` exercises lower
and equal versions, higher-version planning, build failures, native stderr, stale output,
version changes during builds and invalid manifests against disposable copies
of the real executable. `launch-local.ps1 -CheckOnly` reports a decision without
building; `-NoLaunch` performs an update without opening the app window.
