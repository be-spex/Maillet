# Maillet – Projektziel

Maillet ist ein Fork von [OTP Scout](https://github.com/Skrrytch/OTP-Scout) und wird zu einem leichtgewichtigen E-Mail-Programm umgebaut, das im System-Tray sitzt. Es vereint drei Rollen in einer App:

1. **Tray-Anwendung:** läuft dauerhaft im Hintergrund, ohne eigenes Hauptfenster beim Start.
2. **Benachrichtigungszentrale:** meldet neue Mails über IMAP IDLE sofort per Popup bzw. Desktop-Benachrichtigung.
3. **Mailbetrachter:** Postfächer, Ordner und Mails lesen, öffnen per Klick aus dem Tray-Menü.

## Herkunft aus OTP Scout

Übernommen wird die Grundlage, die schon funktioniert:

- IMAP-Verbindung mit IDLE und Wiederverbindung (`src-tauri/src/imap.rs`)
- Konten und Passwörter im System-Keyring (`src-tauri/src/config.rs`)
- Tray-Icon, Popup-Fenster, Benachrichtigungen (`src-tauri/src/main.rs`)
- Protokoll-Log (`src-tauri/src/log.rs`)

Die OTP-spezifischen Teile (Regeln, Katalog, lokale API) bleiben vorerst erhalten, werden aber nicht mehr zentral. Ob sie ganz entfallen oder als optionales Modul weiterleben, ist offen.

## Geplante Schritte

- [ ] Mailbetrachter: Ordnerliste und Mailliste pro Konto, Lesen der Mail-Texte (Text und sicher gerenderte HTML-Ansicht)
- [ ] Mail-Cache und Synchronisation (Entscheidung: nur Metadaten oder vollständige Mails)
- [ ] Tray-Menü mit ungelesenen Mails pro Konto
- [ ] Benachrichtigungen konfigurierbar (Absender, Ordner, Ruhezeiten)
- [ ] Umbenennung der Icons und des Pakets endgültig (Icon ist noch das OTP-Scout-Icon)

## Hinweis zum Regelkatalog

Der Katalog (`catalog/`) liegt noch im Repository von OTP Scout. Maillet lädt ihn von dort. Eine eigene Katalog-Quelle folgt, sobald OTP-spezifische Funktionen entschieden sind.
