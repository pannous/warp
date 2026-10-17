//! Errors: the diagnostic type and its rendering, fix-its, crash cards
#[cfg(feature = "native")]
pub mod crash_card;
pub mod diagnostic;
pub mod fixits;
pub mod run_time;
