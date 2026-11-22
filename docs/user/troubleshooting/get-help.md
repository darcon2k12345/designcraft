---
title: Report a problem
description: What to include and where to post when Patronus Design misbehaves or crashes.
order: 20
verified: 0.7.1
---

Post in the [issue tracker](https://github.com/storytold/designcraft/issues) or in [Discord](https://discord.gg/artcraft), and attach the log file. The log is the most useful thing you can send after a crash.

## What to include

- Your Patronus Design version and how you installed it (installer, portable, AppImage and so on).
- Your operating system and graphics card.
- What you did, what you expected and what happened.
- The log file `patronusdesign.log`.

## Where the log is

Patronus Design writes `logs/patronusdesign.log` in its settings folder, beside `ui.json` and `prefs.json`.

| System  | Log folder                                                              |
|---------|-------------------------------------------------------------------------|
| Windows | `%APPDATA%\Patronus Design\logs\`                                           |
| macOS   | `~/Library/Application Support/Patronus Design/logs/`                       |
| Linux   | `~/.config/patronusdesign/logs/` (or `$XDG_CONFIG_HOME/patronusdesign/logs/`) |

Each start moves the previous log to `patronusdesign.1.log`, and that one to `patronusdesign.2.log`. The log from a run that crashed survives the next start, so attach `patronusdesign.1.log` if you have restarted since. The file stops growing at 16 MiB.

## More detail in the log

Set the environment variable `RUST_LOG` to `debug` before starting Patronus Design to log more. The default is `info` for Patronus Design's own parts and `warn` for everything else.
