#![recursion_limit = "1024"]

pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Safari/537.36";

pub mod config;
pub mod echo_server;
pub mod generators;
pub mod getatext;
pub mod http;
pub mod imap_otp;
pub mod output;
pub mod pvacodes;
pub mod px;
pub mod sms;
pub mod types;
pub mod walmart;
