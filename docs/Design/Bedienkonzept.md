# Maillet – Bedienkonzept

Dieses Dokument beschreibt, **wie** Maillet bedient wird: die Grundhaltung, das mentale Modell, die Bausteine der Oberfläche und die Regeln, nach denen jede Funktion gestaltet wird. Welche Funktionen es konkret gibt, steht in [Funktionen.md](Funktionen.md). Wie es sich anfühlt, zeigt der [Click-Dummy](index.html).

---

## 1. Zielbild

> **Maillet macht aus dem Posteingang eine Durchgangsstation.** Jede Mail wird einmal angesehen, bekommt mit einer Geste ihren Platz und verlässt den Eingang wieder.

Klassische Mailprogramme sind Lager: Mails bleiben liegen, werden gelesen, wieder gelesen, irgendwann vergessen. Maillet ist eine **Sortierstation**: Es sieht, was hereinkommt, schlägt einen Platz vor und lässt den Menschen mit einem Klick entscheiden.

Maillet ist bewusst **kein vollständiger Mail-Client mit allem**. Es ist schlank, sitzt im Tray und kümmert sich um das, was im Alltag Zeit kostet: Sichten, Entscheiden, Wiederfinden.

---

## 2. Leitprinzipien

| # | Prinzip | Bedeutet in der Praxis |
|---|---|---|
| 1 | **Durchgangsstation statt Lager** | Ziel ist ein leerer Eingang. Jede Mail hat einen Ausgang. |
| 2 | **Wenige Verben, überall gleich** | Acht Aktionen. Dieselbe Taste tut in jeder Ansicht dasselbe. |
| 3 | **Vorschlagen, nicht bevormunden** | Maillet schlägt Thema, Frist oder Fakt vor. Ausgeführt wird erst nach einem Klick, außer der Nutzer hat selbst eine Regel angelegt. |
| 4 | **Rückgängig statt Nachfragen** | Keine „Sind Sie sicher?“-Dialoge. Jede Aktion lässt sich einige Sekunden lang zurücknehmen. |
| 5 | **Ruhe als Standard** | Nur Menschen und Codes dürfen stören. Alles andere wartet still. |
| 6 | **Drei Wege zu jeder Aktion** | Klick, Taste, Ziehen. Anfänger klicken, Geübte tippen, Visuelle ziehen. |
| 7 | **Schrittweise Tiefe** | Der erste Tag braucht kein Handbuch. Regeln, Kürzel und Ablage entdeckt man, wenn man sie braucht. |
| 8 | **Lokal und privat** | Keine Cloud-Dienste, keine Bilder aus dem Netz, keine Telemetrie. Daten liegen in offenen Formaten auf dem eigenen Rechner. |
| 9 | **Freude durch Leichtigkeit** | Spaß entsteht durch flüssige Bewegung und das gute Gefühl eines leeren Eingangs, nicht durch Punkte und Abzeichen. |
| 10 | **Fluss und Bestand trennen** | Was erscheint, um abgearbeitet zu werden, lebt woanders als das, was dauerhaft da ist. Der Fluss soll leer werden, der Bestand soll stabil bleiben. |
| 11 | **KI liest mit, der Mensch entscheidet** | KI fasst zusammen, bewertet und schlägt vor, nach Kriterien, die der Nutzer selbst als Prompts hinterlegt. Sie ist optional, sichtbar markiert und handelt nie ohne Klick oder ausdrückliche Regel. |
| 12 | **Wächst mit der Nutzung** | Maillet startet als schmales, effektives Werkzeug. Funktionen und Fensterbreite kommen dazu, wenn sie gebraucht werden. Das Potenzial ist sichtbar, aber nie aufgedrängt. |

---

## 3. Mentales Modell

### 3.1 Der Fluss einer Mail

