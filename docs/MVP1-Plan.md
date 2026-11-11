# MVP 1 – kurzer Plan

**Ziel:** Maillet prüft Postfächer, zeigt die Zahl neuer Mails im Tray und öffnet per Klick ein Fenster mit den neuen Mails im Design des Click-Dummys. Die OTP-Erkennung bleibt, ist aber nur noch eine Funktion unter vielen. Basis sind die [Design-Unterlagen](Design/README.md) und der [Click-Dummy](Design/index.html) – iterativ, ohne fertiges Konzept.

---

## 1. Funktionen im MVP

| Drin | Bewusst noch nicht |
|---|---|
| IMAP-Konten (vorhandene Kontoverwaltung, Keyring) | Themen, Aufgaben, Wissen, Ablage, KI |
| Beim Start: ungelesene Mails im Posteingang laden (neueste 200) | Triage, Board, Zustandskarten |
| Danach Echtzeit per IMAP IDLE, Abgleich, wenn Mails anderswo gelesen werden | Wachsendes Fenster, Seitenleiste, Vollbild |
| **Tray: Zähler neuer Mails** im Symbol und im Tooltip | Antworten, HTML-Darstellung |
| **Klick → Maillet-Fenster** mit der Liste neuer Mails | Mehrere Ordner je Konto |
| Mail öffnen und lesen (Text) | |
| *Erledigt* (archivieren), *Gelesen*, *Jetzt prüfen* | |
| OTP wie bisher (Regeln, Katalog, API, Code-Popup), Codes zusätzlich als Kachel im Fenster | |

---

## 2. Architektur

Im Mittelpunkt steht der **Mail-Kern**: prüfen, merken, anzeigen. Funktionen wie OTP hängen sich an.

```
          ┌──────────── Frontend ────────────┐
          │ Maillet-Fenster · Einstellungen · │
          │ Code-Popup (OTP)                  │
          └───────────────▲──────────────────┘
                          │ Befehle / Ereignisse
┌─────────────────────────┴─────────────────────────┐
│ app/       Tray (Zähler), Fenster, Befehle         │
├────────────────────────────────────────────────────┤
│ features/  otp/ … später weitere                   │  ← hängen sich an
├────────────────────────────────────────────────────┤
│ mail/      Wächter (IDLE) · Befehlsverbindung ·    │  ← Kern
│            MailStore · Ereignisse                  │
├────────────────────────────────────────────────────┤
│ core/      Konfiguration, Keyring, Protokoll       │
└────────────────────────────────────────────────────┘
```

**Ablauf einer neuen Mail**

1. Der **Wächter** (eine Dauerverbindung je Konto, IDLE) meldet neue UIDs und holt die Header.
2. Der **MailStore** nimmt die Mail auf und sendet `MailEvent::New`.
3. Die **Funktionen** werten sie aus. Braucht eine den Text, wird er einmal geladen. Ergebnisse hängen als *Findings* an der Mail (z. B. OTP: Code).
4. Die **App** aktualisiert den Tray-Zähler und schickt `mail:new` ans Fenster. Die OTP-Funktion öffnet zusätzlich ihr Code-Popup.

**Bausteine**

| Modul | Aufgabe | Herkunft |
|---|---|---|
| `mail/imap/` | Verbinden (TLS/STARTTLS), Wächter mit Reconnect, Befehlsverbindung für Text laden, gelesen, archivieren | aus `imap.rs` herausgelöst |
| `mail/model.rs`, `parse.rs` | Mail-Zusammenfassung (Name, Adresse, Betreff, Datum, Flags, Vorschau), Text | neu bzw. aus `imap.rs` |
| `mail/store.rs`, `events.rs` | aktuelle Mails je Konto, Ereignisse per `broadcast` | neu |
| `mail/state.rs` | zuletzt gesehene UID speichern, damit nach Neustart nichts doppelt gemeldet wird | neu |
| `features/mod.rs` | Schnittstelle `Feature` (`wants_body`, `analyse` → Findings) und Pipeline | neu |
| `features/otp/` | Regeln, Katalog, lokale API, Code-Popup | `rules.rs`, `rule_store.rs`, `catalog.rs`, `api.rs`, `code.html` |
| `app/tray.rs` | Symbol mit Zähler, Tooltip, Menü | aus `main.rs` |
| `app/windows.rs`, `commands/` | Maillet-Fenster, Einstellungen, Befehle `list_mails`, `get_body`, `set_seen`, `archive`, `check_now` | neu bzw. aus `main.rs` |

```rust
#[async_trait]
pub trait Feature: Send + Sync {
    fn id(&self) -> &'static str;                         // "otp"
    fn wants_body(&self, mail: &MailSummary) -> bool { false }
    async fn analyse(&self, mail: &MailSummary, body: Option<&MailBody>) -> Vec<Finding>;
}
```

