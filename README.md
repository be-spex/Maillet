# Maillet

**Your mail, rethought: a quick look, a clear decision, an empty inbox.**

Maillet is a mail client that lives in the system tray and organises mail around what you do with it. See
what's new at a glance, turn a mail into a task and keep track of where it stands. One-time passwords appear
the moment they arrive.

Maillet does not replace your mail program yet: it reads, sorts and manages, but does not write mails.

**Website:** https://be-spex.github.io/maillet/ · part of [be spex](https://be-spex.github.io) – small, focused apps for Linux

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/images/maillet-home-dark.png">
  <img src="docs/images/maillet-home-light.png" width="700" alt="Maillet's overview: a fresh login code, a pinned mail and the new mails as cards, grouped into people and noise">
</picture>

## Install

### With APT (Debian, Ubuntu, Linux Mint)

Once, add the be spex repository:

```sh
curl -fsSLO https://be-spex.github.io/packages/be-spex-archive-keyring.deb
sudo apt install ./be-spex-archive-keyring.deb && sudo apt update
```

Then:

```sh
sudo apt install maillet
```

Updates arrive with `sudo apt upgrade`.

### AppImage

Download `Maillet_*_amd64.AppImage` from the [latest release](https://github.com/be-spex/Maillet/releases/latest),
make it executable (`chmod +x Maillet_*_amd64.AppImage`) and start it.

Maillet updates itself: it checks for a newer version and offers to replace the AppImage in place. That
update is built into Maillet and does not use zsync, so tools that rely on AppImageUpdate cannot update it.

An AppImage has no menu entry of its own. To get one, use an AppImage manager such as
[Gear Lever](https://flathub.org/apps/it.mijorus.gearlever),
[AppImageLauncher](https://github.com/TheAssassin/AppImageLauncher) or
[appimaged](https://github.com/probonopd/go-appimage). With APT you get it without extra tools.

### Other ways

Take the `.deb` from the same release and run `sudo apt install ./maillet_*_amd64.deb`.

To uninstall: `sudo apt remove maillet`. Your settings in `~/.config/maillet/` are kept.

Requirements: Ubuntu 24.04+, Linux Mint 22+ or Debian 13+ (x86_64).

## Features

- **One click away** – Maillet sits in the tray and opens as a compact window: all your mailboxes, the new
  mails first.
- **People before noise** – newsletters and automatic mails are set apart, so the mails written to you stand
  out.
- **Mails become tasks** – make a mail a task and move it from open to in progress to done. The state lives
  in folders on your mail server, so nothing is locked in.

  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/images/maillet-tasks-dark.png">
    <img src="docs/images/maillet-tasks-light.png" width="700" alt="Maillet's task board with mails as cards under Open, In progress and Done">
  </picture>
- **Pin, file, archive** – every mail gets its place with one click or one key. The aim is an empty inbox,
  not a full one.
- **One-time passwords** – login codes and magic links are recognised as they arrive and offered for copying
  or opening, without opening the mail. Rules for common services come from a built-in catalog; you can
  write and test your own.

  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/images/popup-code-dark.png">
    <img src="docs/images/popup-code-light.png" width="400" alt="Popup showing the code 482913 from a GitHub mail, with the buttons Copy and Copy and destroy">
  </picture>
- **All your mailboxes** – Gmail, iCloud, GMX, your company account, Proton Mail via the Bridge. New mail
  arrives at once (IMAP IDLE), without polling.
- **Private by default** – Maillet talks to your mail servers and nobody else. Passwords are kept in the
  system keyring.

## Usage

1. Start **Maillet** from your application menu. Its icon appears in the system tray.
2. Click the tray icon and choose **Settings**. Under **Accounts**, click **+ Add account** and enter your
   mail server, username and password. **Test connection** checks them before you save. For Gmail, Proton
   Mail and others, see [Setting up accounts](docs/accounts.md).
3. Click the tray icon to open Maillet: new mails come first. Read a mail, then decide: pin it, make it a
   task, file it or archive it.
4. For one-time passwords, open **Catalog** in the settings and click **+** next to the services you use, or
   create your own rule under **Rules**, see [Rules](docs/rules.md).

Day to day:

- **Code popup:** Enter or a click copies the code, Esc dismisses it. *Copy and destroy* also deletes the
  mail. Several codes at once are stacked, and each disappears after two minutes.
- **Attachments:** *View* opens an attachment with the program registered for its type, on a temporary copy
  that never lands in your Downloads folder. *Save* asks where to store it.
- **Tray menu:** *Check now* checks all mailboxes immediately; the menu also opens *Settings* and *About*.
- **Connection problems** show up on the account card and as a desktop notification. Maillet reconnects on
  its own.
- **No popup for a code?** The *Log* tab shows every checked mail and why each rule did or didn't match.
  See [Why wasn't a code detected?](docs/rules.md#why-wasnt-a-code-detected)

## Privacy & security

- **Everything stays on your computer.** Maillet talks only to your mail servers. The only other request
  loads the rule catalog from GitHub.
- **Codes are not stored.** The *Log* tab keeps sender, subject and rule results in memory only, with codes
  masked.
- **Viewed attachments** are read-only copies in `$XDG_RUNTIME_DIR` (memory, emptied at logout) and are
  removed when Maillet quits or starts, or after a day at the latest.
- **Your passwords stay safe.** They're kept in the system keyring (GNOME Keyring, KWallet), never in a plain
  text file.

## Documentation

- [Setting up accounts](docs/accounts.md): IMAP settings for common providers, Proton Mail Bridge, connection problems
- [Rules](docs/rules.md): code and link rules, patterns, catalog, sharing rules, why a code wasn't detected
- [Codes from your browser](docs/api.md): the local HTTP API for Tampermonkey and other scripts
- [Configuration files](docs/configuration.md): where settings, rules and passwords are stored
- [Rule catalog](catalog/README.md): format and how to contribute

## Build from source

See [DEVELOPMENT.md](DEVELOPMENT.md): building from source, tests, releases, architecture.

## License

MIT – see [LICENSE](LICENSE). © 2026 Bert Speckels. Maillet started as a fork of
[OTP Scout](https://github.com/Skrrytch/OTP-Scout).