```
            ┌───────────────────────────────────────────────┐
            │                  EINGANG                       │
            │  automatisch vorsortiert: Menschen · Maschinen │
            │  · Rauschen, Themen per Regel                  │
            └───────────────────────┬───────────────────────┘
                                    │  sichten (Übersicht oder Triage)
                                    ▼
          ┌──────────────── eine Entscheidung ────────────────┐
          │                                                   │
   ┌──────┴──────┐  ┌─────────────┐  ┌─────────────┐  ┌───────┴──────┐
   │  TUN        │  │  MERKEN     │  │  AUFBEWAHREN│  │  ERLEDIGT    │
   │  → Aufgabe  │  │  → Wissen   │  │  → Ablage   │  │  → Archiv    │
   └─────────────┘  └─────────────┘  └─────────────┘  └──────────────┘

   Nebenwege:  ⏲ Später (kommt wieder)   📌 Anpinnen (bleibt griffbereit)
```

Eine Mail kann mehrere Ausgänge nehmen: Aus derselben Mail entsteht eine Aufgabe **und** ein Fakt, danach wird sie abgelegt. Die Verbindung zur Mail bleibt immer erhalten.

### 3.2 Themen liegen quer

Themen sind keine Ordner, in die Mails verschwinden. Ein Thema ist ein **Arbeitsraum**, der alles zusammenzieht, was zu einem Projekt, Kunden oder Vorgang gehört: Mails, Aufgaben, Wissen, abgelegte Dateien.

```
                   Projekt Atlas
   ┌──────────┬───────────┬──────────┬──────────┐
   │ Mails    │ Aufgaben  │ Wissen   │ Ablage   │
   │ offen: 3 │ offen: 2  │ 4 Fakten │ 12 Mails │
   └──────────┴───────────┴──────────┴──────────┘
```

Eine Mail hat höchstens **ein** Thema. Das hält die Zuordnung schnell und die Übersicht klar. Wer feiner unterscheiden will, nutzt Schlagworte im Wissen oder Ordner in der Ablage.

### 3.3 Begriffe

| Begriff | Bedeutung |
|---|---|
| **Konto** | Ein Postfach. IMAP-Konten liegen auf einem Server, die **Ablage** ist ein lokales Konto aus Dateien. |
| **Gruppe** | Automatische Grobeinteilung jeder Mail: *Menschen*, *Maschinen*, *Rauschen*. Steuert Aufmerksamkeit. |
| **Thema** | Arbeitsraum für ein Projekt oder einen Vorgang. Farbe, Name, optional Regeln. |
| **Regel** | „Wenn Absender/Betreff/… dann Thema/Gruppe/Ablage/…“. Entsteht meist nebenbei beim Zuordnen. |
| **Aufgabe** | Etwas zu tun, mit Status (*Offen, In Arbeit, Warte auf …, Erledigt*) und optionaler Frist. Verknüpft mit ihrer Mail. |
| **Wissen** | Dauerhaft Gemerktes: **Fakten** (Name → Wert) und **Notizen** (Freitext). Mit Quelle. |
| **Ablage** | Lokales, dateibasiertes Mailkonto mit eigener Ordnerstruktur. Offline, dauerhaft, offenes Format. |
| **Anpinnen** | Mail bleibt sichtbar auf der Übersicht, unabhängig vom Eingang. |
| **Später** | Mail verschwindet und kommt zu einem Zeitpunkt oder Ereignis zurück. |
| **Fokus** | Modus, in dem nur Menschen benachrichtigen dürfen. |
| **Fluss** | Alles, was erscheint, um abgearbeitet zu werden und wieder zu verschwinden. |
| **Bestand / Board** | Alles, was dauerhaft sichtbar bleibt und sich höchstens aktualisiert. Vom Nutzer angeordnet. |
| **Zustandskarte** | Eine Karte im Bestand, die den aktuellen Stand eines Vorgangs zeigt (Ticket, Paket, Build, Rechnung). Mails, die nur einen neuen Zustand melden, aktualisieren sie, statt im Fluss zu erscheinen. |
| **Prompt** | Eine vom Nutzer hinterlegte Anweisung an die KI, mit Geltungsbereich, Auslöser und Ausgabeform. |

### 3.4 Fluss und Bestand

