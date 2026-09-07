# nyx-tui

### Unofficial VulNyx terminal dashboard — TUI no oficial para VulNyx

<p align="center">
  <img src="assets/dashboard-machines.png" alt="nyx-tui — Machines tab with Nord theme" width="100%">
</p>

**[English](README.md) | [Español](README.es.md)**

---

**nyx-tui** is an interactive terminal dashboard for [VulNyx](https://vulnyx.com): browse the machine catalog, submit first-blood flags, read and submit community writeups, and track your leaderboard position — all without leaving the terminal. Machine downloads open the VulnyX download page in your browser, where you complete the CAPTCHA and download the machine yourself.

One command, one screen: running `nyx` opens the dashboard. Written in pure **Rust** (ratatui), shipped as a single static binary. UI in English, Nord theme. **No account needed** — just a username.

---

## Screenshots

| Machines | Progress |
| :---: | :---: |
| ![Machines tab](assets/dashboard-machines.png) | ![Progress tab](assets/dashboard-progress.png) |

## Features

* **One command** — `nyx` opens the dashboard: catalog, first-blood flags, writeups and your leaderboard position in one screen.
* **Machines** — the full catalog with the site's official difficulty colors (Low/Easy/Medium/Hard), OS (Linux/Windows), tech tags, and first-blood status per machine. Instant `/` filtering and `s` sorting (site order → name → date → difficulty).
* **First-blood flag submission** — `f` opens a popup to submit User and Root flags (MD5). Slots already taken are shown as read-only notices; open slots are ready for your MD5 hash. Your username is attached automatically.
* **First-blood filter** — `b` shows only machines with an open first-blood slot, so you can be the first to pwn a fresh release.
* **Writeups** — per-machine community writeups popup (`w`): articles 📝 and videos 🎥 with author, language and date. Submit your own (`u`) — the submission popup uses pickers for **type** (Text/Video, single-select) and **language** (all 62 codes of vulnyx.com, multi-select), so nothing gets mistyped.
* **Completed tracking** — machines you finished turn `✓` green in the list: automatic when one of your writeups is approved on the site, or manually per machine with `m` (persisted locally in `~/.nyx-tui/config.json`). `x` hides completed machines so only pending ones remain.
* **Progress** — your first bloods, writeups and leaderboard position (computed from public data with the site's own scoring rules, including the site's staff exclusions so ranks match vulnyx.com).
* **Username management** — `a` opens the username popup; whatever you set is attached to all submissions and used for your leaderboard position.

---

## Prerequisites

* **OS**: Linux (developed and tested on Arch Linux); macOS and Windows release binaries are provided.
* A [VulNyx](https://vulnyx.com) account is **not** required — the platform works with a self-declared username.

---

## Installation

### 1. From a release binary (easiest)

Grab the archive for your platform from the [Releases](https://github.com/setyanoegraha/nyx-tui/releases) page:

| Platform | Archive |
| :--- | :--- |
| Linux x86_64 | `nyx-v0.2.0-x86_64-unknown-linux-gnu.tar.gz` |
| macOS Apple Silicon | `nyx-v0.2.0-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `nyx-v0.2.0-x86_64-apple-darwin.tar.gz` |
| Windows x86_64 | `nyx-v0.2.0-x86_64-pc-windows-msvc.zip` |

```bash
tar xzf nyx-v0.2.0-x86_64-unknown-linux-gnu.tar.gz
install -m 755 nyx ~/.local/bin/nyx
```

### 2. From source

```bash
git clone https://github.com/setyanoegraha/nyx-tui.git
cd nyx-tui
cargo install --path .
```

### 3. From git directly

```bash
cargo install --git https://github.com/setyanoegraha/nyx-tui.git
```

> Requires the Rust toolchain (1.85+): https://rustup.rs

---

## First Setup

Just run:

```bash
nyx
```

The dashboard loads immediately — no login popup. Press `a` to set your **username** (e.g. `noneofyour`). This username is attached to all flag and writeup submissions and used to compute your leaderboard position. You can change it at any time.

---

## Usage Guide

Two keyboard-driven tabs — **Machines** and **Progress**:

| Keys | Action |
| :--- | :--- |
| `Tab` / `←` `→` | Switch tabs |
| `↑` `↓` / `j` `k` | Move selection |
| `g` / `Home` | Jump to the top of the list |
| `/` | Filter the current list (type to narrow, `Enter` keeps it, `Esc` clears & exits) |
| `s` | **Machines** — cycle sort: site order → name → date → difficulty |
| `b` | **Machines** — show only machines with an open first-blood slot |
| `m` | **Machines** — mark/unmark the selected machine as completed (stored locally) |
| `x` | **Machines** — hide/show completed machines |
| `d` | **Machines** — open the machine's download page in your browser (CAPTCHA + download happen there) |
| `f` | **Machines** — first-blood flag popup: submit User and/or Root flags (MD5). Slots already taken are shown as read-only notices |
| `w` | **Machines** — community writeups popup for the selected machine: `j`/`k` select, `Enter` opens the link |
| `u` | **Machines** — submit a writeup URL for the selected machine (pending admin review) |
| `i` / `Enter` | **Machines** — description popup with tags, platforms, MD5, first-blood holders |
| `a` | **Anywhere** — username popup |
| `Enter` | **Progress** — open the selected writeup in your browser |
| `r` | Re-fetch all data |
| `q` / `Esc` / `Ctrl-C` | Quit |

### Downloads

VulnyX gates machine downloads behind a CAPTCHA image (5 characters, A-Z 0-9) in the browser. nyx-tui keeps the flow where it already works well:

1. Press `d` on a machine — its download page (`https://vulnyx.com/download.php?vm=<name>`) opens in your browser.
2. Complete the CAPTCHA and download the `.ova` there.

nyx-tui stays out of the way: no in-app download, no image viewer, no manual code entry.

### Where Your Data Lives
- `~/.nyx-tui/config.json` — your **username** and locally marked completed machines. Nothing else.
- No password is stored — VulNyx has no accounts, and the username is self-declared.

---

## Updating

```bash
cargo install --git https://github.com/setyanoegraha/nyx-tui.git --force
```

or grab the latest release binary from the [Releases](https://github.com/setyanoegraha/nyx-tui/releases) page.

### Uninstallation & Cleanup

```bash
cargo uninstall nyx
```

Delete `~/.nyx-tui/` to clear all local data.

---

## Disclaimer

nyx-tui is an **unofficial** community tool and is not affiliated with VulNyx. It only uses the platform's public JSON data and opens the same download pages the web app uses in your browser — be kind to the service.

---

## Acknowledgements

Thanks to the VulNyx team and community for the platform, and to the [ratatui](https://github.com/ratatui/ratatui) team for the toolkit.

Sibling projects:
- [hmv-tui](https://github.com/setyanoegraha/hmv-tui) — the same dashboard concept for HackMyVM
- [dl-tui](https://github.com/setyanoegraha/dl-tui) — the same dashboard concept for DockerLabs

---

Made with ❤️ by [Ouba](https://github.com/setyanoegraha).

*Happy hacking on VulNyx!*
