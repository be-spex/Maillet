# Maillet – Aktionen an Mails

Konzept für eine flexible Werkzeugleiste pro Mail. Ergänzt das [Bedienkonzept](Bedienkonzept.md), Abschnitt 5. Wie es aussieht, zeigt [aktionen.html](aktionen.html).

---

## 1. Ausgangslage

Die Zahl der Aktionen wächst: Anpinnen, Erledigt, Gelesen, Noise, Aufgabe, Info, Löschen. Bald kommen Später, Antworten und KI-Vorschläge dazu. Die Werkzeugleiste der Karte hat aber nur wenige Plätze, und Nutzer arbeiten verschieden:

- **Durchgangsstation:** Jede Mail verlässt den Eingang (archivieren, ablegen, löschen). „Gelesen“ braucht hier niemand.
- **Lager:** Mails bleiben im Eingang, gelesen oder ungelesen. Ablegen ist selten.

Eine feste Leiste passt zu keinem der beiden richtig. Jede neue Aktion als eigener Knopf macht die Karte voll und unruhig.

---

## 2. Grundidee: drei Arten von Aktionen

Aktionen unterscheiden sich darin, **was mit der Mail passiert**. Das ordnet die Leiste.

| Art | Wirkung | Aktionen |
|---|---|---|
| **Ausgänge** | Die Mail verlässt den Eingang. | Erledigt, Aufgabe, Ablegen (in eine Kategorie), Löschen |
| **Markierungen** | Die Mail bleibt, ihr Zustand ändert sich. | Anpinnen, Gelesen/Ungelesen, Noise/Person |
| **Werkzeuge** | Etwas Neues entsteht, die Mail bleibt. | Antworten, Später, KI (künftig) |

Die L-Form der Karte bekommt damit eine Bedeutung:

- **Waagerecht oben: Ausgänge.** Wohin geht die Mail?
- **Senkrecht rechts: Markierungen.** Wie steht es um die Mail?

---

## 3. Kategorien statt fester Ordner

**Empfehlung: ja, frei konfigurierbare Kategorien.** „Info“ wird die erste, vorbelegte Kategorie, keine eigene Aktion mehr.

Eine Kategorie ist ein Ablageziel:

| Feld | Beispiel | Hinweis |
|---|---|---|
| Name | Info | Anzeige in Maillet |
| Ordner | `Info` | Standard ist der Name; wählbar ist auch ein vorhandener Ordner des Kontos |
| Farbe | Blau | Punkt auf dem Knopf und in der Liste |
| Symbol | Buch | aus einer kleinen Auswahl |
| Taste | `1` | Ziffern 1–9 in der Reihenfolge der Liste |

Regeln:

- **Ordnernamen sind englisch,** unabhängig von der Sprache der Oberfläche: `Tasks`, `Info`, `Receipts`, `Travel`. So lesen sie sich in jedem Mailprogramm gleich. Ein vorhandener Ordner kann trotzdem gewählt werden, z. B. `Rechnungen`.
- **Der Ordner wird beim ersten Ablegen angelegt,** bei Proton automatisch unter `Folders/`.
- **Aufgaben bleiben eine eigene Aktion.** Sie haben einen Ablauf (Open, In Progress, Done) und ein Board. Eine Kategorie ist dagegen nur ein Ort.
- **Erledigt bleibt eine eigene Aktion.** Das Archiv ist der Standard-Ausgang für alles, was keinen besonderen Platz braucht.
- Vorschläge beim ersten Einrichten: Info, Receipts, Travel. Nichts davon ist Pflicht.

---

## 4. Die Werkzeugleiste der Karte

```
                 ┌──────────────────────────────┬────┐
  Jana Wolff     │ ● Info  │ Ablegen ▾ │ ☐ │ ✓  │ ☆  │   ← Ausgänge
  Work · 14:40   └──────────────────────────────┤    │
  Atlas: Review der Spezifikation               │ 👁 │   ← Markierungen
  Könntest du bis Freitag …                     │ 🔇 │
                                                └────┘
```

- **Erledigt** sitzt immer ganz rechts in der Ecke: der häufigste Ausgang an der festen Stelle.
- Daneben **Aufgabe**, dann **Ablegen ▾**.
- **Zuletzt benutzte Kategorie als Direktknopf:** Wer zehn Rechnungen hintereinander ablegt, klickt zehnmal auf „● Receipts“ und nicht jedes Mal auf „Ablegen ▾“ und dann auf das Ziel. Der Knopf zeigt Farbpunkt und Name.
- **Höchstens vier Ausgänge und drei Markierungen** sind direkt sichtbar. Was der Nutzer zusätzlich eingeschaltet hat, liegt hinter `⋯` am Ende der waagerechten Leiste.
- Was ausgeschaltet ist, erscheint nirgends (Einstellung „Mail actions“).
- Die Leiste erscheint beim Überfahren oder bei Tastaturfokus, wie bisher.