Nicht jede Anzeige hat denselben Zweck. Manche erscheinen, **um abgearbeitet zu werden**: eine neue Mail, ein Newsletter, eine Jira-Benachrichtigung, ein Login-Code. Andere sind **dauerhaft da**: das angepinnte Protokoll, der Stand eines Tickets, die Kostenstelle, die man täglich braucht. Werden beide gleich behandelt, entsteht genau das Durcheinander klassischer Postfächer: Wichtiges geht unter, Erledigtes bleibt liegen.

Maillet trennt deshalb zwei Zonen mit eigenen Regeln:

| | **Fluss** | **Bestand (Board)** |
|---|---|---|
| Zweck | erscheint, um abgearbeitet zu werden | bleibt, um nachzuschlagen oder zu beobachten |
| Beispiele | neue Mails, Newsletter, Bündel, Codes, Benachrichtigungen | Angepinntes, Zustandskarten, Spickzettel, Smart-Karten, Fälliges, Themen |
| Zielzustand | **leer** | **stabil und aktuell** |
| Anordnung | nach Wichtigkeit, von Maillet sortiert | feste Plätze, **vom Nutzer angeordnet** (räumliches Gedächtnis) |
| Verhalten | Karten verschwinden nach einer Aktion oder verfallen von selbst | Karten aktualisieren sich an ihrem Platz und verschwinden nur auf Wunsch |
| Zähler | ja: Badge, Fortschritt, „Ziel: 0“ | nein: Bestand erzeugt keinen Druck |
| Bewegung | hinein und hinaus animiert | still, eine Änderung leuchtet kurz auf |

**Übergänge zwischen den Zonen:**

- **Fluss → Bestand:** *Anpinnen* macht aus einer Mail einen Bestandsteil. *Merken* mit Stern legt einen Fakt auf den Spickzettel.
- **Zustandsmails werden zu Zustandskarten:** Mails, die nur einen neuen Stand melden („ATLAS-412 ist jetzt In Review“, „Ihr Paket ist in Zustellung“, „Build grün“), erscheinen nicht als neue Karte im Fluss. Sie aktualisieren eine Zustandskarte im Bestand und gelten als erledigt. Die Historie bleibt an der Karte.
- **Bestand → weg:** Erreicht eine Zustandskarte ihren Endzustand (zugestellt, Ticket geschlossen, Rechnung bezahlt), bietet sie an, sich zu entfernen.

**Verfall im Fluss:** Manche Anzeigen haben ein natürliches Ablaufdatum: ein Code nach Minuten, „Mittag heute?“ nach der Mittagszeit, eine Einladung nach dem Termin, ein Newsletter nach einer Woche (einstellbar). Solche Karten zeigen ihr Ablaufdatum dezent an und verlassen den Fluss von selbst. In der Bilanz erscheinen sie als „durch Zeitablauf erledigt“.

---

## 4. Ebenen der Oberfläche

Maillet hat vier Ebenen. Jede tiefere Ebene ist nur einen Klick entfernt, und `Esc` führt immer eine Ebene zurück.

| Ebene | Wann | Inhalt |
|---|---|---|
| **1. Tray-Icon** | immer | Zustand auf einen Blick: neutral, neue Mail von Menschen (Badge), Verbindungsproblem. |
| **2. Desktop-Benachrichtigung** | bei Ereignis | Absender, Betreff, bis zu drei Aktionen (*Öffnen, Aufgabe, Erledigt*). Erledigt Dinge, ohne Maillet zu öffnen. |
| **3. Maillet-Fenster** | Klick aufs Icon | **Ein** Fenster in drei Formen: Popup am Tray, Seitenleiste, Vollbild. Seine Breite wächst mit der Nutzung (siehe 4.3). Navigation links. |
| **4. Mail-Ansicht** | Klick auf eine Mail | Deckt das ganze Fenster ab, nur die Navigation bleibt sichtbar. `Esc` oder *Zurück* führt zur vorherigen Ansicht. |

**Dialoge** (Aufgabe anlegen, Merken, Ablegen) sind klein, haben sinnvolle Vorbelegung und lassen sich mit `Enter` bestätigen. **Popover** (Thema wählen, Später) öffnen direkt am Auslöser und brauchen keinen Bestätigungsknopf.

