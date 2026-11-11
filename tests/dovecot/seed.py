"""Creates test mails on the local Dovecot test server (see DEVELOPMENT.md). Usage: seed.py [initial|otp|person|newsletter|seen-first]"""
import imaplib, ssl, sys, time, email.utils
ctx = ssl.create_default_context(); ctx.check_hostname = False; ctx.verify_mode = ssl.CERT_NONE
M = imaplib.IMAP4_SSL('127.0.0.1', 31993, ssl_context=ctx)
M.login('test@maillet.example', 'testpass')
def put(raw, flags=''):
    M.append('INBOX', flags, imaplib.Time2Internaldate(time.time()), raw.replace('\n', '\r\n').encode())
now = email.utils.formatdate(localtime=True)
mode = sys.argv[1] if len(sys.argv) > 1 else 'initial'
if mode == 'initial':
    M.create('Archive')
    put(f"""From: Jana Wolff <jana.wolff@firma.example>
To: test@maillet.example
Subject: Atlas: Review der Spezifikation bis Freitag
Date: {now}
MIME-Version: 1.0
Content-Type: multipart/mixed; boundary="mix"

--mix
Content-Type: multipart/alternative; boundary="alt"

--alt
Content-Type: text/plain; charset=utf-8
Content-Transfer-Encoding: quoted-printable

Hallo,

anbei die Spezifikation f=C3=BCr den neuen Login-Flow (v0.9). Kannst du bitte bis Freitag ein Review machen?

Danke dir!
Jana
--alt
Content-Type: text/html; charset=utf-8

<p>Hallo,</p><p>anbei die Spezifikation</p>
--alt--
--mix
Content-Type: application/pdf; name="Atlas-Spec-v0.9.pdf"
Content-Disposition: attachment; filename="Atlas-Spec-v0.9.pdf"
Content-Transfer-Encoding: base64

JVBERi0xLjQK
--mix--
""")
    put(f"""From: =?UTF-8?Q?Malte_M=C3=BCller?= <m.mueller@nordwind.example>
To: test@maillet.example
Subject: =?UTF-8?Q?Angebot_Phase_2_=E2=80=93_drei_R=C3=BCckfragen?=
Date: {now}
Content-Type: text/plain; charset=utf-8

Guten Tag,

vielen Dank für das Angebot zu Phase 2. Bevor wir intern freigeben, haben wir drei Rückfragen.

Beste Grüße
Malte Müller
""")
    put(f"""From: heise Developer <newsletter@heise.example>
To: test@maillet.example
Subject: Developer Weekly #47: Rust 1.95, WebGPU und mehr
Date: {now}
List-Unsubscribe: <mailto:off@heise.example>
Content-Type: text/html; charset=utf-8

<html><body><h1>Die Themen dieser Woche</h1><p>Rust 1.95 ist da, WebGPU in allen Browsern.</p></body></html>
""")
    put(f"""From: Tom Becker <tom@firma.example>
To: test@maillet.example
Subject: Mittag heute?
Date: {now}
Content-Type: text/plain; charset=utf-8

Hey, 12:30 Kantine oder Thai um die Ecke?
""")
    put(f"""From: alt@firma.example
To: test@maillet.example
Subject: Bereits gelesen
Date: {now}
Content-Type: text/plain

Diese Mail ist gelesen und darf nicht erscheinen.
""", r'(\Seen)')
elif mode == 'otp':
    put(f"""From: GitHub <noreply@github.com>
To: test@maillet.example
Subject: 482913 is your verification code
Date: {now}
Content-Type: text/plain

Your GitHub verification code is 482913.
""")
elif mode == 'person':
    put(f"""From: Sara Yilmaz <sara@firma.example>
To: test@maillet.example
Subject: Kurze Frage zur Demo morgen
Date: {now}
Content-Type: text/plain; charset=utf-8

Hi, können wir die Demo morgen um 10:00 machen statt um 14:00?
""")
elif mode == 'newsletter':
    put(f"""From: Produktnews <news@saas.example>
To: test@maillet.example
Subject: 12 neue Funktionen
Date: {now}
List-Unsubscribe: <mailto:off@saas.example>
Content-Type: text/plain

Entdecken Sie unsere neuesten Funktionen.
""")
elif mode == 'seen-first':
    M.select('INBOX')
    typ, data = M.uid('search', None, 'UNSEEN')
    first = data[0].split()[0]
    M.uid('store', first, '+FLAGS', r'(\Seen)')
    print('als gelesen markiert: UID', first.decode())
M.select('INBOX')
print(mode, 'ok – ungelesen:', len(M.uid('search', None, 'UNSEEN')[1][0].split()))
M.logout()