---

## 5. Der Ablage-Wähler

Ein Klick auf **Ablegen ▾** (oder die Taste `F`) öffnet ein kleines Popover direkt am Knopf:

```
  ┌────────────────────────────┐
  │ 🔍 Kategorie …             │
  ├────────────────────────────┤
  │ 1  ● Info                  │
  │ 2  ● Receipts              │
  │ 3  ● Travel                │
  │ 4  ● Projekt Atlas         │
  ├────────────────────────────┤
  │ +  Neue Kategorie …        │
  └────────────────────────────┘
```

- **Feste Reihenfolge** (wie in den Einstellungen), damit die Ziffern immer gleich bleiben.
- **Tippen filtert,** Enter nimmt den ersten Treffer, Ziffer wählt direkt, Esc schließt.
- **„Neue Kategorie …“** legt sie mit dem getippten Namen an, sofort benutzbar. Farbe und Symbol lassen sich später ändern.
- Nach dem Ablegen: Meldung „In Receipts abgelegt“ mit **Rückgängig**.

---

## 6. Die Mail-Ansicht

In der Mail-Ansicht ist Platz für Beschriftungen. Die Reihenfolge entspricht der Karte:

```
  [ ✓ Done ]  [ ☐ Task ]  [ ● Info │ ▾ ]   ☆  👁  🗑  ⋯
```

- **Erledigt** als Hauptknopf (Akzentfarbe).
- **Ablegen als geteilter Knopf:** links die zuletzt benutzte Kategorie (ein Klick), rechts `▾` für den Wähler.
- Markierungen und seltene Aktionen nur als Symbole, mit Tooltip.

---

## 7. Tastatur

Englische Buchstaben, weil die Oberfläche englisch ist. Sie wirken auf die geöffnete oder fokussierte Mail.

| Taste | Aktion |
|---|---|
| `E` | Done (archivieren) |
| `T` | Task |
| `F` | File: Ablage-Wähler öffnen, dann `1`–`9` |
| `1`–`9` | direkt in Kategorie 1–9 ablegen |
| `P` | Pin |
| `U` | Read / Unread |
| `N` | Noise / Person |
| `Del` | Delete |
| `Z` | Undo |

---

## 8. Einstellungen

„Mail actions“ wird eine eigene Seite **Actions** mit zwei Teilen:

1. **Quick actions:** Schalter je Aktion, Reihenfolge per Ziehen. Darüber eine **Live-Vorschau einer Karte** mit der Leiste, wie sie gleich aussieht.
2. **Categories:** Liste zum Anlegen, Umbenennen, Umsortieren (bestimmt die Ziffern), Farbe, Symbol, Ordner (neuer Name oder vorhandener Ordner aus einer Liste des Kontos).

---

## 9. Später

- **Ziehen:** Wird eine Karte gegriffen, erscheinen die Kategorien als Dock (Bedienkonzept 5.2).
- **Vorschläge:** Maillet merkt sich, wohin Mails eines Absenders gehen, und zeigt die Kategorie gestrichelt als Vorschlag. Nach mehrfachem Übernehmen: „Immer so ablegen?“ als Regel.
- **Kategorie-Ansicht:** Ein Eintrag in der Navigation zeigt die abgelegten Mails einer Kategorie.

---

## 10. Umsetzung in Schritten

| Schritt | Inhalt |
|---|---|
| 1 | Kategorien als Datenmodell (Info vorbelegt), Ablage-Wähler, Aktion „File“ ersetzt „Info“ |
| 2 | Leiste nach Ausgängen und Markierungen geordnet, zuletzt benutzte Kategorie als Direktknopf, `⋯` für den Überlauf |
| 3 | Tastenkürzel |
| 4 | Einstellungsseite „Actions“ mit Live-Vorschau und Kategorie-Editor |
| 5 | Ziehen, Vorschläge, Kategorie-Ansicht |

**Stand:** Schritte 1–4 sind umgesetzt. Abweichungen: Umsortieren in den Einstellungen mit ↑/↓ statt Ziehen; Kategorien haben eine Farbe, aber noch kein Symbol.