### 4.1 Die Ansichten des Hauptfensters

| Ansicht | Frage, die sie beantwortet | Darstellung |
|---|---|---|
| **Übersicht** | Was ist neu, was ist wichtig? | Karten, Filter nach Gruppe und Konto, rechts Angepinntes, Fälliges, Themen. |
| **Triage** | Was mache ich mit dieser einen Mail? | Eine Karte im Fokus, große Entscheidungsknöpfe, Fortschritt. |
| **Themen** | Was gehört zu Projekt X? | Kacheln, dann Arbeitsraum mit vier Spalten. |
| **Aufgaben** | Was ist zu tun, worauf warte ich? | Kanban mit vier Spalten. |
| **Wissen** | Wie war noch mal …? | Kartenwand mit Suche und Schlagworten. |
| **Ablage** | Wo ist das Original von …? | Ordnerbaum und Dateiliste, wie ein Dateimanager. |

**Listen gibt es nur, wo gesucht wird:** in der Suche und in der Ablage. Überall sonst zeigt Maillet Karten, weil Karten zum Entscheiden einladen und Listen zum Scrollen.

### 4.2 Die Übersicht: zwei Zonen

```
┌──────────────────────────────────────────┬──────────────────────────┐
│ FLUSS · leert sich                Ziel 0 │ BOARD          Anpassen  │
│ [Alle][Menschen][Maschinen][Rauschen]    │ ┌──────────────────────┐ │
│ Sortierung: Zeit | ✦ Wichtigkeit         │ │ 📌 Angepinnt         │ │
│ ┌──────────────────────────────────────┐ │ └──────────────────────┘ │
│ │ ✦ Newsletter-Radar: 1 von 2 relevant │ │ ┌──────────────────────┐ │
│ └──────────────────────────────────────┘ │ │ ◉ Zustand            │ │
│ ┌────────────┐ ┌────────────┐            │ │ ATLAS-412  In Review │ │
│ │ Karte      │ │ Karte      │            │ │ DHL-Paket  morgen    │ │
│ │ ✦ 5 · Bitte│ │ läuft ab   │            │ │ CI main    ✓ grün    │ │
│ └────────────┘ └────────────┘            │ └──────────────────────┘ │
│                                          │ ┌──────────────────────┐ │
│                                          │ │ ★ Spickzettel        │ │
│                                          │ │ KST 4711-ATL   ⧉     │ │
│                                          │ └──────────────────────┘ │
└──────────────────────────────────────────┴──────────────────────────┘
```

- **Links der Fluss.** Er zählt herunter und hat ein Ziel: leer.
- **Rechts das Board.** Seine Karten haben feste Plätze. *Anpassen* erlaubt Verschieben, Entfernen und Hinzufügen von Karten: Angepinnt, Zustand, Spickzettel, Heute dran, Themen, Smart-Karten (gespeicherte Suchen), Konten.
- **Ohne Board** (wer nichts anpinnt und keine Zusatzfunktion nutzt) gibt es nur den Fluss in voller Breite, ohne Zonen-Beschriftung.

### 4.3 Ein Fenster, das mitwächst

Maillet soll sich wie eine Tray-Anwendung anfühlen, nicht wie ein großes Mailprogramm. Deshalb **beginnt das Fenster schmal und wächst mit dem, was der Nutzer tatsächlich nutzt.**

**Grundsatz:** Wer keine Themen anlegt, nichts anpinnt und keine Mails in Aufgaben verwandelt, bekommt ein schmales Fenster, eher ein Menü: Navigation links, darunter vertikale Kacheln. Das Design ist dasselbe, nur reduziert. Wer Funktionen nutzt, bekommt Platz dafür.

| Stufe | Typische Nutzung | Breite (Popup) | Layout |
|---|---|---|---|
| **Kern** | nur Mails lesen und erledigen, Codes | ≈ 420 px | Navigation nur mit Symbolen, eine Kartenspalte, Fluss ohne Board |
| **Erweitert** | 1–2 Funktionen, z. B. Themen und Angepinntes | ≈ 640 px | Navigation mit Beschriftung, zwei Kartenspalten, Board unter dem Fluss |
| **Arbeitsplatz** | 3–4 Funktionen | ≈ 920 px | Fluss und Board nebeneinander |
| **Zentrale** | 5 und mehr | ≈ 1200 px | volle Darstellung wie im Vollbild |

