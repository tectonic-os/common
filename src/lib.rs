//! Carries every component the `tect` CLI and the bootc installer both use.
//!
//! A consumer pins this crate by git rev. The crate reaches for no consumer.

pub mod json;
pub mod prompt;
pub mod ui;
