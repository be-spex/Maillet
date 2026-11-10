# Maillet – Funktionen und Ideen-Pool

Dieses Dokument sammelt die Funktionen, die bei der täglichen Mailarbeit helfen. Jede Funktion beginnt beim **Problem** aus dem Alltag, beschreibt den **Nutzen**, den **Kern** und mögliche **Erweiterungen**. Am Ende steht eine Priorisierung.

Die Bedienregeln, denen alle Funktionen folgen, beschreibt [Bedienkonzept.md](Bedienkonzept.md).

**Kennzeichnung:** 🟢 MVP · 🟡 Ausbau · 🔵 Vision

---

## Übersicht

| Bereich | Leitfrage | Funktionen |
|---|---|---|
| **A. Ordnen** | Wie kommt Struktur in den Eingang, ohne dass ich sortiere? | Gruppen, Regeln, Themen, Bündel, Newsletter-Zentrale |
| **B. Bearbeiten** | Was wird aus einer Mail? | Aufgabe, Wissen, Termin, Kontakt, Delegieren, Schnellantwort, Notiz |
| **C. Aufbewahren** | Wo lebt eine Mail, die ich behalten will? | Ablage als lokales Mailkonto |
| **D. Themen** | Wie behalte ich ein Projekt im Blick? | Arbeitsräume, Abschluss, Übergabe |
| **E. Aufmerksamkeit** | Wann darf Maillet stören? | Fokus, Ruhezeiten, Briefing, Tagesabschluss |
| **F. Wiederfinden** | Wie finde ich etwas wieder? | Eine Suche, Smart-Karten, Personen |
| **G. Freude** | Warum macht das Spaß? | Triage, Bilanz, Rückblick |
| **H. Anbindung** | Wie spielt Maillet mit anderen Werkzeugen zusammen? | Kalender, Aufgaben, Notizen, Dateimanager |
| **I. Bestand** | Was ist immer da, ohne zu drängen? | Board, Zustandskarten, Spickzettel, Smart-Karten |
| **J. KI-Assistenz** | Wie bewältige ich große Mengen? | Eigene Prompts, Radar, Bewertung, Kurzfassung, Fragen an die Mails |