**Wie Funktionen dazukommen:**
- **Durch Benutzen.** Wer eine Mail einem Thema zuordnet, in eine Aufgabe verwandelt, merkt oder ablegt, schaltet die Funktion damit ein. Die Navigation bekommt einen Eintrag, das Board eine Karte, das Fenster wird bei Bedarf breiter. Eine kurze Meldung sagt, was dazugekommen ist.
- **Bewusst einschalten** unter *Mehr*: eine Übersicht aller Funktionen mit Nutzen und Schalter.
- **Ausschalten** blendet nur aus, die Daten bleiben.
- Die Aktionen selbst (Aufgabe, Merken, Ablegen, Thema) sind immer erreichbar, in schmalen Fenstern hinter `⋯`. So ist das Potenzial sichtbar, ohne aufzudrängen.

**Responsiv auf jeder Stufe:** Jede Kachel und jedes Bedienelement richtet sich nach der **Fensterbreite**, nicht nach der Bildschirmbreite (technisch: Container-Abfragen). Das gilt schon bei wenig Inhalt:

| Fensterbreite | Anpassung |
|---|---|
| < 520 px | Navigation nur Symbole, Suche als Symbol, eine Kartenspalte mit einzeiliger Vorschau, Schnellaktionen reduziert plus `⋯`, Board-Karten einklappbar (ab der dritten eingeklappt), Kanban als Liste, Aktionsknöpfe der Mail-Ansicht nur mit Symbol |
| 520–899 px | Beschriftungen kommen zurück, zwei Kartenspalten, Board als Raster unter dem Fluss, Kanban zweispaltig |
| ≥ 900 px | Fluss und Board nebeneinander, alle Schnellaktionen sichtbar, Kanban vierspaltig |

**Drei Fensterformen** (Umschalter in der Kopfzeile, auch per Suche):

| Form | Verhalten | Breite |
|---|---|---|
| **Popup am Tray** | öffnet per Klick aufs Symbol, schließt bei Klick daneben | automatisch nach Nutzung |
| **Seitenleiste** | rechts angedockt, volle Höhe, bleibt offen | automatisch, innerhalb der eingestellten Grenzen (Standard 200–480 px); mehr Inhalt bedeutet mehr Scrollen und Einklappen |
| **Vollbild** | maximale Übersicht | ganze Bildschirmbreite |

Die Breite lässt sich jederzeit **von Hand** am linken Fensterrand ziehen. Dann gilt die manuelle Breite, bis der Nutzer auf *automatisch* zurückstellt.

**Grenzen der Seitenleiste:** Der Nutzer legt fest, wie schmal und wie breit die Seitenleiste werden darf, von **200 px bis zur vollen Bildschirmbreite**.
- Die Grenzen lassen sich in Pixeln oder **relativ zur Bildschirmbreite** angeben, z. B. „mindestens 240 px, höchstens 30 %“. So passt die Seitenleiste auf Laptop und großem Monitor gleichermaßen.
- Die automatische Breite wächst nur innerhalb dieser Grenzen. Auch Ziehen am Fensterrand bleibt innerhalb.
- Ist das Maximum die volle Bildschirmbreite, darf die Seitenleiste mit der Nutzung bis zur Zentrale-Stufe wachsen.

**Unter ≈ 300 px** wandert die Navigation als Symbolleiste nach oben, damit die Inhalte die ganze Breite nutzen. Schnellaktionen auf Karten entfallen, die Aktionen bleiben in der Mail-Ansicht erreichbar.

Auch mit viel Inhalt bleibt Maillet damit als kleines Popup nutzbar: Es scrollt mehr und klappt mehr ein, verliert aber keine Funktion.

---

## 5. Interaktionsmuster

### 5.1 Die acht Verben

Alle Bearbeitung läuft über dieselben acht Aktionen. Sie sind überall gleich benannt, gleich bebildert und haben dieselbe Taste.

