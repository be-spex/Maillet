# Maillet – Design-Konzept

Maillet überwacht Postfächer und zeigt Mails so, wie man sie wirklich braucht: als fokussierte Karten statt endloser Listen. Es sitzt im System-Tray, meldet sich nur, wenn es etwas zu tun gibt, und läuft auf GNOME, KDE, Cinnamon, XFCE und anderen Desktops.

Ziel: **einfach und schlicht, aber effektiv.** Jedes Element muss eine Frage beantworten: *Was muss ich jetzt tun?*

---

## 1. Leitprinzipien

1. **Karten statt Listen.** Listen gibt es nur dort, wo der Nutzer aktiv sucht.
2. **Fokus vor Übersicht.** Immer nur ein Vorgang gleichzeitig im Vordergrund.
3. **Erledigen statt lesen.** OTP, Bestätigung, Rechnung: der Nutzer soll zum Ergebnis kommen, nicht zur Mail.
4. **Ruhe als Standard.** Nur Dinge, die Aufmerksamkeit verdienen, erzeugen Benachrichtigungen.
5. **Tastatur first, Maus gleichwertig.** Jede Aktion hat einen Shortcut.
6. **Privat by default.** Keine Remote-Inhalte, keine externen Dienste, keine Tracker-Pixel.

---

## 2. Plattform und Technik

Die Technikwahl folgt den Anforderungen: moderne, elegante UI, Start in der Tray-Leiste, Laufen auf allen gängigen Linux-Desktops, kein Python.

### Empfehlung: Tauri 2 (Rust-Backend, Web-Frontend)

| Aspekt | Begründung |
|---|---|
| **UI** | HTML/CSS/TypeScript mit einem Komponenten-Framework (Svelte empfohlen, schlank und schnell). Volle Freiheit für Animationen, Glasmorphismus, Dark/Light, Blur. |
| **Backend** | Rust. IMAP IDLE, Keyring, Parsing und Cache laufen dort. Die bestehende OTP-Scout-Logik (`imap.rs`, `rules.rs`, `config.rs`) ist bereits Rust und wird wiederverwendet. |
| **Ressourcen** | Ein WebKitGTK-Fenster, das nur bei Bedarf sichtbar ist. Im Hintergrund läuft nur der Rust-Prozess, typischerweise wenige Dutzend MB. |
| **Plattformen** | Linux (GNOME, KDE, Cinnamon, XFCE). Später Windows und macOS ohne Umbau möglich. |
| **Pakete** | `.deb`, AppImage, Flatpak, `.rpm`. |

**Alternative: Qt 6 (C++ oder Rust mit cxx-qt).** Native KDE-Optik, sehr gute Tray-Unterstützung, aber schwerere Entwicklung der Oberfläche und mehr Aufwand für Animationen. Empfehlenswert, falls native KDE-Integration wichtiger ist als Designfreiheit.

**Nicht empfohlen:** GTK4 (wie im Vorschlag), weil es GNOME-zentriert ist und Gestaltung mit CSS stark eingeschränkt ist. Außerdem war Python ausdrücklich nicht gewünscht.

### Desktop-Integration

| Funktion | GNOME | KDE Plasma | Cinnamon / XFCE / MATE |
|---|---|---|---|
| Tray-Icon | **Braucht** die Erweiterung *AppIndicator and KStatusNotifierItem Support* (sonst kein Tray) | Nativ (StatusNotifierItem) | Nativ |
| Benachrichtigungen | Freedesktop-Standard (`org.freedesktop.Notifications`) | ✓ | ✓ |
| Passwörter | Secret Service (GNOME Keyring) | Secret Service (KWallet bietet es an) | ✓ |
| Autostart | XDG-Autostart-Eintrag | ✓ | ✓ |
| Globaler Shortcut | Über Portal oder Desktop-Einstellung | ✓ | ✓ |

**Hinweis GNOME:** Ohne die AppIndicator-Erweiterung ist kein Tray-Icon sichtbar. Maillet muss dann einen Fallback bieten: Start-Fenster mit Hinweis und Link zur Erweiterung, oder Benachrichtigung als Einstieg.

---

## 3. Informationsarchitektur

Drei Ebenen, die sich wie ein Stapel verhalten:

