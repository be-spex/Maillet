//! The mail core: checking mailboxes, keeping the unread mails, reading and
//! moving them. Knows nothing about individual features.
pub mod classify;
pub mod imap;
pub mod model;
pub mod parse;
pub mod store;