| Taste | Aktion | Wirkung | Rückgängig |
|---|---|---|---|
| `E` | **Erledigt** | Archiviert auf dem Server. Nie Löschen. | ✓ |
| `A` | **Aufgabe** | Dialog mit vorbelegtem Titel und erkannter Frist. Archiviert die Mail danach (abwählbar). | ✓ |
| `M` | **Merken** | Dialog für Fakt oder Notiz. Vorbelegt aus Markierung oder Erkanntem. | ✓ |
| `L` | **Ablegen** | Ordnerwahl im lokalen Konto, Vorschlag aus dem Thema. Verschieben oder Kopie. | ✓ |
| `T` | **Thema** | Popover: tippen, Ziffer wählen, Enter legt neu an. | ✓ |
| `P` | **Anpinnen** | Umschalten. | ✓ |
| `S` | **Später** | Popover mit Zeitpunkten und Ereignissen. | ✓ |
| `R` | **Antworten** | Schlichter Editor, Entwurf liegt auf dem Server. | – |

### 5.2 Drei Wege zu jeder Aktion

1. **Klick:** Schnellaktionen erscheinen beim Überfahren einer Karte. In der Mail-Ansicht stehen sie als beschriftete Knöpfe.
2. **Taste:** Die acht Buchstaben wirken auf die gerade geöffnete oder fokussierte Mail.
3. **Ziehen:** Wird eine Karte gegriffen, fährt unten ein **Dock** mit allen Zielen herein: Themen, Anpinnen, Aufgabe, Merken, Ablegen, Später, Erledigt. Loslassen führt die Aktion aus.

### 5.3 Vorschläge

- Vorschläge sind **gestrichelt** und mit ✦ markiert. Sie unterscheiden sich sichtbar von Zuständen.
- Ein Klick übernimmt den Vorschlag. Ignorieren kostet nichts.
- Jeder Vorschlag kann sagen, **warum**: „Atlas im Betreff“, „Hansen ist schon dort“.
- Wird ein Vorschlag mehrfach übernommen, bietet Maillet an, eine **Regel** daraus zu machen.

### 5.4 Rückmeldung und Rückgängig

- Jede Aktion bestätigt sich mit einer kurzen Meldung unten in der Mitte, z. B. „Erledigt – liegt im Archiv des Servers“.
- Die Meldung trägt **Rückgängig** und, wo sinnvoll, einen Folgeschritt, z. B. „Immer: @bahn.example → Reisen“.
- Bestätigungsdialoge gibt es nur bei Aktionen, die sich nicht zurücknehmen lassen, etwa beim endgültigen Löschen aus der Ablage.

### 5.5 Bewegung mit Bedeutung

Animationen sind kurz (150–250 ms) und zeigen, **wohin** etwas gegangen ist: Eine erledigte Karte fliegt nach links, eine angepinnte nach oben, eine übersprungene nach rechts. Wer reduzierte Bewegung eingestellt hat, bekommt Überblendungen.

### 5.6 Leerzustände

Ein leerer Bereich ist ein Erfolg, kein Fehler. Leerzustände sind freundlich und ruhig („Nichts Neues hier. Genieß die Ruhe.“) und zeigen, wie der Bereich gefüllt wird („Hierher ziehen, was griffbereit bleiben soll“).

---

## 6. Aufmerksamkeit

### 6.1 Drei Lautstärken

| Stufe | Wer | Was passiert |
|---|---|---|
| **Laut** | Menschen, Codes (OTP), VIPs | Desktop-Benachrichtigung, Badge am Tray-Icon |
| **Leise** | Maschinen (Systeme, Bestellungen, Tickets) | Kein Ton, keine Benachrichtigung, zählt im Popup |
| **Stumm** | Rauschen (Newsletter, Werbung, soziale Netzwerke) | Gedämpft dargestellt, zählt nirgends mit |

Regeln können die Stufe einzelner Absender oder Themen ändern.

### 6.2 Fokus und Ruhezeiten