```
Tray-Klick
   │
   ▼
┌─────────────────────┐   Klick auf Karte    ┌──────────────────────┐
│ Dashboard           │ ───────────────────▶ │ Drilldown (Stapel)   │
│ OTP · Konten · Gruppen│                    │ eine Mail im Fokus   │
└─────────────────────┘ ◀─────── Esc ─────── └──────────────────────┘
   │
   │  tippen
   ▼
┌─────────────────────┐
│ Suche (Liste)       │  Löschen des Begriffs → zurück zum Dashboard
└─────────────────────┘
```

Einstellungen sind eine eigene Ebene und öffnen sich als Seitenpanel, nicht als verschachtelter Dialog.

---

## 4. Die Oberfläche

### 4.1 Dashboard (Tray-Popup, ca. 420 × 640 px, skalierbar)

```
┌──────────────────────────────────────────┐
│ Maillet                    ◐  ⚙  ⌕       │
├──────────────────────────────────────────┤
│ ✦ FRISCHE CODES                          │  ← nur sichtbar, wenn vorhanden
│ ┌──────────────────┐ ┌─────────────────┐ │
│ │ GitHub           │ │ Spotify         │ │
│ │ 492 104          │ │ 771 330         │ │
│ │ ⏱ 1:48 · Enter   │ │ ⏱ 0:31 · Enter  │ │
│ └──────────────────┘ └─────────────────┘ │
├──────────────────────────────────────────┤
│ ┌────────────────┐ ┌────────────────┐    │
│ │ Privat    12   │ │ Büro        3  │    │
│ │ ● ● ●          │ │ ● ●  ●         │    │
│ │ vor 2 Min      │ │ vor 6 Min      │    │
│ └────────────────┘ └────────────────┘    │
│ ┌────────────────┐ ┌────────────────┐    │
│ │ ♥ Menschen   2 │ │ ⚙ Maschinen  7 │    │
│ │ Anna · Max     │ │ Amazon · DHL   │    │
│ └────────────────┘ └────────────────┘    │
│ ┌────────────────┐                       │
│ │ ≋ Rauschen  48 │  ← gedämpft           │
│ └────────────────┘                       │
├──────────────────────────────────────────┤
│  ⌕  Suchen … Name, Datum, Stichwort      │
└──────────────────────────────────────────┘
```

- **Raster:** CSS Grid mit `auto-fit`. Die Karten brechen selbst um, wenn das Fenster breiter wird.
- **Leerzustand:** Ohne frische Codes verschwindet der OTP-Bereich komplett.
- **Gedämpftes Rauschen:** Die Karte „Rauschen“ ist optisch zurückgenommen und zählt nicht zur Gesamtzahl.

### 4.2 Drilldown: der Mail-Stapel

```
┌──────────────────────────────────────────┐
│ ‹ Privat                     3 von 12    │
├──────────────────────────────────────────┤
│                                          │
│   Anna Berger                    14:02   │
│   Re: Samstag Grillen?                   │
│   ──────────────────────────────────     │
│   Klar, ich bring den Salat mit.         │
│   Sag Bescheid, ob Max auch kommt…       │
│                                          │
│   ▸ Vollansicht                          │
│                                          │
├──────────────────────────────────────────┤
│   ↩ Antworten   ✓ Archiv   ⏲ Später     │
└──────────────────────────────────────────┘
        ░░░░░░░░░░░░░░░░░░░░░░░░░░░░    ← Stapel-Rand dahinter
        ░░░░░░░░░░░░░░░░░░░░░░░░░░░░
```

- **Stapel:** Die oberste Karte ist voll sichtbar, darunter sind die nächsten Karten als Kanten angedeutet.
- **Wisch-Geste:** Nach links archivieren, nach rechts später lesen, nach oben antworten.
- **Archivieren** verschiebt die Mail in den Archivordner des Servers, sie wird nicht gelöscht.
- **Nach der letzten Karte** erscheint eine Ruhe-Anzeige: „Alles erledigt. 👌“ mit Rückweg zum Dashboard.

### 4.3 Suche

```
┌──────────────────────────────────────────┐
│ ⌕ rechnung                          ✕    │
├──────────────────────────────────────────┤
│ Amazon   Rechnung 2026-1187       gestern│
│ Büro     Rechnung Q3, Stadtwerke   Mo   │
│ Privat   Rechnung Kfz-Versicherung  12.10│
│ …                                        │
└──────────────────────────────────────────┘
```

- Die Liste erscheint nur bei aktiver Suche.
- Ein Filter-Chip pro Feld: `von:`, `an:`, `Konto:`, `Gruppe:`, `vor:`, `hat:anhang`.
- Löschen des Begriffs blendet die Liste aus und zeigt wieder das Dashboard.

