//! Backend language for user-facing messages. The frontend resolves "auto" (system
//! language) and reports the effective language via `set_language`.

use std::sync::atomic::{AtomicBool, Ordering};

static ENGLISH: AtomicBool = AtomicBool::new(true);

pub fn set(lang: &str) {
    ENGLISH.store(!lang.to_lowercase().starts_with("de"), Ordering::Relaxed);
}

pub fn en() -> bool {
    ENGLISH.load(Ordering::Relaxed)
}

#[allow(dead_code)]
pub fn code() -> &'static str {
    if en() {
        "en"
    } else {
        "de"
    }
}

/// `tr!("Deutsch {x}", "English {x}")` – formats the message in the active language.
#[macro_export]
macro_rules! tr {
    ($de:literal, $en:literal $(,)?) => {
        if $crate::i18n::en() { format!($en) } else { format!($de) }
    };
    ($de:literal, $en:literal, $($args:tt)+) => {
        if $crate::i18n::en() { format!($en, $($args)+) } else { format!($de, $($args)+) }
    };
}