- **Fokus** (Taste im Kopf oder Tray-Menü): nur *Laut* bleibt laut, alles andere wird still eingereiht.
- **Ruhezeiten** nach Zeitplan, z. B. abends und am Wochenende für das Arbeitskonto.
- Beim Verlassen des Fokus zeigt Maillet, was in der Zwischenzeit kam: „Während des Fokus: 2 Menschen, 9 Maschinen.“

### 6.3 Tray-Icon

| Zustand | Darstellung |
|---|---|
| Alles ruhig | schlichtes Symbol |
| Neue Mail von Menschen | Symbol mit Zahl |
| Frischer Code | Symbol mit Schlüssel-Akzent, solange der Code gilt |
| Fokus aktiv | Symbol mit Mond |
| Verbindung gestört | Symbol mit Warnpunkt, Ursache im Popup |

---

## 7. Tastatur und Navigation

- **Aktionen:** `E A M L T P S R`, siehe 5.1.
- **Bewegen:** `J`/`K` oder Pfeiltasten zwischen Karten, `Enter` öffnet, `→` überspringt in der Triage.
- **Springen:** `G` und dann `Ü` (Übersicht), `R` (Triage), `T` (Themen), `A` (Aufgaben), `W` (Wissen), `L` (Ablage).
- **Suchen:** `Strg K` oder `/` öffnet die Suche mit Befehlen. Von dort ist alles erreichbar.
- **Zurück:** `Esc` schließt die oberste Ebene: Popover → Dialog → Mail-Ansicht → Unteransicht → Übersicht.
- **Hilfe:** `?` zeigt alle Kürzel.
- **Global:** ein frei belegbarer Systemkürzel öffnet das Tray-Popup von überall.

---

## 8. Gestaltung

| Thema | Richtlinie |
|---|---|
| **Formen** | Karten mit großzügigen Rundungen (12–20 px), weiche Schatten, klare Flächen. |
| **Farbe** | Neutraler Grund. Eine Akzentfarbe für Aktionen und Vorschläge. **Farbe trägt Bedeutung nur bei Themen.** Warnfarben nur für Fristen und Fehler. |
| **Typografie** | Systemschrift (Inter, Cantarell, Noto Sans). Lesetext in der Mail-Ansicht größer (15 px) mit großzügiger Zeilenhöhe, Breite begrenzt auf ≈ 65 Zeichen. |
| **Absender** | Initialen in Kreisen, Farbe aus dem Namen errechnet. Keine Bilder aus dem Netz. |
| **Dichte** | Zwei Stufen: *Komfortabel* (Standard) und *Kompakt* für kleine Bildschirme und viele Mails. |
| **Hell/Dunkel** | Folgt dem System. Akzentfarbe aus dem Desktop-Theme, wenn verfügbar. |
| **Zugänglichkeit** | Alle Aktionen per Tastatur, sichtbarer Fokus, Kontraste nach WCAG AA, Bewegung abschaltbar. |

---

## 9. Freude, ohne Spielerei

- **Gefühl von Fortschritt:** Fortschrittsbalken in der Triage, Bilanz am Ende („14 Mails in 4 Minuten: 6 erledigt, 4 Aufgaben, 2 gemerkt“).
- **Belohnende Leere:** ein ruhiger, schöner Leerzustand statt einer grauen Fläche.
- **Flüssigkeit:** Jede Aktion fühlt sich sofort an. Nichts wartet auf den Server, die Synchronisation läuft im Hintergrund.
- **Bewusst nicht:** Punkte, Ranglisten, Streaks mit schlechtem Gewissen. Maillet soll entlasten, nicht antreiben.

---

## 10. KI-Assistenz

Bei großen Mailmengen hilft eine KI beim **Lesen und Bewerten**: Sie fasst zusammen, filtert Massen nach den eigenen Interessen und erkennt Bitten, Fristen und Fragen. Die Bedienung folgt festen Regeln, damit sie hilft, ohne Kontrolle zu nehmen.

### 10.1 Prompts sind die Regeln der KI

Der Nutzer hinterlegt **eigene Prompts** in einer Bibliothek. Jeder Prompt hat:

| Eigenschaft | Beispiel |
|---|---|
| **Name** | „Newsletter-Radar“ |
| **Anweisung** | „Mich interessieren Rust, Tauri und Linux-Desktop. Nenne nur Beiträge dazu, je ein Satz.“ |
| **Geltungsbereich** | Gruppe *Rauschen*, ein Konto, ein Thema, ein Absender |
| **Auslöser** | bei Eingang, gebündelt (z. B. zweimal täglich), beim Öffnen, nur auf Knopfdruck |
| **Ausgabeform** | Kurzfassung, Ja/Nein mit Begründung, Punktzahl 1–5, Liste (z. B. Fragen, Fristen) |

So entsteht KI-Unterstützung, die zu den eigenen Kriterien passt, statt einer allgemeinen „Wichtig“-Markierung, deren Gründe niemand kennt.

### 10.2 Bedienregeln

1. **Optional und einzeln abschaltbar.** Ein Hauptschalter, dazu jeder Prompt einzeln.
2. **Sichtbar getrennt.** KI-Ausgaben stehen in einem eigenen, getönten Band mit ✦. Sie werden nie mit dem Originaltext vermischt.
3. **Nachvollziehbar.** Jede Ausgabe hat *Warum?*: welcher Prompt, welches Modell, welche Daten gingen an die KI.
4. **Ordnen, nicht verstecken.** Eine KI-Bewertung kann den Fluss sortieren und markieren, aber sie blendet keine Mail aus und archiviert nichts von selbst.
5. **Handeln nur mit Klick.** Vorschläge der KI (Aufgabe mit Teilaufgaben, Fakt, Thema) folgen denselben Regeln wie alle Vorschläge: gestrichelt, ein Klick übernimmt. Eine automatische Aktion gibt es nur, wenn der Nutzer sie ausdrücklich als Regel anlegt, und auch dann mit *Rückgängig*.
6. **Lokal bevorzugt.** Standard ist ein lokales Modell (z. B. über Ollama). Ein eigener API-Schlüssel ist möglich, dann zeigt Maillet deutlich, dass Daten den Rechner verlassen.
7. **Sperrbereiche.** Konten, Themen oder Absender lassen sich vollständig von der KI ausschließen (z. B. Personal, Privat). Gesperrte Mails tragen einen dezenten Hinweis.
8. **Datensparsam.** Gesendet wird nur, was der Prompt braucht, z. B. Betreff und die ersten Absätze. Die Ablage ist standardmäßig ausgeschlossen.

---

## 11. Plattform-Leitplanken

Maillet läuft auf GNOME, KDE Plasma, Cinnamon, XFCE und weiteren Desktops und nutzt dafür nur Standards:

| Bereich | Standard | Hinweis |
|---|---|---|
| Tray | StatusNotifierItem / AppIndicator | GNOME braucht die Erweiterung *AppIndicator*. Fehlt sie, startet Maillet mit Fenster und Hinweis. |
| Benachrichtigungen | freedesktop Notifications mit Aktionen | überall verfügbar |
| Passwörter | Secret Service | GNOME Keyring, KWallet |
| Autostart | XDG Autostart | überall |
| Ablage | Maildir im Home-Verzeichnis | mit jedem Dateimanager und Mailprogramm lesbar |

---

## 12. Bewusst nicht

- **Kein Ordnerbaum im Eingang.** Ordner gibt es nur in der lokalen Ablage.
- **Keine Label-Flut.** Ein Thema pro Mail, Schlagworte nur im Wissen.
- **Keine Kategorien auf dem Server.** Maillet verändert das Server-Postfach nur durch Archivieren, Gelesen-Status und Entwürfe.
- **Keine KI als Voraussetzung.** Die Grundfunktionen laufen über Regeln und Muster. KI ist eine Zusatzschicht, lokal bevorzugt, nie Pflicht.
- **Kein Mischen von Fluss und Bestand.** Was dauerhaft gebraucht wird, steht nicht zwischen dem, was abgearbeitet werden soll.
- **Kein Ersatz für Kalender oder Projektmanagement.** Maillet übergibt an solche Werkzeuge, statt sie nachzubauen.
