//! The unread mails of all accounts – single source for tray and window.

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::watch;

use super::model::MailSummary;

#[derive(Debug, Clone, Default, Serialize)]
pub struct AccountMails {
    pub account_id: String,
    pub account: String,
    /// All unread mails on the server; `mails` holds at most the newest ones.
    pub unread_total: usize,
    /// Newest first.
    pub mails: Vec<MailSummary>,
    /// Pinned (flagged) mails, read or not, newest first.
    pub pinned: Vec<MailSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountInfo {
    pub account_id: String,
    pub account: String,
    pub unread_total: usize,
}

/// What the window shows: per-account counts and all mails, newest first.
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub accounts: Vec<AccountInfo>,
    pub mails: Vec<MailSummary>,
    pub pinned: Vec<MailSummary>,
}

pub struct MailStore {
    accounts: Mutex<HashMap<String, AccountMails>>,
    /// Bumped on every change; tray and window listen to it.
    version: watch::Sender<u64>,
}

impl Default for MailStore {
    fn default() -> Self {
        Self { accounts: Mutex::default(), version: watch::channel(0).0 }
    }
}

impl MailStore {
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.version.subscribe()
    }

    fn changed(&self) {
        self.version.send_modify(|v| *v += 1);
    }

    /// Replaces the unread list of an account after a mailbox check.
    pub fn set_account(&self, account_id: &str, account: &str, unread_total: usize, mut mails: Vec<MailSummary>, mut pinned: Vec<MailSummary>) {
        mails.sort_by_key(|m| std::cmp::Reverse(m.uid));
        pinned.sort_by_key(|m| std::cmp::Reverse(m.uid));
        let entry = AccountMails { account_id: account_id.into(), account: account.into(), unread_total, mails, pinned };
        self.accounts.lock().unwrap().insert(account_id.into(), entry);
        self.changed();
    }

    /// Mails already known for an account, by UID – kept across checks with their findings.
    pub fn known(&self, account_id: &str) -> HashMap<u32, MailSummary> {
        self.accounts
            .lock()
            .unwrap()
            .get(account_id)
            .map(|a| a.mails.iter().chain(a.pinned.iter()).map(|m| (m.uid, m.clone())).collect())
            .unwrap_or_default()
    }

    /// Removes a mail right away (read, archived, deleted) – the next check confirms it.
    pub fn remove(&self, account_id: &str, uid: u32) {
        let mut accounts = self.accounts.lock().unwrap();
        let Some(acc) = accounts.get_mut(account_id) else { return };
        let before = acc.mails.len();
        acc.mails.retain(|m| m.uid != uid);
        if acc.mails.len() < before {
            acc.unread_total = acc.unread_total.saturating_sub(1);
            drop(accounts);
            self.changed();
        }
    }

    /// A sender was marked as person or bulk: re-flag the mails already shown.
    pub fn reclassify(&self, addr: &str, bulk: bool) {
        let mut accounts = self.accounts.lock().unwrap();
        let mut changed = false;
        for m in accounts.values_mut().flat_map(|a| a.mails.iter_mut().chain(a.pinned.iter_mut())) {
            if m.from_addr.as_deref().is_some_and(|a| a.eq_ignore_ascii_case(addr)) && m.bulk != bulk {
                m.bulk = bulk;
                changed = true;
            }
        }
        drop(accounts);
        if changed {
            self.changed();
        }
    }

    /// Notify listeners without a data change (e.g. the tray counting mode changed).
    pub fn touch(&self) {
        self.changed();
    }

    /// Unread mails from people among the listed ones (bulk mail excluded).
    pub fn unread_people(&self) -> usize {
        self.accounts.lock().unwrap().values().flat_map(|a| a.mails.iter()).filter(|m| !m.bulk).count()
    }

    pub fn clear(&self) {
        self.accounts.lock().unwrap().clear();
        self.changed();
    }

    pub fn find(&self, id: &str) -> Option<MailSummary> {
        self.accounts.lock().unwrap().values().flat_map(|a| a.mails.iter().chain(a.pinned.iter())).find(|m| m.id == id).cloned()
    }

    pub fn unread_total(&self) -> usize {
        self.accounts.lock().unwrap().values().map(|a| a.unread_total).sum()
    }

    pub fn snapshot(&self) -> Snapshot {
        let accounts = self.accounts.lock().unwrap();
        let mut infos: Vec<AccountInfo> = accounts
            .values()
            .map(|a| AccountInfo { account_id: a.account_id.clone(), account: a.account.clone(), unread_total: a.unread_total })
            .collect();
        infos.sort_by(|a, b| a.account.cmp(&b.account));
        let mut mails: Vec<MailSummary> = accounts.values().flat_map(|a| a.mails.iter().cloned()).collect();
        mails.sort_by_key(|m| std::cmp::Reverse(m.date.unwrap_or(0)));
        let mut pinned: Vec<MailSummary> = accounts.values().flat_map(|a| a.pinned.iter().cloned()).collect();
        pinned.sort_by_key(|m| std::cmp::Reverse(m.date.unwrap_or(0)));
        Snapshot { accounts: infos, mails, pinned }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mail(account: &str, uid: u32, date: i64) -> MailSummary {
        MailSummary {
            id: format!("{account}:1:{uid}"),
            account_id: account.into(),
            account: account.into(),
            uid,
            from_name: None,
            from_addr: None,
            subject: format!("Mail {uid}"),
            date: Some(date),
            preview: String::new(),
            has_attachments: false,
            seen: false,
            flagged: false,
            message_id: None,
            bulk: false,
            findings: vec![],
        }
    }

    #[test]
    fn test_counts_snapshot_and_remove() {
        let store = MailStore::default();
        let mut rx = store.subscribe();
        store.set_account("a", "Arbeit", 5, vec![mail("a", 1, 100), mail("a", 3, 300)], vec![]);
        store.set_account("p", "Privat", 1, vec![mail("p", 9, 200)], vec![mail("p", 4, 50)]);
        assert_eq!(store.snapshot().pinned.len(), 1);
        assert!(store.find("p:1:4").is_some());
        assert!(rx.has_changed().unwrap());
        rx.mark_unchanged();

        assert_eq!(store.unread_total(), 6);
        let snap = store.snapshot();
        assert_eq!(snap.mails.iter().map(|m| m.uid).collect::<Vec<_>>(), vec![3, 9, 1]);
        assert_eq!(snap.accounts[0].account, "Arbeit");

        store.remove("a", 3);
        assert_eq!(store.unread_total(), 5);
        assert!(store.find("a:1:3").is_none());
        assert!(rx.has_changed().unwrap());
        rx.mark_unchanged();

        // Unknown UID: no change, no event.
        store.remove("a", 42);
        assert!(!rx.has_changed().unwrap());
        assert_eq!(store.known("a").len(), 1);
    }
}
