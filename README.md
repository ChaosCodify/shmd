# SH.MD

[![Ko-fi](https://img.shields.io/badge/Ko--fi-Support%20Me-ff5e5b?logo=ko-fi&logoColor=white)](https://ko-fi.com/chaoscodify)

A local markdown editor built with Tauri (Rust) + CodeMirror. No account, no telemetry, no cloud.

Runs entirely on your machine. Your files and sessions stay on disk — nothing is uploaded anywhere.

> Agent Coded Software: You've been warned

## Features

- Live markdown preview
- Multi-file: open several `.md` / `.markdown` / `.txt` files at once
- File sidebar with groups, save/restore of your session between launches
- Markdown cheatsheet built in
- Custom title bar and themes (theme editor included)
- Export / Import session as JSON
- Status-bar word/character stats

## Shortcuts

| Action                | Shortcut        |
|-----------------------|-----------------|
| New file              | `Ctrl+N`        |
| Open file(s)          | `Ctrl+O`        |
| Save                  | `Ctrl+S`        |
| Markdown cheatsheet   | `Ctrl+Shift+M`  |

## Download

Grab the latest portable exe from [Releases](https://github.com/ChaosCodify/shmd/releases) — no install needed.

## Build from source

Prerequisites: [Rust](https://rustup.rs/), [Tauri CLI](https://tauri.app/), Node.js.

```sh
npm install
npm run build       # full installers
# or
npm run build-portable   # just the shmd.exe binary
```

## Notes

- The editor libraries (CodeMirror, highlight.js) load from a CDN, so an internet connection is needed for syntax highlighting to load.
- Windows only.