**Entscheidungen für den MVP** (änderbar):
- **Frontend:** HTML/CSS/JS ohne Build-Schritt, CSS und Bausteine direkt aus dem Click-Dummy. Umstieg auf Svelte, wenn das Fenster wächst.
- **Einstellungen:** Das vorhandene Einstellungsfenster bleibt (Konten, Codes).
- **Hauptaktion:** *Erledigt* archiviert auf dem Server (Ordner mit `\Archive`, sonst einstellbar).

---

## 3. Design und Look

Grundlage ist die **Kern-Stufe** des Click-Dummys: schmal, ruhig, Karten statt Liste.

**Tray**
- Symbol mit **Zahl neuer Mails** als Badge, zur Laufzeit ins Icon gezeichnet. Ohne neue Mails das schlichte Symbol.
- Tooltip: „Maillet – 3 neue Mails (Arbeit 2, Privat 1)“.
- Menü: **Maillet öffnen**, Jetzt prüfen, Einstellungen, Beenden.

**Fenster** (≈ 420 × 720 px, rahmenlos, abgerundet, öffnet sich am Tray-Symbol per Linksklick, schließt bei Klick daneben)

```
┌──┬───────────────────────────────┐
│M │ ⌕            ⟳  ◐             │  Kopf: Suche (noch ohne Funktion),
│  ├───────────────────────────────┤  Jetzt prüfen, Hell/Dunkel
│⌂ │ Guten Morgen.                 │
│  │ 3 neue Mails                  │
│  │ ┌ ✦ CODES ─────────────────┐  │  nur wenn ein Code da ist (OTP)
│  │ │ GitHub   492 104   ◔  ⧉  │  │
│  │ └──────────────────────────┘  │
│  │ NEU            Arbeit · Privat│
│  │ ┌──────────────────────────┐  │
│  │ │▌JW Jana Wolff     09:14  │  │  Karte: Initialen-Avatar, Absender,
│  │ │  Atlas: Review der Spec  │  │  Konto · Zeit, Betreff, 2 Zeilen
│  │ │  Hallo, anbei die …   ✓  │  │  Vorschau; beim Überfahren ✓ und ●
│  │ └──────────────────────────┘  │
│⚙ │ ● Arbeit live · ● Privat live │  Status der Konten
└──┴───────────────────────────────┘
```

- **Navigation links** (60 px, nur Symbole): Übersicht, unten Einstellungen.
- **Mail öffnen:** Die Mail-Ansicht deckt das Fenster ab, nur die Navigation bleibt. Oben *Zurück*, dann Absender, Betreff, Text; Aktionen *Erledigt*, *Gelesen/Ungelesen*. `Esc` führt zurück.
- **Leerzustand:** „Nichts Neues. Genieß die Ruhe.“
- **Fehler:** Statuszeile des Kontos wird rot mit kurzer Ursache, Klick öffnet die Kontoeinstellungen.
- **Look:** Farben, Rundungen, Schrift und Abstände aus dem Click-Dummy. Hell und dunkel folgen dem System. Akzentfarbe Violett, Codes mit leichtem Verlauf.

---

## 4. Vorgehen

Jeder Schritt ist für sich lauffähig.

1. **Umbau:** Module wie oben, OTP als erste Funktion. OTP verhält sich danach exakt wie vorher, alle Tests grün.
2. **Mail-Kern:** Erstsynchronisation, Abgleich, MailStore, gespeicherter Stand, Tray-Zähler. Test: Zähler stimmt mit dem Webmail überein.
3. **Fenster:** Liste, Mail-Ansicht, Erledigt/Gelesen, Codes-Kachel, Status. Test im Alltag mit eigenen Konten.
4. **Paket:** Texte, Version 0.5.0, `.deb`.

---

## 5. Linux-Hinweise

| Thema | Lage | Umgang |
|---|---|---|
| **Klick aufs Tray-Symbol** | Tauris eigenes Tray (libappindicator) meldet keinen Linksklick. **Getestet am 11.11.2026 unter Cinnamon/X11:** Ein eigenes Tray-Symbol mit `ksni` (StatusNotifierItem direkt über D-Bus) empfängt Linksklick (`activate`), Mittelklick (`secondary_activate`) und Menüeinträge; Tooltip und ein ins Symbol gezeichneter Zähler funktionieren und aktualisieren sich. | Tray mit `ksni` statt mit Tauri bauen. Linksklick öffnet das Fenster, Rechtsklick das Menü (*Maillet öffnen* bleibt als Rückfall). KDE und GNOME noch testen. |
| **Zähler am Symbol** | Text neben dem Symbol zeigen nicht alle Desktops | Zahl ins Icon zeichnen (mit `ksni` bestätigt) |
| **Fensterposition** | Der Klick liefert die Bildschirmkoordinaten des Symbols mit (im Test: Panel unten rechts). Unter Wayland bestimmt der Compositor die Position. | Unter X11 das Fenster **am Symbol** öffnen, egal ob das Panel oben oder unten ist; unter Wayland, wo der Compositor es platziert |