### 4.4 Mail lesen (Vollansicht)

Auf Wunsch öffnet sich die Mail in einer Lesespalte mit max. 680 px Textbreite. Typografie: große Lesegröße, weiche Zeilenhöhe. HTML-Mails werden **sandboxed** dargestellt (siehe Abschnitt 8).

---

## 5. Ideen für besondere Usability

### 5.1 OTP-Schnellzugriff
- **Ein Klick kopiert, Enter kopiert, Esc verwirft.**
- **Auto-Kopieren optional:** Bei einem frischen Code kopiert Maillet ihn bei Klick auf die Benachrichtigung in die Zwischenablage.
- **Ablauf-Timer:** Ein dünner Ring um den Code zeigt die verbleibende Zeit.
- **Archiv statt Löschen:** Die Mail wandert ins Archiv, nicht in den Papierkorb. Löschen ist eine bewusste Zweitaktion (`Shift+Klick`).
- **Mehrfach-Codes:** Mehrere Codes stehen nebeneinander, jeder mit eigenem Timer.

### 5.2 Intelligente Gruppen
Maillet ordnet jede Mail einer Gruppe zu, mit dieser Priorität:

1. **Regeln des Nutzers** (z. B. „Absender enthält @amazon → Maschinen“). Haben immer Vorrang.
2. **Kontakte:** Absender aus dem Adressbuch → *Menschen*.
3. **Heuristik:** Listen-Header (`List-Unsubscribe`, `Precedence: bulk`), typische Muster in Absender und Betreff → *Rauschen* oder *Maschinen*.
4. **Rest:** → *Menschen*, damit nichts Wichtiges verloren geht.

Der Nutzer kann jede Zuordnung mit einer Taste korrigieren. Die Korrektur wird als Regel gelernt. Die Zuordnung läuft **lokal**, ohne Cloud-KI.

### 5.3 Fokus-Modus und Ruhezeiten
- **Fokus-Modus** (`Strg+Alt+F` oder Tray-Menü): Nur Menschen-Mails erzeugen Benachrichtigungen. Der Rest sammelt sich still.
- **Zeitplan:** z. B. Mo–Fr 9–17 Uhr Fokus, Wochenende aus.
- **Sprint-Triage:** Ein Timer (5 Min) bietet an, den Stapel abzuarbeiten. Am Ende steht eine Zusammenfassung: „14 Mails erledigt in 4:12 Min.“

### 5.4 Später lesen (Snooze)
IMAP kennt kein Snooze. Maillet speichert den Zeitpunkt lokal und zeigt die Mail danach wieder oben im Stapel. Der Server bleibt unverändert, also gibt es keine Spuren auf dem Server.

### 5.5 Konten-Karte mit Lebenszeichen
- **Ungelesen-Zahl** als große Ziffer.
- **Sendermarken:** Die drei letzten Absender als Kreise mit Initialen. Die Farbe kommt aus einem Hash des Namens, also keine Bilder aus dem Netz.
- **Statuszeile:** „Live · vor 2 Min“, „Verbindung wird wiederhergestellt…“ oder „Anmeldung fehlgeschlagen“ mit Direktlink zu den Einstellungen.

### 5.6 Tastatur-Modus
| Taste | Aktion |
|---|---|
| `J` / `K` oder `↓` `↑` | nächste / vorherige Karte |
| `E` | archivieren |
| `S` | später lesen |
| `R` | antworten |
| `Enter` | OTP kopieren / Mail öffnen |
| `/` | Suche |
| `Esc` | eine Ebene zurück |
| `Strg+Alt+M` | Maillet global öffnen (Shortcut muss vom Desktop bereitgestellt werden) |

### 5.7 Tray-Icon als Anzeige
- Das Icon zeigt die Zahl der ungelesenen Menschen-Mails als Badge.
- Farbe zeigt den Zustand: neutral, aktiv (neue Mail), Warnung (Verbindung weg).
- Ohne Badge bleibt das Icon schlicht, damit es nicht stört.

### 5.8 Benachrichtigungen mit Aktion
Eine Benachrichtigung hat Schaltflächen: *Öffnen*, *Archivieren*, bei OTP *Kopieren*. Der Nutzer erledigt Dinge, ohne das Popup zu öffnen.

