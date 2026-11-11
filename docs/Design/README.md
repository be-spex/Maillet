# Maillet – Click-Dummy

Ein klickbarer Entwurf für die Bedienung von Maillet. Er zeigt, wie sich die App anfühlen soll, nicht wie sie technisch gebaut wird. Alle Daten sind fiktiv, nichts wird gespeichert, nichts verlässt den Browser.

**Öffnen:** `index.html` im Browser öffnen (Doppelklick genügt). Keine Installation, keine Abhängigkeiten.

**Dazu gehören:**
- [Bedienkonzept.md](Bedienkonzept.md) – Grundhaltung, mentales Modell, Ebenen, Interaktionsmuster, Gestaltung
- [Funktionen.md](Funktionen.md) – Ideen-Pool mit Nutzen, Kern, Erweiterungen und Priorisierung

## Leitidee

Im Büroalltag ist eine Mail selten nur eine Nachricht. Meist ist sie eins von vier Dingen:

| Die Mail ist … | Maillet macht daraus … | Wo es landet |
|---|---|---|
| etwas zu tun | eine **Aufgabe**, verknüpft mit der Mail | Aufgaben (Kanban) |
| etwas zu merken | einen **Fakt** oder eine **Notiz** | Wissen |
| etwas zum Aufbewahren | eine **Datei** im lokalen Konto | Ablage |
| erledigt oder Rauschen | nichts – **Erledigt** räumt sie weg | Archiv des Servers |

Der Posteingang wird so zur Durchgangsstation statt zum Lager. **Themen** halten alles zusammen, was zu einem Projekt oder Kunden gehört.

## Was man ausprobieren sollte

0. **Wachsendes Fenster:** Der Dummy zeigt einen Desktop. Maillet sitzt als Popup oben rechts am Tray-Symbol (M). Unten links ist die **Dummy-Steuerung**:
   - **Nutzerprofil** *Einsteiger* (≈ 420 px, nur Übersicht und Triage), *Fortgeschritten* (Themen und Angepinntes, ≈ 640 px), *Power* (alles, ≈ 1200 px).
   - Als Einsteiger eine Mail in eine Aufgabe verwandeln, einem Thema zuordnen oder anpinnen: Navigation und Fenster wachsen mit. *Mehr* in der Navigation schaltet Funktionen bewusst ein und aus.
   - **Fensterform** Popup, Seitenleiste, Vollbild. Den linken Fensterrand ziehen ändert die Breite; alles reagiert auf die Fensterbreite.
   - Eine geöffnete Mail deckt das Fenster ab, nur die Navigation bleibt.

1. **Übersicht:** Neue Mails als Karten, gefiltert nach *Menschen / Maschinen / Rauschen* und Konto. Rechts: Angepinntes, heute Fälliges, Themen, Konten.
2. **Ziehen:** Eine Mail-Karte greifen – unten erscheint ein **Dock** mit allen Zielen (Themen, Anpinnen, Aufgabe, Merken, Ablegen, Später, Erledigt). Loslassen = erledigt.
3. **Themen zuordnen:**
   - gestrichelter Vorschlag auf der Karte (✦ *Projekt Atlas?*) – ein Klick übernimmt ihn,
   - oder `#`-Chip bzw. Taste `T` → tippen, `1–9` wählen, Enter legt ein neues Thema an,
   - oder Karte auf ein Thema ziehen.
   - Nach dem Zuordnen fragt Maillet: *„Immer: @bahn.example → Reisen?“* Ein Klick macht daraus eine Regel. So lernt die Gruppierung nebenbei.
