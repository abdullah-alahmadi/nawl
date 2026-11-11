//! The language of messages Nawl shows people (validation problems, the app's
//! errors): English unless the app's interface is Arabic. Each message is written in
//! both languages where it's made, with [`tr!`](crate::tr):
//!
//! ```
//! let field = "title";
//! let m = nawl_core::tr!("unknown field {field}", "حقل غير معروف {field}");
//! assert_eq!(m, "unknown field title");
//! ```
//!
//! The command line stays English.

use std::sync::atomic::{AtomicBool, Ordering};

static ARABIC: AtomicBool = AtomicBool::new(false);

/// Sets the language of messages from now on (the app's interface language).
pub fn set_arabic(arabic: bool) {
    ARABIC.store(arabic, Ordering::Relaxed);
}

/// Whether messages are written in Arabic.
pub fn arabic() -> bool {
    ARABIC.load(Ordering::Relaxed)
}

/// A message in the current language: `tr!("English {x}", "عربي {x}")`, with the
/// same `format!` arguments for both.
#[macro_export]
macro_rules! tr {
    ($en:literal, $ar:literal $(, $($args:tt)*)?) => {
        if $crate::msg::arabic() { format!($ar $(, $($args)*)?) } else { format!($en $(, $($args)*)?) }
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn picks_the_language() {
        let n = 3;
        assert_eq!(crate::tr!("{n} problems", "المشكلات: {n}"), "3 problems");
        super::set_arabic(true);
        let ar = crate::tr!("{n} problems", "المشكلات: {n}");
        super::set_arabic(false);
        assert_eq!(ar, "المشكلات: 3");
        assert_eq!(crate::tr!("plain", "عادي"), "plain");
    }
}