### 5.9 Themes
Hell, dunkel, folgt dem System. Akzentfarbe aus dem Desktop-Theme, wenn verfügbar (KDE und GNOME stellen sie bereit).

---

## 6. Datenmodell und Synchronisation

- **Lokaler Cache** in SQLite: Mail-Metadaten (Absender, Betreff, Datum, Gruppe, Status, Snooze-Zeit), keine Mail-Texte standardmäßig.
- **Mail-Texte** werden on demand vom Server geladen und nur bei Wunsch dauerhaft gespeichert (Einstellung „Offline-Lesen“).
- **IMAP IDLE** für Echtzeit, Wiederverbindung wie in OTP Scout.
- **Lokale Zustände** (Snooze, Gruppe, Fokus) liegen in der eigenen Datenbank. Aktionen, die den Server betreffen (Archiv, Gelesen), werden gegen den Server ausgeführt.
- **Konfliktregel:** Der Server gewinnt, lokale Zustände sind nur Ergänzung.

---

## 7. Die Einstellungen

Seitenleiste statt verschachtelter Dialoge:

| Bereich | Inhalt |
|---|---|
| **Konten** | Server, Verschlüsselung, Ordner, Verbindungsstatus |
| **Gruppen** | Regeln, Kontakte, Zuordnung korrigieren |
| **Benachrichtigungen** | Fokus, Zeitplan, Ton, Badge |
| **Tastatur** | Shortcuts anpassen |
| **Darstellung** | Theme, Akzent, Schriftgröße |
| **Protokoll** | Verbindungen und Fehler |

Die OTP-Regeln aus OTP Scout gehen in den Bereich *Gruppen* und *Codes* über.

---

## 8. Datenschutz und Sicherheit

Das ist ein Kernversprechen, nicht Beiwerk.

- **Keine Remote-Inhalte** in Mails. Bilder und Tracker werden blockiert, einzeln freigebbar pro Absender.
- **Keine Avatare aus dem Netz.** Gravatar oder Favicons würden Absender und Nutzer verraten. Sendermarken sind lokal erzeugt.
- **HTML-Mails** laufen in einer isolierten Ansicht ohne Skript, ohne Netzzugriff, ohne Zugriff auf die App.
- **Links** werden vor dem Öffnen angezeigt (Ziel-Domain sichtbar). Tracking-Parameter werden entfernt.
- **Passwörter** nur im Keyring, nie in Klartext-Dateien.
- **Telemetrie:** keine.

---

## 9. Zustände, die man sieht

| Zustand | Darstellung |
|---|---|
| Leer, alles erledigt | Ruhige Illustration, Satz „Nichts Dringendes.“ |
| Erstes Start | Assistent: Konto hinzufügen, Gruppen erklären, Tray-Hinweis für GNOME |
| Verbindung weg | Gedämpfte Statuszeile, kein Alarm |
| Fehler | Karte mit Lösungsvorschlag, nie nur ein Fehlercode |
| Laden | Skeleton-Karten statt Spinner |

---

## 10. Phasen

**Phase 1 – Kern (MVP)**
- Tray, Popup, Konten-Karten, Stapel-Ansicht, Archiv, Suche
- OTP-Schnellzugriff (übernommen aus OTP Scout)
- Tastatur-Modus, Hell/Dunkel

**Phase 2 – Intelligenz**
- Gruppen mit Regeln und Heuristik, Korrektur-Lernen
- Snooze, Fokus-Modus, Zeitpläne
- Benachrichtigungs-Aktionen

**Phase 3 – Feinschliff**
- Sprint-Triage mit Auswertung
- Akzentfarben, Themes, Offline-Lesen
- Flatpak, AppImage, `.rpm`; Windows- und macOS-Builds

---

## 11. Offene Fragen

1. **Mailtext offline speichern:** standardmäßig aus oder an? (Empfehlung: aus, Einstellung vorhanden)
2. **Antworten:** nur Verweis auf den Mail-Client, oder eigener Editor? (Empfehlung: Phase 1 einfacher Editor, Antwort als Entwurf auf dem Server)
3. **Gruppen-Start:** Heuristik ab Tag 1, oder erst nach Korrekturen? (Empfehlung: Heuristik mit sichtbarer Erklärung „Warum hier?“)
4. **Technik final:** Tauri 2 (Empfehlung) oder Qt 6?
5. **OTP-Scout:** als Modul in Maillet weiterführen oder als eigenes Produkt pflegen?