4. **Mail öffnen:** Unter *Erkannt* stehen Fristen, Telefonnummern, Daten und *Name: Wert*-Zeilen aus dem Text. Ein Klick macht daraus eine Aufgabe mit Termin oder einen Wissens-Fakt. **Text markieren** öffnet eine Blase *Merken / Aufgabe*.
5. **Triage** (`⚡`): Eine Mail nach der anderen, große Entscheidungsknöpfe, alles per Tastatur (`E A T P M L S →`). Am Ende eine kleine Bilanz.
6. **Aufgaben:** Kanban mit *Offen, In Arbeit, Warte auf …, Erledigt*. *Warte auf* zeigt, seit wann der Ball beim anderen liegt, mit *Nachfassen*. Karten zwischen Spalten ziehen.
7. **Wissen:** Fakten (mit Kopierknopf) und Notizen, Schlagworte, Quelle bleibt verlinkt.
8. **Ablage:** Das lokale, dateibasierte Konto (Maildir unter `~/Maillet/Ablage`). Ordner frei anlegen, Dateiname sichtbar, immer offline.
9. **Suche** (`Strg K` oder `/`): eine Suche über Posteingang, Archiv, Ablage, Aufgaben, Wissen, Themen und Befehle. Filter wie `von:hansen` oder `thema:atlas`.
10. **Fluss und Board:** Die Übersicht hat zwei Zonen. Links der **Fluss** (zählt herunter, Ziel 0), rechts das **Board** (bleibt, drängt nicht). *Anpassen* verschiebt, entfernt und ergänzt Board-Karten, z. B. *Ich schulde Antworten* oder eine Smart-Karte. Manche Fluss-Karten zeigen „läuft ab …“.
11. **Zustandskarten:** *Simulieren → Jira-Statuswechsel* mehrmals ausführen. Die Karte *ATLAS-412* aktualisiert sich, im Fluss erscheint nichts Neues. Ein Klick auf die Karte zeigt die Historie.
12. **Spickzettel:** Im Wissen einen Fakt mit ☆ markieren, er erscheint auf dem Board. Ein Klick kopiert ihn.
13. **KI-Assistenz:**
    - Karten tragen eine Wichtigkeit (1–5) und „Bitte an dich“. *Wichtigkeit* im Fluss sortiert danach.
    - Oben im Fluss fasst das *Newsletter-Radar* das Rauschen nach deinem Prompt zusammen.
    - Die Mail von Malte Hansen öffnen: KI-Band mit Kurzfassung, Kundenstimmung und *Aufgabe mit 3 Teilaufgaben*. *Warum?* zeigt Prompt, Modell und gesendete Daten.
    - Gesperrte Bereiche (Team & HR, Privat) zeigen „KI gesperrt“.
    - Navigation *KI*: Hauptschalter, Modellwahl, eigene Prompts anlegen und an einer Mail testen, Sperrbereiche.
    - Suche (`Strg K`) mit einer Frage, z. B. „Wer gibt bei Nordwind frei?“ → Antwort mit Quellen.
14. **Oben rechts:** *Simulieren* (neue Mail, Kundenmail mit Regel, Jira-Status, OTP-Code, Newsletter), *Fokus* (nur Menschen dürfen stören), *Tray-Popup*, Hell/Dunkel, Tastenkürzel.

## Bewusste Entscheidungen im Entwurf

- **Erledigt statt Löschen.** Die Hauptaktion archiviert, sie löscht nicht.
- **Rückgängig überall.** Jede Aktion hat für einige Sekunden *Rückgängig*. Deshalb gibt es keine Bestätigungsdialoge.
- **Aufgabe archiviert die Mail** (abwählbar). Die Mail bleibt über die Aufgabe erreichbar.
- **Später** kennt auch *„Wenn keine Antwort kommt (3 Tage)“*, für Mails, auf die man selbst wartet.
- **Rauschen** löst nie eine Benachrichtigung aus und ist optisch gedämpft. Ein Klick erledigt alles Rauschen auf einmal.
- **Gleiche Taste, gleiche Aktion** in Übersicht, Mail-Ansicht und Triage.
- **Keine Bilder aus dem Netz.** Absender werden als Initialen gezeigt, die Farbe ist aus dem Namen abgeleitet.

## Nicht enthalten

Antworten und Verfassen, Kontoeinrichtung, Einstellungen, echte HTML-Mails. Diese Teile sind nur angedeutet. Die KI-Ergebnisse sind vorbereitete Beispieltexte, es läuft kein Modell.
