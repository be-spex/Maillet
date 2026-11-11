# Outlook and Microsoft 365

Many organisations run their mail on Microsoft 365 (Exchange Online) with IMAP turned off. Maillet can read such a mailbox through **Microsoft Graph**, the official interface – if your organisation lets you allow it.

## Will it work for me?

Two settings of your organisation decide:

1. **May users allow apps themselves?** ("User consent"). If yes, Microsoft shows a consent screen when you sign in, and Maillet works.
2. If not, Microsoft says **"Need admin approval"** during sign-in. Then Maillet cannot read this mailbox without your IT.

You find out by trying – the sign-in changes nothing in your mailbox and needs no request to anyone.

Personal Outlook.com and Hotmail accounts always work.

## 1. Register Maillet (once, about 5 minutes)

Maillet needs an *app registration*: an entry at Microsoft that says "this program may ask for access to mail". You create it with your Microsoft account – a personal one is fine, it does not have to be the work account.

1. Open [entra.microsoft.com](https://entra.microsoft.com) → **Applications → App registrations → New registration**.
   (Without access there: [portal.azure.com](https://portal.azure.com) → *App registrations*.)
2. **Name:** `Maillet`
3. **Supported account types:** *Accounts in any organizational directory and personal Microsoft accounts*.
4. **Redirect URI:** leave empty. → **Register**.
5. Copy the **Application (client) ID** shown on the overview page.
6. **Authentication** → *Advanced settings* → **Allow public client flows: Yes** → Save.
   (Maillet signs in with a code you enter in the browser; it stores no app secret.)
7. **API permissions** → *Add a permission* → *Microsoft Graph* → *Delegated permissions*:
   `Mail.ReadWrite`, `User.Read`, `offline_access` → **Add permissions**.
   Do **not** click "Grant admin consent" – that is for administrators of the organisation.

## 2. Add the account in Maillet

1. Settings → **Accounts → + Add account**.
2. **Account type:** *Microsoft 365 / Outlook.com*.
3. **Application (client) ID:** the ID from step 1.5.
4. **Accounts:** *Work or school* for Microsoft 365, *Personal* for Outlook.com.
5. **Sign in with Microsoft.** Maillet shows a code and opens the Microsoft page. Enter the code, sign in with your mail account and allow access.

Maillet then creates the account and starts checking it.

## What Maillet does with the access

- `Mail.ReadWrite`: reads mails, marks them read or unread, flags them (pin) and moves them (Done, Tasks, categories, trash). It cannot send mail.
- `User.Read`: reads your name and address for the account list.
- `offline_access`: keeps you signed in. The sign-in token lives in the system keyring, like IMAP passwords.

Access can be withdrawn at any time: [myaccount.microsoft.com](https://myaccount.microsoft.com) → *Apps & services* (work) or [account.live.com/consent/Manage](https://account.live.com/consent/Manage) (personal).

## Differences to IMAP

- New mails show up within a minute (Graph offers no push for desktop programs); **Check now** checks at once.
- Done moves mails to the mailbox's *Archive* folder, Delete to *Deleted Items*.
- Task and category folders are created as normal Outlook folders, e.g. `Tasks/In Progress`.
