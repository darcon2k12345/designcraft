# patronus-launcher

Installs and updates **Patronus Design**, **PdfCraft** and **PhotoCraft** from their GitHub releases
(`darcon2k12345/designcraft`, `darcon2k12345/pdfcraft`, `darcon2k12345/photocraft`). It is a thin
command-line layer over
[`craft-update`](https://github.com/storytold/craft-libs/tree/main/crates/update) (MIT OR
Apache-2.0), which does the release lookup, the checksum-verified download, the per-OS install
and the detection of existing copies.

```
patronus-launcher status [app]   # what is installed, the latest release, what would happen (default: all apps)
patronus-launcher check [app]    # same; exit 0 = all up to date, 10 = install/update available, 1 = error
patronus-launcher install <app>  # install, or update an older install
patronus-launcher update [app]   # update installed apps (all installed apps if none is named)
patronus-launcher launch <app>   # start the installed app
patronus-launcher apps           # list the managed apps
```

`<app>` is `patronusdesign` (also `designcraft`), `pdfcraft` (also `printcraft`) or `photocraft`.
Set `GITHUB_TOKEN` to raise GitHub's API rate limit.

## Where releases come from

`crates/release` (`patronus-release`) is the single definition of the apps' identities for both
this launcher and Patronus Design's own update check. PdfCraft and PhotoCraft are the `craft-update`
registry entries with their repository swapped to your forks. The Patronus Design repository is fixed **at build time**, so
nothing at run time can redirect updates:

```
PATRONUS_RELEASE_REPO=your-org/your-repo cargo build --release -p patronus-launcher -p patronusdesign
```

Without it, `DEFAULT_RELEASE_REPO` in `crates/release/src/lib.rs` is used (`darcon2k12345/designcraft`).

## Windows

`patronus-launcher.exe` is a console program with no installer: put it anywhere. Double-clicking
it (no arguments) shows the status, offers to install or update, offers to start the app, and
waits for Enter. From PowerShell the commands above work as usual. It keeps its records in
`%LOCALAPPDATA%\Patronus Launcher`. Windows packages are installed with the system's MSI
installer, which asks for administrator approval itself.

Getting a build with your release repository baked in:

1. **GitHub Actions (recommended).** `.github/workflows/launcher-windows.yml` builds it natively
   with MSVC using this repository as the release repository, then attaches
   `patronus-launcher-<version>-windows-x64.exe` and a checksum file to the release tag you
   give it (Actions > Launcher (Windows) > Run workflow).
2. **On a Windows machine:**
   `$env:PATRONUS_RELEASE_REPO = "owner/repo"; cargo build --release --locked -p patronus-launcher`
3. **Cross-compiled from Linux** (what produced the zip this was shipped with):
   `rustup target add x86_64-pc-windows-gnu`, install `gcc-mingw-w64-x86-64`, then
   `cargo build --release --locked --target x86_64-pc-windows-gnu -p patronus-launcher` with
   `CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc-posix`.

The executable is unsigned, so Windows SmartScreen will warn on first run until it is signed.

## What a release must contain

- Assets named `<app>-<version>-<os>-<arch>.<ext>` (`patronusdesign-…`, `pdfcraft-…`, `photocraft-…`), for example
  `patronusdesign-0.21.0-linux-x86_64.deb` or `patronusdesign-0.21.0-windows-x64.msi`. Releases that still
  use the old `designcraft-…` names are found too.
- A checksum for each asset (GitHub's per-file digest or a `SHA256SUMS.txt`). A file nothing
  vouches for is never installed.

## Publishing the Windows release

`.github/workflows/release-windows-unsigned.yml` builds the MSI and portable zip with
`packaging/windows/package.ps1` and publishes them with a `SHA256SUMS.txt` (Actions > Release
(Windows, unsigned) > Run workflow, with a tag matching the Cargo.toml version). The build is
unsigned until you add signing secrets, so SmartScreen warns on first run.

## Not handled yet

- A graphical window (this is the command-line core).
- The launcher updating itself. It has to be installed or updated by hand for now.
- Packages owned by a system package manager (deb, rpm, Flatpak) are left to that manager.
