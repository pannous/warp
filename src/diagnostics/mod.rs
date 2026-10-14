//! Diagnostics: the diagnostic type and its rendering, hints toward canonical syntax (normalize), fix-its, crash cards
#[cfg(feature = "native")]
pub mod crash_card;
pub mod diagnostic;
pub mod fixits;
pub mod normalize;
