//! Translations for Rustshot's interface, using [Fluent](https://projectfluent.org/).
//!
//! Each language is one file, `locales/<lang>/rustshot.ftl`, compiled into
//! the binary. Look strings up with [`tr!`]:
//!
//! ```ignore
//! tr!("menu-capture-region")
//! tr!("toast-saved", name = file_name)
//! ```
//!
//! The language follows the system unless one is chosen with
//! [`set_language`]. Anything missing from a translation falls back to
//! English, and anything missing from English shows its message id.

use std::borrow::Cow;
use std::sync::{Arc, RwLock};

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

/// A supported language: code, and its name in that language.
pub struct Language {
    pub code: &'static str,
    pub name: &'static str,
    source: &'static str,
}

pub const LANGUAGES: &[Language] = &[
    Language { code: "en", name: "English", source: include_str!("../locales/en/rustshot.ftl") },
    Language { code: "fr", name: "Français", source: include_str!("../locales/fr/rustshot.ftl") },
    Language { code: "es", name: "Español", source: include_str!("../locales/es/rustshot.ftl") },
    Language { code: "de", name: "Deutsch", source: include_str!("../locales/de/rustshot.ftl") },
];

struct State {
    code: &'static str,
    current: FluentBundle<FluentResource>,
    english: FluentBundle<FluentResource>,
}

static STATE: RwLock<Option<Arc<State>>> = RwLock::new(None);

fn bundle(lang: &Language) -> FluentBundle<FluentResource> {
    let id: LanguageIdentifier = lang.code.parse().expect("valid language code");
    let mut bundle = FluentBundle::new_concurrent(vec![id]);
    // No Unicode isolation marks around arguments: they show up as boxes in
    // fonts that lack them, and our UI is left-to-right only.
    bundle.set_use_isolating(false);
    let resource = FluentResource::try_new(lang.source.to_string()).unwrap_or_else(|(res, errors)| {
        log::error!("errors in {} translations: {errors:?}", lang.code);
        res
    });
    if let Err(errors) = bundle.add_resource(resource) {
        log::error!("duplicate messages in {} translations: {errors:?}", lang.code);
    }
    bundle
}

/// The best supported language for the system's locale, e.g. `fr-CA` → `fr`.
pub fn system_language() -> &'static str {
    let locale = sys_locale::get_locale().unwrap_or_default();
    supported(&locale).unwrap_or("en")
}

/// Map any locale string (`de`, `de-AT`, `es_ES.UTF-8`) to a supported code.
pub fn supported(locale: &str) -> Option<&'static str> {
    let primary = locale.split(['-', '_', '.']).next().unwrap_or("").to_ascii_lowercase();
    LANGUAGES.iter().find(|l| l.code == primary).map(|l| l.code)
}

/// Use `code` (or the system language if `None`/unsupported). The
/// `RUSTSHOT_LANG` environment variable (e.g. `RUSTSHOT_LANG=de`) overrides
/// both, which is handy for testing translations.
pub fn set_language(code: Option<&str>) {
    let forced = std::env::var("RUSTSHOT_LANG").ok();
    let code = forced.as_deref().or(code).and_then(supported).unwrap_or_else(system_language);
    let lang = LANGUAGES.iter().find(|l| l.code == code).unwrap_or(&LANGUAGES[0]);
    let state = State { code: lang.code, current: bundle(lang), english: bundle(&LANGUAGES[0]) };
    *STATE.write().unwrap() = Some(Arc::new(state));
    log::info!("interface language: {}", lang.code);
}

/// The language in use.
pub fn current_language() -> &'static str {
    state().code
}

fn state() -> Arc<State> {
    if let Some(s) = STATE.read().unwrap().as_ref() {
        return Arc::clone(s);
    }
    set_language(None);
    Arc::clone(STATE.read().unwrap().as_ref().unwrap())
}

fn format(bundle: &FluentBundle<FluentResource>, id: &str, args: Option<&FluentArgs>) -> Option<String> {
    let message = bundle.get_message(id)?;
    let pattern = message.value()?;
    let mut errors = vec![];
    let text = bundle.format_pattern(pattern, args, &mut errors);
    if !errors.is_empty() {
        log::warn!("translation {id}: {errors:?}");
    }
    Some(text.into_owned())
}

/// Look up a message, with optional named arguments.
pub fn lookup(id: &str, args: &[(&str, FluentValue<'_>)]) -> String {
    let state = state();
    let args = (!args.is_empty()).then(|| {
        let mut a = FluentArgs::new();
        for (k, v) in args {
            a.set(Cow::Owned(k.to_string()), v.clone());
        }
        a
    });
    format(&state.current, id, args.as_ref())
        .or_else(|| format(&state.english, id, args.as_ref()))
        .unwrap_or_else(|| {
            log::warn!("missing translation: {id}");
            id.to_string()
        })
}

pub use fluent_bundle::FluentValue as Value;

/// `tr!("id")` or `tr!("id", name = value, count = n)`.
#[macro_export]
macro_rules! tr {
    ($id:expr) => {
        $crate::lookup($id, &[])
    };
    ($id:expr, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::lookup($id, &[$((stringify!($name), $crate::Value::from($value))),+])
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Message ids: lines starting with a lowercase letter, up to " =".
    fn ids(source: &str) -> BTreeSet<String> {
        source
            .lines()
            .filter(|l| l.chars().next().is_some_and(|c| c.is_ascii_lowercase()))
            .filter_map(|l| l.split_once(" =").map(|(id, _)| id.trim().to_string()))
            .collect()
    }

    #[test]
    fn every_language_parses_and_has_every_message() {
        let english = ids(LANGUAGES[0].source);
        assert!(english.len() > 50, "English file looks empty");
        for lang in LANGUAGES {
            FluentResource::try_new(lang.source.to_string())
                .unwrap_or_else(|(_, e)| panic!("{} has syntax errors: {e:?}", lang.code));
            let have = ids(lang.source);
            let missing: Vec<_> = english.difference(&have).collect();
            let extra: Vec<_> = have.difference(&english).collect();
            assert!(missing.is_empty(), "{} is missing {missing:?}", lang.code);
            assert!(extra.is_empty(), "{} has unknown messages {extra:?}", lang.code);
        }
    }

    #[test]
    fn locale_matching() {
        assert_eq!(supported("fr-CA"), Some("fr"));
        assert_eq!(supported("de_AT.UTF-8"), Some("de"));
        assert_eq!(supported("ja-JP"), None);
    }

    /// One test for everything that switches the global language, so tests
    /// running in parallel can't interfere with each other.
    #[test]
    fn switching_lookup_fallback_and_paragraphs() {
        set_language(Some("fr"));
        assert_eq!(current_language(), "fr");
        assert!(tr!("toast-saved", name = "a.png").contains("a.png"));
        assert_eq!(tr!("no-such-message"), "no-such-message");

        set_language(Some("en"));
        let body = tr!("permission-body");
        assert!(body.contains("background.\n\n1. Click"), "{body:?}");

        set_language(Some("de"));
        assert!(tr!("permission-body").contains("\n\n1. "));
        set_language(Some("en"));
    }
}
