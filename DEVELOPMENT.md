# Maillet — Development

Technical notes for building, packaging and extending Maillet. User documentation lives in [README.md](README.md) and [docs/](docs/): [accounts](docs/accounts.md), [rules](docs/rules.md), [local API](docs/api.md) and [configuration files](docs/configuration.md).

## How It Works

Maillet is built in layers; dependencies only point downwards:

| Layer | Path | Job |
|---|---|---|
| App | `src-tauri/src/app/` | tray icon (StatusNotifierItem via `ksni`), windows, Tauri commands, watcher tasks |
| Features | `src-tauri/src/features/` | look at new mails and attach *findings*; OTP detection (`features/otp/`) is one feature among others |
| Mail core | `src-tauri/src/mail/` | check mailboxes, keep the unread mails (`MailStore`), read, mark and move mails |
| Base | `src-tauri/src/base/` | configuration, keyring, log, connection status |

- **Watcher:** one IMAP connection per account. On connect it loads the newest 200 unread mails (header plus the first 4 KB of the body for the preview, `BODY.PEEK`, so nothing is marked as read), then waits in **IDLE** ([RFC 2177](https://tools.ietf.org/html/rfc2177)). After every wake-up it re-syncs the unread list, so mails read or moved elsewhere disappear. IDLE is re-issued every 5 minutes; commands time out after 30 s; failed connections reconnect with exponential backoff (5 s … 5 min).
- **Features:** mails arriving while Maillet runs go through the feature pipeline (`features::Feature`). A feature may ask for the full body; it is fetched once for all features. The OTP feature matches its rules and opens the code popup.
- **Commands:** reading, marking and archiving use a second connection per account (the watcher sits in IDLE), kept for reuse for up to 4 minutes. *Erledigt* marks the mail as read and moves it to the `\Archive` folder, Gmail's `\All`, or a folder named *Archive*/*Archiv*. Deleting moves to `\Trash`.
- **Tray:** the number of unread mails is drawn into the icon; a left click opens the window at the icon. Built on StatusNotifierItem directly, because libappindicator does not report left clicks. On X11 the window activates itself like a pager would (`_NET_ACTIVE_WINDOW`), otherwise focus-stealing prevention keeps it unfocused.
- Bodies are decoded with `mail-parser` (multipart, quoted-printable, base64, charsets); cut-off MIME parts of the preview are closed before parsing.

## Building from Source

Requirements: Linux with GTK 3 and WebKit2GTK 4.1, a StatusNotifierItem tray (Cinnamon, KDE, XFCE; GNOME with the AppIndicator extension); Rust 1.90+.

```bash
sudo apt install build-essential pkg-config \
  libwebkit2gtk-4.1-dev libgtk-3-dev \
  librsvg2-dev libssl-dev libdbus-1-dev

git clone https://github.com/Skrrytch/Maillet.git
cd maillet/src-tauri
cargo build --release
./target/release/maillet
```

### Tests

```bash
cargo test                    # unit tests
cargo clippy --all-targets
```

**Integration test against a local IMAP server** (Dovecot in Docker, no real mailbox involved):

```bash
docker run -d --name maillet-imap -e USER_PASSWORD=testpass \
  -p 127.0.0.1:31143:31143 -p 127.0.0.1:31993:31993 dovecot/dovecot:latest
python3 tests/dovecot/seed.py initial         # 4 unread mails, 1 read, folder "Archive"
cargo test imap_commands -- --ignored         # preview, read/unread, archive, delete
docker rm -f maillet-imap
```

To try the whole app against it, add an account *127.0.0.1:31993, SSL, accept self-signed certificate, user `test@maillet.example`, password `testpass`*. `seed.py otp` delivers a mail with a login code, `seed.py seen-first` marks a mail as read from "another client".

## Packaging & Releases

```bash
cargo install tauri-cli --version "^2" --locked
cargo tauri build --bundles deb      # → target/release/bundle/deb/
```

A package built locally only runs on systems with the same or a newer glibc. Official releases are therefore built by GitHub Actions on Ubuntu 22.04 ([`.github/workflows/release.yml`](.github/workflows/release.yml)):

1. Bump `version` in `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`.
2. Commit, then tag and push: `git tag v0.4.0 && git push origin v0.4.0`
3. The workflow tests, builds the `.deb`, and publishes a GitHub release with the package and `SHA256SUMS`.
4. Add the release notes: the workflow only writes the install section, since there are no pull requests to generate notes from. Put a *What's new* section on top, grouped by feature area as in earlier releases, and apply it with `gh release edit v0.4.0 --notes-file notes.md`.

## Screenshots

The images in `docs/images/` are rendered from `dist/` in headless Chrome, with demo data instead of the app ([`docs/screenshots/mock-tauri.js`](docs/screenshots/mock-tauri.js)). No real accounts, senders or codes end up in them. After UI changes, run:

```bash
python3 docs/screenshots/make.py                # all, light and dark
python3 docs/screenshots/make.py rule-editor    # single image
```

The demo data and the scenario for each image are in `mock-tauri.js`; size and page in `make.py`.

## Architecture

```
dist/                — Settings, popup and about pages (HTML/CSS/JS, no build step)
src-tauri/src/
├── main.rs          — App setup, tray menu, IPC commands, popup, reconnect loop
├── config.rs        — JSON config, keyring storage, legacy migration
├── rule_store.rs    — rules.json, account tags, migration from config.json
├── rules.rs         — Rule model and matching (sender, subject, code, link)
├── imap.rs          — IMAP IDLE loop, SSL/STARTTLS, two-stage fetch, MIME parsing, delete
├── api.rs           — Local HTTP API
├── status.rs        — Per-account connection status
├── log.rs           — In-memory log of checked mails and connection events (Log tab)
└── html.rs          — HTML-to-text conversion
```

| Component | Technology |
|---|---|
| Desktop framework | [Tauri v2](https://v2.tauri.app/) |
| Async runtime | [Tokio](https://tokio.rs/) |
| IMAP client | [async-imap](https://crates.io/crates/async-imap) |
| MIME parsing | [mail-parser](https://crates.io/crates/mail-parser) |
| TLS | native-tls (OpenSSL) |
| HTTP API | [axum](https://crates.io/crates/axum) |
| Secrets | [keyring](https://crates.io/crates/keyring) (Secret Service) |

## Rule Matching

| Field | Meaning |
|---|---|
| `sender` | Comma-separated globs (`*` = any text), case-insensitive. Empty = any sender |
| `subject` | Pattern: literal text, `*` = any text, whitespace = any whitespace, `{code}` = the code. Case-insensitive, matched anywhere in the subject |
| `body` | Code rules only, if the subject has no `{code}`: pattern containing `{code}`. Empty = first token matching `code_pattern` |
| `code_pattern` | Regex for the code itself (default `[A-Za-z0-9-]{4,12}`) |
| `link_prefix` | Link rules only: the first `http(s)://` URL starting with this (case-insensitive) is offered |

A rule needs a sender or a subject. Each account checks the rules tagged with its tag or `*`, in order; the first match wins. The full schema is in [docs/configuration.md](docs/configuration.md).

## Local HTTP API

Endpoints, token handling and a Tampermonkey example: [docs/api.md](docs/api.md). The server is in `src-tauri/src/api.rs`.
