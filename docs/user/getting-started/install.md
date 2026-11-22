---
title: Install Patronus Design
description: Download Patronus Design 0.7.1 for Windows 11, macOS or Linux, or install it from ArtCraft Launcher.
order: 20
verified: 0.7.1
---

Download the file for your system from the [latest release](https://github.com/storytold/designcraft/releases/latest) and install it. In the file names, `<version>` stands for the version number, for example `0.7.1`. The release also has `SHA256SUMS.txt`, which lists a checksum for every file. The [Patronus Design page on getartcraft.com](https://getartcraft.com/apps/designcraft) links to the download as well.

## Windows 11

1. Open the [latest release](https://github.com/storytold/designcraft/releases/latest).
2. Download `patronusdesign-<version>-windows-x64.msi`. It fits most PCs (64-bit Intel and AMD).
3. Run the file and follow the installer.
4. Start **Patronus Design** from the Start menu.

The release also has an `arm64` installer for ARM PCs such as Snapdragon, and a 32-bit `x86` installer. To run without installing, download `patronusdesign-<version>-windows-x64-portable.zip` (or the `arm64` or `x86` version), unzip it and start the program inside. The installers and executables are code-signed.

## macOS

1. Download `patronusdesign-<version>-macos-universal.dmg`. It works on Apple silicon and Intel Macs.
2. Open it and drag Patronus Design to Applications.

The app is signed and notarized.

## Linux

Pick one file from the release. Replace `x86_64` with `aarch64` on ARM.

| File                                          | Notes                                                    |
|-----------------------------------------------|----------------------------------------------------------|
| `patronusdesign-<version>-linux-x86_64.AppImage` | Runs anywhere.                                           |
| `patronusdesign-<version>-linux-x86_64.flatpak`  | Sandboxed. Install with `flatpak install --user <file>`. |
| `patronusdesign-<version>-linux-x86_64.deb`      | Debian and Ubuntu.                                       |
| `patronusdesign-<version>-linux-x86_64.rpm`      | Fedora, RHEL and openSUSE.                               |
| `patronusdesign-<version>-linux-x86_64.tar.gz`   | Unpack anywhere.                                         |

For RISC-V there is a `.deb`, an `.rpm` and a tarball, named with `riscv64` in place of `x86_64`. There is no AppImage or Flatpak for RISC-V. The RISC-V build needs glibc 2.39 or newer (Ubuntu 24.04 or newer). A FreeBSD build (`patronusdesign-<version>-freebsd-x86_64.tar.gz`) is on the release page too.

## Install with ArtCraft Launcher

ArtCraft Launcher lists Patronus Design and can install and update it for you. See [ArtCraft Launcher](https://getartcraft.com/docs/craft-launcher) for how it works.

## First run

Open Patronus Design. You land on the start screen. Continue with [Getting started](index.md). If the window closes right away, see [Patronus Design closes right after it starts](../troubleshooting/app-closes-at-start.md).