Grundlage für I und J ist die Unterscheidung von **Fluss** (erscheint, um abgearbeitet zu werden) und **Bestand** (ist dauerhaft da) sowie die Bedienregeln für KI, beide beschrieben im [Bedienkonzept](Bedienkonzept.md#34-fluss-und-bestand).

---

## A. Ordnen – Struktur ohne Sortierarbeit

### A1 Automatische Gruppen: Menschen · Maschinen · Rauschen 🟢

**Problem:** 80 Mails am Tag, davon 15 von echten Menschen. Die wichtigen gehen zwischen Jira, Newslettern und Bestellbestätigungen unter.

**Nutzen:** Der Blick fällt sofort auf das, was ein Mensch geschrieben hat. Maschinen und Rauschen stören nicht mehr.

**Kern:**
- Jede Mail bekommt automatisch eine von drei Gruppen.
- Erkennung über Header (`List-Unsubscribe`, `Precedence: bulk`, `Auto-Submitted`, `noreply`-Absender), bekannte Kontakte und frühere Antworten.
- Die Gruppe steuert die Lautstärke (siehe E1).
- Korrektur mit einem Klick, z. B. „Ist ein Mensch“. Die Korrektur gilt künftig für den Absender.

**Erweiterungen:**
- 🟡 **Direkt vs. CC:** Mails, in denen ich nur in CC stehe, werden gedämpft gezeigt.
- 🟡 **VIP-Absender:** immer laut, auch im Fokus (Chefin, wichtigster Kunde).
- 🟡 **„Erstkontakt“-Hinweis:** Absender, die zum ersten Mal schreiben, sind markiert. Das schützt auch vor Phishing.
- 🔵 **Einstufung per KI** (opt-in) für Grenzfälle, siehe J3.

---

### A2 Regeln: Mails automatisch zuordnen 🟢

**Problem:** Mails vom selben Kunden oder System landen immer wieder im selben Thema. Jede manuelle Zuordnung kostet Klicks.

**Nutzen:** Wiederkehrendes ordnet sich selbst. Der Nutzer pflegt keine Regelwerke, sie entstehen nebenbei.

**Kern:**
- **Regeln entstehen beim Zuordnen.** Nach „Mail → Kunde Nordwind“ fragt Maillet: *„Immer: @nordwind.de → Kunde Nordwind?“*. Ein Klick legt die Regel an und wendet sie auf vorhandene Mails an.
- **Bedingungen:** Absender, Domain, Empfänger (an welche meiner Adressen), Betreff enthält, Mailingliste (`List-Id`), hat Anhang.
- **Aktionen:** Thema setzen, Gruppe ändern, Lautstärke ändern, anpinnen.
- **„Warum hier?“:** Jede automatisch zugeordnete Mail trägt den Hinweis *auto*. Ein Klick zeigt die Regel.

**Erweiterungen:**
- 🟡 **Vorschau beim Anlegen:** „Diese Regel betrifft 23 vorhandene Mails“ mit Liste.
- 🟡 **Weitere Aktionen:** automatisch *später* (z. B. Reports erst am Freitag), automatisch *erledigt* (Bestätigungen, die niemand liest), automatisch *ablegen* (Rechnungen → `Ablage/Rechnungen/{Jahr}`), automatisch *Aufgabe* (z. B. Jira „Dir zugewiesen“).
- 🟡 **Regel-Pflege:** Regeln, die nie greifen, schlägt Maillet zum Löschen vor. Widersprüche werden angezeigt.
- 🟡 **Vorlagen:** Katalog wie in OTP Scout, z. B. „Jira-Benachrichtigungen“, „Rechnungen gängiger Anbieter“, „GitHub“.
- 🔵 **Regeln aus Mustern vorschlagen:** „Du hast 5 Mails von @hostwerk.example zu Rechnungen gezogen. Regel anlegen?“

---

### A3 Themen-Vorschläge 🟢

**Problem:** Für neue Absender gibt es noch keine Regel, aber das Thema ist offensichtlich.

**Nutzen:** Die Zuordnung ist ein Klick auf einen Vorschlag statt eine Suche in einer Liste.

**Kern:**
- Vorschlag aus Stichworten im Betreff („Atlas“ → Projekt Atlas), aus dem Verlauf (gleiche Konversation, gleicher Absender) und aus Empfängern (dieselben Leute wie im Thema).
- Gestrichelter Chip auf der Karte: ✦ *Projekt Atlas?*. Ein Klick übernimmt ihn.

**Erweiterungen:**
- 🟡 **Konversationen erben das Thema:** Antworten in einem zugeordneten Verlauf landen automatisch im selben Thema.

---

### A4 Bündel für Maschinen 🟡

**Problem:** Sieben Jira-Mails, vier Build-Meldungen, drei Kalender-Updates am Vormittag. Jede einzeln ist wertlos, zusammen sind sie Lärm.

**Nutzen:** Ein Bündel statt vierzehn Karten. Ein Klick erledigt alle.

**Kern:**
- Mails desselben Systems werden zu einer Karte gebündelt: „Jira · 7 Updates“.
- Aufgeklappt zeigt das Bündel eine kompakte Liste. Einzelne Mails lassen sich herausziehen.
- *Erledigt* auf dem Bündel archiviert alle.

**Erweiterungen:**
- 🟡 **Zusammenfassung aus Struktur:** „3 Tickets dir zugewiesen, 4 Kommentare“ (aus dem Betreff abgeleitet, ohne KI).
- 🔵 **Zustellung im Takt:** Bündel erscheinen nur zur vollen Stunde oder zweimal am Tag.

---

### A5 Konversationen 🟡

**Problem:** Ein Thread mit acht Antworten erscheint als acht Mails.

**Nutzen:** Eine Karte pro Gespräch, die neueste Antwort oben, die Historie aufklappbar.

**Kern:** Bündelung über `In-Reply-To`/`References`. Zitierte Textteile werden eingeklappt.

---

### A6 Newsletter-Zentrale 🟡

**Problem:** Newsletter sind manchmal interessant, aber nie dringend. Sie verstopfen den Eingang.

**Nutzen:** Rauschen hat einen eigenen, ruhigen Ort. Abbestellen dauert eine Sekunde.

**Kern:**
- Alle Rauschen-Mails gesammelt, gruppiert nach Absender, mit Häufigkeit.
- **Abbestellen-Knopf** über `List-Unsubscribe` (ohne den Link im Browser öffnen zu müssen, wo möglich).
- „Alles Rauschen erledigen“ mit einem Klick.

**Erweiterungen:**
- 🟡 **Lesestapel:** ausgewählte Newsletter in einen „Lesen am Freitag“-Stapel.
- 🟡 **Statistik:** „Diesen Newsletter hast du seit 3 Monaten nicht geöffnet. Abbestellen?“

---

### A7 Codes und Login-Links 🟢

**Problem:** Login-Codes per Mail: Mailprogramm öffnen, warten, suchen, kopieren.

**Nutzen:** Der Code steht sofort im Tray und in der Übersicht. Ein Klick kopiert ihn und räumt die Mail weg.

**Kern:** Übernommen aus OTP Scout: Erkennung per Regeln und Katalog, Popup, Countdown, *Kopieren und erledigen*.

**Erweiterungen:**
- 🟡 **Code in der Benachrichtigung** mit Knopf *Kopieren*.
- 🟡 **Lokale API** für Codes aus anderen Quellen (vorhanden).

---

### A8 Verfallende Anzeigen 🟡

**Problem:** „Mittag heute?“ ist um 15 Uhr wertlos, eine Einladung nach dem Termin auch. Trotzdem bleiben solche Mails liegen und müssen einzeln weggeräumt werden.

**Nutzen:** Der Fluss räumt sich teilweise selbst auf.

**Kern:**
- Erkennbares Ablaufdatum: Codes (Minuten), Tagesbezug („heute“, „12:30“), Einladungen (Terminende), Angebote mit Frist.
- Die Karte zeigt dezent „läuft ab 14:00“ und verlässt den Fluss danach von selbst, mit Eintrag in der Bilanz.
- Pro Gruppe einstellbar, z. B. „Newsletter nach 7 Tagen automatisch erledigt“.

**Erweiterungen:**
- 🟡 **Kurz vor Ablauf leicht hervorheben**, falls noch eine Reaktion möglich ist.

---

## B. Bearbeiten – was aus einer Mail wird

Die vier Hauptausgänge *Aufgabe, Wissen, Ablage, Erledigt* decken das meiste ab. Im Büroalltag kommen weitere hinzu. Die Antwort auf „Was noch?“:

| Ausgang | Wann | Funktion |
|---|---|---|
| **Aufgabe** | Ich muss etwas tun | B1 |
| **Wissen** | Ich muss mir etwas merken | B2 |
| **Termin** | Ein Datum betrifft mich | B3 |
| **Kontakt** | Eine Person ist neu oder relevant | B4 |
| **Delegieren** | Jemand anderes soll es tun | B5 |
| **Schnellantwort** | Eine kurze Reaktion genügt | B6 |
| **Notiz an der Mail** | Ich will mir zur Mail etwas vermerken | B7 |
| **Anhang sichern** | Die Datei ist wichtiger als die Mail | B8 |
| **Anpinnen** | Ich brauche sie heute noch öfter | B9 |
| **Später** | Jetzt nicht, aber bestimmt | B10 |
| **Antwort schulden** | Jemand wartet auf mich | B11 |
| **Zustandskarte** | Die Mail meldet nur einen neuen Stand | I2 |
| **KI-Kurzfassung** | Die Mail ist lang, ich brauche nur den Kern | J4 |

### B1 Aufgabe aus Mail 🟢

**Problem:** Viele Mails sind Arbeitsaufträge. Sie bleiben als „ungelesen markiert“ im Eingang liegen, weil es keinen besseren Ort gibt. Der Eingang wird zur schlechten Aufgabenliste.

**Nutzen:** Die Mail verlässt den Eingang, die Aufgabe bleibt mit Frist und Status sichtbar. Ein Klick in der Aufgabe öffnet die Mail wieder.

**Kern:**
- Titel aus dem Betreff, bereinigt („AW:“, „[ATLAS-412]“, „bis Freitag“ entfernt).
- **Frist aus dem Text erkannt** („bis Freitag“, „bis 20.11.“).
- Status: *Offen, In Arbeit, Warte auf …, Erledigt*. Darstellung als Kanban.
- Mail wird danach archiviert (abwählbar). Die Verknüpfung bleibt.
- Thema wird von der Mail übernommen.

**Erweiterungen:**
- 🟡 **Teilaufgaben aus Listen:** Enthält die Mail eine nummerierte Liste („1. Ist die Migration enthalten? 2. … 3. …“), bietet Maillet Teilaufgaben mit Checkboxen an.
- 🟡 **„Warte auf“ mit Antwort-Erkennung:** Kommt eine Antwort im Verlauf, meldet die Aufgabe „Antwort da“ und rückt nach vorn.
- 🟡 **Nachfassen:** Liegt der Ball zu lange beim anderen, legt ein Klick einen Erinnerungsentwurf als Antwort an.
- 🟡 **Zeit blocken:** Aus der Aufgabe einen Kalenderblock erzeugen (ICS).
- 🟡 **Wiederkehrend:** z. B. „Reisekosten einreichen, monatlich“.
- 🟡 **Mehrere Aufgaben aus einer Mail.**
- 🔵 **Export und Abgleich** mit CalDAV-Aufgaben (Nextcloud, Thunderbird), Todo.txt oder Markdown.

---

### B2 Wissen aus Mail 🟢

**Problem:** Kostenstellen, Auftragsnummern, Ansprechpartner, Regeln, Serveradressen. Die Information steckt in einer Mail von vor drei Monaten, und man sucht jedes Mal neu.

**Nutzen:** Einmal gemerkt, für immer griffbereit, auch wenn die Mail längst gelöscht ist. Die Quelle bleibt verlinkt.

**Kern:**
- Zwei Formen: **Fakt** (Name → Wert, mit Kopierknopf) und **Notiz** (Freitext).
- **Erkennung:** Zeilen der Form „Name: Wert“, Telefonnummern, IBAN, Auftrags- und Ticketnummern, Links. Erscheint in der Mail-Ansicht unter *Erkannt*, ein Klick übernimmt.
- **Text markieren** → *Merken*.
- Schlagworte und Thema.

**Erweiterungen:**
- 🟡 **Wissen im Kontext:** Beim Lesen einer Nordwind-Mail zeigt Maillet am Rand: „Zu Nordwind weißt du: Freigabe bis 50.000 € durch Hansen.“
- 🟡 **Ablaufdatum:** „VPN gültig bis 31.12.2026“ erinnert vier Wochen vorher.
- 🟡 **Versionen:** Ändert sich ein Fakt (neue Kostenstelle), bleibt die Historie mit Quelle erhalten.
- 🟡 **Schutz vor Geheimnissen:** Erkennt Maillet ein Passwort, rät es vom Speichern im Wissen ab und verweist auf den Passwortmanager.
- 🟡 **Markdown-Ordner:** Wissen als Markdown-Dateien, kompatibel mit Obsidian oder Logseq.

---

### B3 Termin aus Mail 🟡

**Problem:** „Go-Live am 18.12.“, „Hinfahrt 24.11., 07:12“. Termine stecken in Mails und müssen abgetippt werden.

**Nutzen:** Ein Klick auf ein erkanntes Datum erzeugt den Kalendereintrag.

**Kern:** Erkannte Daten und Uhrzeiten als Chips. Klick → Termin mit Titel aus dem Betreff und Link zur Mail, als ICS an den Systemkalender.

**Erweiterungen:**
- 🟡 **Einladungen (`text/calendar`)** direkt annehmen oder ablehnen.
- 🔵 **Reisen erkennen:** Bahn- und Flugbestätigungen werden zu einer Reise-Karte mit Hin- und Rückfahrt.

---

### B4 Kontakt aus Signatur 🟡

**Problem:** Telefonnummer und Rolle eines Ansprechpartners stehen nur in seiner Signatur.

**Nutzen:** Personen werden zu Karten mit Rolle, Firma, Telefon und allen gemeinsamen Mails, Aufgaben und Themen.

**Kern:** Signatur erkennen, Vorschlag „Kontakt anlegen: Malte Hansen, Einkauf, Nordwind, +49 40 …“. Personen-Karte per Klick auf den Avatar.

---

### B5 Delegieren 🟡

**Problem:** „Kannst du das übernehmen?“ – und dann vergisst man, nachzufragen.

**Nutzen:** Weiterleiten und Nachverfolgen in einem Schritt.

**Kern:** *Delegieren an …* leitet die Mail weiter und legt automatisch eine Aufgabe *Warte auf: Tom* an. Antwortet Tom, meldet sich die Aufgabe.

---

### B6 Schnellantworten 🟡

**Problem:** Viele Antworten sind kurz und immer gleich: „Danke, erledigt.“, „Bin dran, melde mich bis Freitag.“

**Nutzen:** Antworten in zwei Sekunden, und der Status stimmt danach automatisch.

**Kern:**
- Bausteine mit Platzhaltern, auswählbar per Kürzel.
- Jeder Baustein kann eine Folgeaktion haben: „Bin dran …“ legt eine Aufgabe an, „Erledigt“ archiviert, „Ich warte auf …“ setzt *Warte auf*.

---

### B7 Notiz an der Mail 🟡

**Problem:** „Habe Malte am Telefon gesagt, dass Punkt 2 geht.“ Wo schreibt man das hin?

**Nutzen:** Eine private, lokale Notiz direkt an der Mail. Sie erscheint bei der Mail, im Thema und in der Suche.

---

### B8 Anhänge sichern 🟡

**Problem:** Die Datei ist das Wichtige, die Mail nur die Verpackung. Anhänge verschwinden im Download-Ordner.

**Nutzen:** Anhänge landen dort, wo sie hingehören, mit Herkunft.

**Kern:**
- *Anhang sichern in …* mit Ordnervorschlag aus dem Thema.
- **Anhänge eines Themas** als eigene Sammlung.

**Erweiterungen:**
- 🟡 **Versionen erkennen:** „Spec v0.9“, „Spec v1.0“ werden als Versionen derselben Datei gezeigt.
- 🟡 **Belege-Ordner:** Rechnungs-PDFs automatisch in `~/Belege/{Jahr}` (für Steuer oder Buchhaltung).

---

### B9 Anpinnen 🟢

**Problem:** Manche Mails braucht man heute noch fünfmal: das Protokoll, die Zugangsdaten zum Workshop.

**Nutzen:** Griffbereit auf der Übersicht und im Tray, ohne den Eingang zu verstopfen.

**Erweiterungen:**
- 🟡 **Pin mit Ablauf:** „angepinnt bis Freitag“ löst sich von selbst.
- 🟡 **Pin im Thema:** wichtige Mails oben im Arbeitsraum.

---

### B10 Später 🟢

**Problem:** Die Mail ist wichtig, aber nicht jetzt.

**Nutzen:** Sie verschwindet und kommt verlässlich zurück. Der Eingang bleibt leer.

**Kern:** Zeitpunkte (heute Nachmittag, morgen früh, Montag) und frei wählbares Datum. Der Zustand liegt lokal, der Server bleibt unverändert.

**Erweiterungen:**
- 🟢 **„Wenn keine Antwort kommt“:** für eigene gesendete Mails. Kommt in drei Tagen keine Antwort, erscheint die Mail wieder.
- 🟡 **Ereignisse:** „Wenn ich im Büro bin“ (Netzwerk erkannt), „vor dem Termin X“.

---

### B11 Antwortschuld 🟡

**Problem:** Die Mail ist gelesen, die Antwort „mache ich gleich“. Drei Tage später fragt der Absender nach.

**Nutzen:** Maillet zeigt, wer auf mich wartet, bevor es peinlich wird.

**Kern:**
- Mails von Menschen, die eine Frage oder Bitte enthalten (Muster wie Fragezeichen am Absatzende, „bitte“, „kannst du“, oder KI-Prompt J5) und auf die ich nicht geantwortet habe, sammeln sich als **„Ich schulde Antworten“**.
- Als Board-Karte mit Alter: „Malte Hansen · 2 Tage“.

**Erweiterungen:**
- 🟡 **Gegenstück zu „Warte auf“**: beide Richtungen auf einen Blick.

---

## C. Aufbewahren – die Ablage als lokales Mailkonto

### C1 Die Idee

Maillet hat **keine eigenen Kategorien, Ordner oder Labels auf dem Server**. Wer eine Mail behalten will, legt sie in die **Ablage**: ein eigenes Mailkonto, das nur aus Dateien auf dem eigenen Rechner besteht (Maildir oder `.eml`-Dateien unter `~/Maillet/Ablage`). Darin gibt es frei gestaltbare Ordner, z. B. `Projekte/Atlas`, `Verträge`, `Personal`.

Für Maillet ist die Ablage ein Konto wie jedes andere: Gleiche Darstellung, gleiche Suche, gleiche Aktionen. Aufgaben und Wissen verlinken auf abgelegte Mails genauso wie auf Server-Mails.

### C2 Der Mehrwert

| Mehrwert | Warum das wichtig ist |
|---|---|
| **Unabhängig vom Server** | Firmen löschen Mails oft nach 90 oder 180 Tagen automatisch, Postfächer haben Quoten. Der Rahmenvertrag von 2026 ist 2028 noch da. |
| **Unabhängig vom Arbeitgeber und Anbieter** | Beim Anbieterwechsel, bei einem gekündigten Privatkonto oder einem neuen Job bleibt die eigene Ablage (sofern die Mail einem gehören darf). |
| **Immer offline** | Im Zug, im Flugzeug, ohne VPN. Kein Cache, der irgendwann verfällt, sondern echte Dateien. |
| **Offenes, dauerhaftes Format** | `.eml`/Maildir liest jedes Mailprogramm, jeder Texteditor, `grep`. In 20 Jahren noch lesbar, ohne Maillet. |
| **Original statt Abschrift** | Header, Anhänge und Signaturen bleiben unverändert. Wichtig als Nachweis („Wann genau kam die Freigabe?“). |
| **Eigene Struktur** | Ordner nach eigener Logik, ohne Rücksicht auf Server-Ordner, die Kollegen oder andere Programme sehen. |
| **Schlankes Server-Postfach** | Weniger Mails auf dem Server: schnellere Synchronisation, weniger Quota-Probleme, weniger Daten beim Anbieter. |
| **Normale Werkzeuge** | Backup mit dem eigenen Werkzeug (rsync, restic, Déjà Dup), Desktop-Suche (Baloo, Tracker), Dateimanager. |
| **Weitergabe** | Ein Ordner lässt sich als ZIP übergeben, z. B. bei einer Projektübergabe. |

### C3 Grenzen, die das Design berücksichtigen muss

- **Aufbewahrungspflichten im Unternehmen:** Manche Firmen verbieten das Entfernen von Mails vom Server (Compliance, Legal Hold). Für Firmenkonten ist daher **„Kopie, Mail bleibt auf dem Server“** wählbar und kann Standard sein.
- **Verantwortung für Sicherung:** Lokale Dateien sind nur so sicher wie das Backup. Maillet erinnert daran und zeigt den Zeitpunkt des letzten Backups, wenn erkennbar.
- **Mehrere Rechner:** Die Ablage synchronisiert sich nicht selbst. Sie kann in einem Sync-Ordner liegen (Syncthing, Nextcloud).
- **Vertrauliche Inhalte:** Optional verschlüsselte Ablage.

### C4 Funktionen der Ablage

| | Funktion | Nutzen |
|---|---|---|
| 🟢 | **Ablegen** mit Ordnerwahl, Vorschlag aus dem Thema, Modus *Verschieben* oder *Kopie* | Kern |
| 🟢 | **Ordner frei anlegen**, auch beim Ablegen | Struktur entsteht, wenn sie gebraucht wird |
| 🟢 | **Suche über die Ablage** zusammen mit allen Konten | Eine Suche für alles |
| 🟢 | **Sichtbarer Dateiname**, z. B. `Verträge/2026-03-14_Rahmenvertrag-Nordwind.eml` | Vertrauen: „Das liegt wirklich bei mir.“ |
| 🟡 | **Ablage-Regeln:** Rechnungen von X → `Rechnungen/{Jahr}` automatisch | Belege ordnen sich selbst |
| 🟡 | **Anhänge daneben speichern**, z. B. als PDF neben der `.eml` | Direkt im Dateimanager nutzbar |
| 🟡 | **Als PDF exportieren** (Mail mit Kopfzeilen) | Für Akten und Ausdrucke |
| 🟡 | **Aufbewahrungsfristen** pro Ordner, z. B. „Rechnungen: 10 Jahre“, mit Hinweis bei Ablauf | Ordnung ohne Nachdenken |
| 🟡 | **Duplikaterkennung** über `Message-ID` | Nichts doppelt |
| 🟡 | **Import** aus Thunderbird, mbox oder einem IMAP-Ordner | Altbestände übernehmen |
| 🟡 | **Mehrere Ablagen**, z. B. *Arbeit* und *Privat*, an verschiedenen Orten | Trennung von Beruf und Privatem |
| 🔵 | **Verschlüsselte Ablage** (z. B. age oder gocryptfs) | Vertrauliches sicher |
| 🔵 | **Themen-Paket:** Thema als ZIP mit Mails, Aufgaben, Wissen und einer Übersicht als HTML | Projektübergabe in einem Schritt |

---

## D. Themen – Arbeitsräume

### D1 Thema als Arbeitsraum 🟢

**Problem:** Zu einem Kunden gibt es Mails, Aufgaben, Notizen und Dateien, verteilt auf Postfach, Aufgabenliste, Notizprogramm und Dateisystem.

**Nutzen:** Ein Klick auf das Thema zeigt alles an einem Ort.

**Kern:** Vier Spalten: offene Mails, Aufgaben, Wissen, Ablage. Farbe und Name direkt bearbeitbar, Regeln sichtbar.

**Erweiterungen:**
- 🟡 **Beteiligte:** alle Personen des Themas mit Rolle und letzter Mail.
- 🟡 **Zeitleiste:** was wann passiert ist, über Mails, Aufgaben und Ablage hinweg.
- 🟡 **Ruhend / Aktiv:** ruhende Themen verschwinden aus der Übersicht, bleiben aber durchsuchbar.
- 🟡 **Thema abschließen:** offene Mails archivieren, Mails ablegen, Aufgaben prüfen – geführt in einem Schritt.
- 🟡 **Themen-Notiz:** ein freies Feld für den Stand der Dinge.
- 🔵 **Übergabe:** Themen-Paket für Vertretung oder Nachfolge (siehe C4).

### D2 Vor dem Termin 🔵

**Problem:** In zehn Minuten ist der Call mit Nordwind. Was war noch offen?

**Nutzen:** Vorbereitung in einem Blick, ohne zu suchen.

**Kern:** Steht im Kalender ein Termin mit Personen eines Themas, zeigt Maillet vorher eine Karte: offene Aufgaben, letzte Mails, Wissen zum Thema, offene Fragen. Optional mit KI-Kurzfassung „Stand der Dinge“ (J4).

---

## E. Aufmerksamkeit – wann Maillet stören darf

### E1 Drei Lautstärken 🟢

*Laut* (Menschen, Codes, VIPs), *Leise* (Maschinen), *Stumm* (Rauschen). Details im [Bedienkonzept](Bedienkonzept.md#6-aufmerksamkeit).

### E2 Fokus-Modus 🟢

**Nutzen:** Konzentriert arbeiten, ohne das Gefühl, etwas Wichtiges zu verpassen.

**Kern:** Nur *Laut* benachrichtigt. Beim Beenden: „Während des Fokus: 2 Menschen, 9 Maschinen.“

**Erweiterungen:**
- 🟡 **Zeitplan** und **Ruhezeiten** pro Konto (Arbeitskonto am Wochenende stumm).
- 🟡 **Kopplung** an den „Nicht stören“-Modus des Desktops.

### E3 Morgen-Briefing 🟡

**Problem:** Der Tag beginnt mit 40 ungelesenen Mails und keinem Überblick.

**Nutzen:** Ein Blick genügt: Was ist neu, was ist fällig, worauf warte ich?

**Kern:** Beim ersten Öffnen des Tages eine Karte: „5 Mails von Menschen, 2 Aufgaben heute fällig, 1 überfällig, du wartest auf 3 Antworten. Triage starten?“

### E4 Tagesabschluss 🟡

**Problem:** Offenes nimmt man gedanklich mit in den Feierabend.

**Nutzen:** Ein geführter Abschluss in zwei Minuten: Liegengebliebenes auf morgen schieben, Erledigtes abhaken, Eingang leeren.

**Kern:** Zur Feierabendzeit (oder per Klick): unbearbeitete Mails, offene Aufgaben mit Frist heute, je mit *morgen* / *erledigt* / *später*.

### E5 Benachrichtigungen mit Aktionen 🟢

*Öffnen, Aufgabe, Erledigt*, bei Codes *Kopieren*. Viele Mails lassen sich erledigen, ohne Maillet überhaupt zu öffnen.

### E6 Rückkehr nach Abwesenheit 🟡

**Problem:** Nach zwei Wochen Urlaub warten 600 Mails.

**Nutzen:** Aus einem Berg wird eine geführte Stunde.

**Kern:**
- Maillet erkennt die Lücke (oder die Abwesenheitsnotiz) und bietet einen **Rückkehr-Modus** an.
- Reihenfolge: zuerst Menschen und VIPs, dann je Thema gebündelt, zuletzt Maschinen und Rauschen mit „alles erledigen“.
- Erledigte Vorgänge erkennen: Threads, in denen andere die Frage längst beantwortet haben, werden als „hat sich erledigt“ markiert.
- Mit KI (J2, J4): „Was ist während deiner Abwesenheit passiert?“ als Kurzfassung je Thema.

---

## F. Wiederfinden

### F1 Eine Suche für alles 🟢

**Problem:** Wo war das noch? Im Postfach, im Archiv, in den Notizen, in einem Ordner?

**Nutzen:** Ein Suchfeld durchsucht Posteingang, Server-Archiv, Ablage, Aufgaben, Wissen, Themen und Befehle.

**Kern:** `Strg K` oder `/`. Filter wie `von:hansen`, `thema:atlas`, `hat:anhang`, `vor:30d`. Ergebnisse nach Art gruppiert. Befehle direkt ausführbar.

**Erweiterungen:**
- 🟡 **Gespeicherte Suchen als Smart-Karten** auf dem Board, z. B. „Rechnungen unbezahlt“, „Mails an mich von der Geschäftsführung“ (siehe I3).
- 🔵 **Fragen statt suchen:** „Wer gibt bei Nordwind frei?“ beantwortet die KI aus Mails, Ablage und Wissen, mit Quellen (siehe J7).
- 🟡 **Unscharfe Erinnerung:** „PDF von Nordwind im Oktober“ wird zu `von:nordwind hat:anhang zeit:2026-10`.
- 🔵 **Semantische Suche** über lokale Einbettungen (opt-in).

### F2 Personen 🟡

Klick auf einen Absender zeigt alles mit dieser Person: Mails, Aufgaben, Wissen, gemeinsame Themen, letzte Antwort.

---

## G. Freude – warum Mailarbeit Spaß machen kann

| | Idee | Wirkung |
|---|---|---|
| 🟢 | **Triage-Modus:** eine Mail nach der anderen, große Knöpfe, Tastatur | Fokus, Tempo, Fluss |
| 🟢 | **Bilanz am Ende:** „14 Mails in 4 Minuten: 6 erledigt, 4 Aufgaben, 2 gemerkt“ | Fortschritt wird sichtbar |
| 🟢 | **Bewegung mit Bedeutung:** Karten fliegen in Richtung ihres Ziels | Jede Aktion fühlt sich an wie Aufräumen |
| 🟢 | **Schöne Leerzustände** | Ein leerer Eingang ist eine kleine Belohnung |
| 🟡 | **Sprint:** 5-Minuten-Timer für die Triage | Kleine, machbare Etappen |
| 🟡 | **Wochenrückblick:** „Diese Woche: 142 Mails, 31 Aufgaben, 12 Fakten gemerkt, Eingang 4× leer“ | Stolz statt Stress |
| 🟡 | **Dezente Klänge** (optional) für Erledigt und leeren Eingang | Haptisches Gefühl |
| — | **Bewusst nicht:** Punkte, Abzeichen, Ranglisten, Streaks | Maillet entlastet, es treibt nicht an |

---

## H. Anbindung an andere Werkzeuge

Maillet baut keine Kalender-, Aufgaben- oder Notizprogramme nach. Es übergibt an sie.

| | Anbindung | Format |
|---|---|---|
| 🟡 | Kalender | ICS-Datei an den Systemkalender, später CalDAV |
| 🟡 | Aufgaben | CalDAV-VTODO, Todo.txt, Markdown |
| 🟡 | Wissen | Markdown-Ordner (Obsidian, Logseq) |
| 🟢 | Ablage | Maildir, für Dateimanager, Backup und Desktop-Suche |
| 🟢 | Codes | lokale HTTP-API (aus OTP Scout) |
| 🔵 | Kommandozeile | `maillet task add …`, `maillet search …` |
| 🔵 | Erweiterungen | Plugin-Schnittstelle für Erkennungsmuster und Aktionen |

---

## I. Bestand – was immer da ist

Der Fluss soll leer werden. Der Bestand ist das Gegenstück: Karten mit festem Platz, die man nachschlägt oder beobachtet und die nie zum Abarbeiten drängen.

### I1 Board mit festen Plätzen 🟢

**Problem:** Im klassischen Postfach liegen Dauerbrenner (Protokoll, Zugangsdaten, Ticketstand) zwischen neuen Mails. Entweder sie gehen unter, oder sie verstopfen den Eingang.

**Nutzen:** Dauerhaftes hat einen eigenen, ruhigen Ort. Das Auge findet es immer an derselben Stelle.

**Kern:**
- Rechte Zone der Übersicht. Karten: Angepinnt, Zustand, Spickzettel, Heute dran, Themen, Smart-Karten, Konten.
- **Anpassen:** Karten verschieben, entfernen, hinzufügen. Die Anordnung bleibt.
- Board-Karten zählen nicht in Badges und Fortschritt.

**Erweiterungen:**
- 🟡 **Tray-Board:** auswählen, welche Board-Karten im Tray-Popup erscheinen.
- 🟡 **Mehrere Boards**, z. B. „Arbeit“ und „Privat“, oder je Wochentag.
- 🔵 **Board je Thema:** Der Arbeitsraum eines Themas bekommt eigene Board-Karten.

---

### I2 Zustandskarten 🟡

**Problem:** Systeme schicken für jeden Zwischenstand eine Mail: Ticket zugewiesen, kommentiert, in Review, geschlossen. Paket versendet, im Zentrum, in Zustellung. Build rot, grün. Jede Mail ist sofort veraltet, wenn die nächste kommt. Interessant ist nur der **aktuelle Stand**.

**Nutzen:** Statt zehn Mails im Fluss eine Karte im Bestand, die immer den aktuellen Stand zeigt. Der Fluss bleibt frei für Menschen.

**Kern:**
- **Erkennung** über Regeln und Katalog: Schlüssel aus Betreff oder Text (`[ATLAS-412]`, Sendungsnummer, Pipeline-Name, Rechnungsnummer), Zustand aus Betreff oder Text.
- Erste Mail eines Vorgangs legt die Karte an, jede weitere aktualisiert sie. Die Mails gelten als erledigt.
- Karte zeigt Titel, aktuellen Zustand farbig, Zeit der letzten Änderung, Anzahl der Updates. Änderung leuchtet kurz auf.
- Klick zeigt die **Historie** mit Links zu den einzelnen Mails.
- Endzustand (geschlossen, zugestellt, bezahlt) → Karte bietet an, sich zu entfernen.

**Beispiele:**

| Quelle | Schlüssel | Zustände |
|---|---|---|
| Jira, GitHub, GitLab | Ticket-/Issue-Nummer | zugewiesen → in Arbeit → Review → erledigt |
| CI/CD | Projekt + Branch | läuft → grün / rot |
| Paketdienste | Sendungsnummer | versendet → unterwegs → Zustellung → zugestellt |
| Bestellungen | Bestellnummer | bestätigt → versendet → geliefert |
| Rechnungen | Rechnungsnummer | offen → fällig → bezahlt (manuell oder per Zahlungsmail) |
| Flüge, Bahn | Buchungsnummer | gebucht → Check-in offen → Gleis/Gate → beendet |

**Erweiterungen:**
- 🟡 **Ausnahme laut:** Bestimmte Zustände dürfen doch in den Fluss und benachrichtigen, z. B. „Build rot auf main“ oder „Ticket wieder geöffnet“.
- 🟡 **Zustandskarten-Katalog** wie der Regelkatalog aus OTP Scout.
- 🔵 **Eigene Zustandskarten per KI-Prompt:** „Fasse den aktuellen Stand dieses Vorgangs in einer Zeile zusammen.“

---

### I3 Smart-Karten 🟡

**Problem:** Manche Fragen stellt man immer wieder: Welche Rechnungen sind offen? Was kam von der Geschäftsführung?

**Nutzen:** Die Antwort steht dauerhaft auf dem Board und aktualisiert sich selbst.

**Kern:** Jede Suche lässt sich als Smart-Karte auf das Board legen. Die Karte zeigt Anzahl und die neuesten Treffer.

---

### I4 Spickzettel 🟡

**Problem:** Kostenstelle, Hotline, Projektnummer: täglich gebraucht, jedes Mal gesucht.

**Nutzen:** Ein Klick kopiert den Wert, direkt vom Board oder aus dem Tray.

**Kern:** Fakten aus dem Wissen mit Stern markieren. Sie erscheinen auf der Board-Karte *Spickzettel* und optional im Tray.

---

## J. KI-Assistenz – große Mengen bewältigen

Die KI ist eine optionale Schicht über allen Funktionen. Ihr Zweck: **Lesen und Bewerten erleichtern**, wenn es zu viel ist. Sie folgt den Bedienregeln im [Bedienkonzept](Bedienkonzept.md#10-ki-assistenz): eigene Prompts, sichtbar markiert, nachvollziehbar, lokal bevorzugt, handelt nie allein.

### J1 Prompt-Bibliothek 🟡

**Problem:** Eine allgemeine KI-Bewertung („wichtig“) passt nie zu den eigenen Kriterien. Was wichtig ist, weiß nur der Nutzer.

**Nutzen:** Der Nutzer beschreibt in eigenen Worten, worauf es ihm ankommt. Die KI wendet das auf jede passende Mail an.

**Kern:**
- Prompts mit **Name, Anweisung, Geltungsbereich, Auslöser, Ausgabeform** (siehe Bedienkonzept 10.1).
- **Testen an einer Mail** direkt beim Schreiben, mit sichtbarem Ergebnis.
- Einzeln an- und abschaltbar.

**Beispiel-Prompts:**

| Name | Bereich | Auslöser | Ausgabe | Anweisung (gekürzt) |
|---|---|---|---|---|
| Newsletter-Radar | Rauschen | 2× täglich gebündelt | Kurzliste | „Mich interessieren Rust, Tauri, Linux-Desktop. Nenne nur Beiträge dazu.“ |
| Bitte an mich? | Menschen | bei Eingang | Ja/Nein + was + bis wann | „Enthält die Mail eine Bitte, Frage oder Aufgabe an mich?“ |
| Wichtigkeit | Konto Arbeit | bei Eingang | Punktzahl 1–5 + Grund | „Wichtig ist, was Budget, Termine, Kunde Nordwind betrifft.“ |
| Kundenstimmung | Thema Nordwind | beim Öffnen | Kurzfassung | „Wie ist die Stimmung? Gibt es Risiken?“ |
| Jira-Tagesstand | Absender Jira | täglich 9 Uhr | Kurzfassung | „Was hat sich seit gestern an meinen Tickets geändert?“ |

**Erweiterungen:**
- 🟡 **Vorlagen-Katalog** mit bewährten Prompts.
- 🟡 **Platzhalter** wie `{mein_name}`, `{meine_projekte}`, `{thema}`.
- 🔵 **Teilen** von Prompts im Team (als Datei).

---

### J2 Radar und Digest für Massen 🟡

**Problem:** 40 Newsletter, 25 Jira-Mails, 3 Mailinglisten pro Tag. Lesen ist unmöglich, ungelesen löschen fühlt sich riskant an.

**Nutzen:** Aus vielen Mails wird eine Karte: „Von 38 Newslettern sind 3 für dich relevant: …“. Der Rest kann mit gutem Gewissen weg.

**Kern:**
- Gebündelte Auswertung eines Bereichs (Rauschen, ein Bündel, eine Mailingliste) mit einem Prompt.
- Ergebnis als **Digest-Karte** oben im Fluss, mit Links zu den relevanten Mails und *Rest erledigen*.

---

### J3 Bewertung und Sortierung 🟡

**Problem:** Die Reihenfolge nach Eingangszeit sagt nichts über Wichtigkeit.

**Nutzen:** Der Fluss lässt sich nach den eigenen Kriterien sortieren. Das Wichtigste steht oben, mit Begründung.

**Kern:**
- Punktzahl 1–5 aus dem Prompt *Wichtigkeit*, sichtbar als kleines ✦-Abzeichen mit Grund.
- Umschalter im Fluss: *Zeit* oder *✦ Wichtigkeit*.
- Keine Mail wird versteckt, nur sortiert.

---

### J4 Kurzfassungen 🟡

**Problem:** Lange Mails, lange Threads, lange Abwesenheiten.

**Nutzen:** Erst den Kern lesen, dann entscheiden, ob das Original nötig ist.

**Kern:**
- **Mail:** zwei bis drei Sätze, oben in der Mail-Ansicht im ✦-Band.
- **Thread:** „Worum geht es, was ist entschieden, was ist offen?“
- **Thema:** „Stand der Dinge mit Nordwind“ im Arbeitsraum (passt zu D2 und E6).

---

### J5 Bessere Erkennung: Bitten, Fragen, Fristen 🟡

**Problem:** Muster erkennen „bis Freitag“, aber nicht „wäre super, wenn du dir das vor dem Termin mit dem Kunden anschaust“.

**Nutzen:** Treffsichere Vorschläge für Aufgaben, Teilaufgaben und Antwortschuld.

**Kern:**
- Der Prompt *Bitte an mich?* liefert: ja/nein, was, bis wann.
- Mehrere Fragen werden zu **Teilaufgaben** einer Aufgabe.
- Speist B1 (Aufgabe), B11 (Antwortschuld) und die Triage-Vorschläge.

---

### J6 KI als Regelbedingung 🔵

**Problem:** Manche Regeln lassen sich nicht über Absender oder Betreff ausdrücken: „alle Mails, in denen ein Kunde unzufrieden ist“.

**Nutzen:** Regeln in natürlicher Sprache.

**Kern:** Bedingung „Prompt X sagt ja“, z. B. „Ist das eine Rechnung?“ → Thema *Rechnungen*, Ablage `Rechnungen/{Jahr}`. Wie alle automatischen Aktionen mit *Rückgängig* und sichtbarem *auto*.

---

### J7 Frag deine Mails 🔵

**Problem:** „Wer gibt bei Nordwind frei?“ Die Antwort steckt in einer von 3.000 Mails.

**Nutzen:** Eine Frage statt einer Suche. Die Antwort kommt mit Quellen.

**Kern:**
- In der Suche: Endet die Eingabe mit `?`, bietet Maillet *✦ KI fragen* an.
- Antwort aus Mails, Ablage (falls freigegeben) und Wissen, jede Aussage mit Link zur Quelle.
- *Ins Wissen übernehmen* mit einem Klick.

---

### J8 Schreibhilfe 🔵

**Problem:** Antworten kosten Zeit, besonders auf Englisch oder in heiklen Situationen.

**Nutzen:** Schneller antworten, ohne den eigenen Ton zu verlieren.

**Kern:**
- **Entwurf aus Stichworten:** „ja zu 1, 2 nein, Ansprechpartner Tom“ wird zu einer höflichen Antwort.
- **Übersetzen** beim Lesen und Schreiben.
- **Ton-Check** vor dem Senden: „Klingt gereizt. Freundlicher formulieren?“

---

### J9 Kontrolle und Datenschutz 🟡

Pflicht, sobald es irgendeine KI-Funktion gibt.

| Funktion | Inhalt |
|---|---|
| **Modellwahl** | Lokal (Ollama o. ä.), eigener API-Schlüssel, aus |
| **Sperrbereiche** | Konten, Themen, Absender ganz ausschließen |
| **Protokoll** | Was wurde wann mit welchem Prompt an welches Modell gegeben |
| **Datenumfang** | je Prompt: nur Betreff, Betreff + Anfang, ganze Mail |
| **Kosten und Last** | Auswertung bevorzugt im Leerlauf, Ergebnisse zwischengespeichert, Tageslimit bei API-Modellen |
| **Ein Schalter** | Hauptschalter schaltet alles sofort ab |

---

## Priorisierung

### Phase 1 – Kern (MVP)

Ziel: Der Eingang wird zur Durchgangsstation.

| Funktion | Nutzen | Aufwand |
|---|---|---|
| A1 Gruppen | hoch | M |
| A2 Regeln (Kern, entstehen beim Zuordnen) | hoch | M |
| A3 Themen-Vorschläge | mittel | S |
| A7 Codes (aus OTP Scout) | hoch | S |
| B1 Aufgabe (Kern, Frist erkannt, Kanban) | hoch | M |
| B2 Wissen (Kern, Erkannt-Chips, Markieren) | hoch | M |
| B9 Anpinnen, B10 Später inkl. „wenn keine Antwort“ | hoch | S |
| C Ablage (Ablegen, Ordner, Suche, Kopie/Verschieben) | hoch | M |
| D1 Themen als Arbeitsraum | hoch | M |
| E1 Lautstärken, E2 Fokus, E5 Benachrichtigungs-Aktionen | hoch | S |
| F1 Eine Suche | hoch | M |
| G Triage, Bilanz, Bewegung | mittel | S |
| I1 Board mit festen Plätzen (Fluss und Bestand getrennt) | hoch | S |

### Phase 2 – Entlastung

Ziel: Weniger Klicks, weniger vergessen.

A4 Bündel · A5 Konversationen · A6 Newsletter-Zentrale · A8 Verfallende Anzeigen · B1 Teilaufgaben, Warte-auf mit Antwort-Erkennung, Nachfassen · B2 Wissen im Kontext, Ablaufdatum · B3 Termin · B5 Delegieren · B6 Schnellantworten · B11 Antwortschuld · C Ablage-Regeln, Anhänge daneben, Import · E3 Morgen-Briefing · E4 Tagesabschluss · E6 Rückkehr nach Abwesenheit · I2 Zustandskarten · I3 Smart-Karten · I4 Spickzettel · **J1 Prompt-Bibliothek, J2 Radar, J3 Bewertung, J4 Kurzfassungen, J5 Erkennung, J9 Kontrolle** (lokales Modell)

### Phase 3 – Vision

Ziel: Maillet als ruhiges Zentrum der Kommunikationsarbeit.

B4 Kontakte · B8 Anhang-Versionen · C verschlüsselte Ablage, Themen-Paket · D Zeitleiste, Übergabe · D2 Vor dem Termin · F semantische Suche · H CalDAV, Kommandozeile, Plugins · I Board je Thema · J6 KI als Regelbedingung · J7 Frag deine Mails · J8 Schreibhilfe

---

## Offene Fragen

1. **Firmenkonten:** Soll *Kopie* statt *Verschieben* der Standard beim Ablegen sein, um Aufbewahrungsregeln nicht zu verletzen?
2. **Aufgaben-Speicher:** Eigene lokale Datenbank, oder von Anfang an CalDAV-kompatibel?
3. **Wissen-Speicher:** Eigene Datenbank mit Export, oder direkt Markdown-Dateien als Quelle?
4. **Ein Thema pro Mail:** Reicht das im Alltag, oder braucht es eine zweite Ebene?
5. **Antworten:** Wie viel Editor soll Maillet haben, bevor es an einen vollwertigen Client übergibt?
6. **KI-Modell:** Welches lokale Modell ist der Standard, und ab welcher Rechnerleistung? Gibt es einen Fallback für schwache Rechner (nur Muster, keine KI)?
7. **Zustandskarten:** Sollen Zustandsmails auf dem Server automatisch archiviert werden, oder nur in Maillet aus dem Fluss verschwinden?
