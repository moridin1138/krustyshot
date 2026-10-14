# Translating Rustshot

Rustshot's interface text lives in [Fluent](https://projectfluent.org/) files, one per language:

```
crates/rustshot-i18n/locales/en/rustshot.ftl   English (reference)
crates/rustshot-i18n/locales/fr/rustshot.ftl   French
crates/rustshot-i18n/locales/es/rustshot.ftl   Spanish
crates/rustshot-i18n/locales/de/rustshot.ftl   German
```

The French, Spanish and German files are machine-drafted and have not yet been reviewed by native speakers. Corrections are very welcome.

## Fixing a translation

Edit the message in the language's `.ftl` file and keep the message id (left of `=`) unchanged. Keep every `{ $placeholder }` exactly as it is in English; you can move it within the sentence. For multi-line messages, indent every line by four spaces and leave blank lines between paragraphs.

Check your change:

```bash
cargo test -p rustshot-i18n
RUSTSHOT_LANG=fr cargo run -p rustshot-app -- --settings
```

The test fails if a file has a syntax error, a missing message, or a message English doesn't have.

## Adding a language

1. Copy `locales/en/rustshot.ftl` to `locales/<code>/rustshot.ftl`, where `<code>` is the two-letter language code (`it`, `pt`, `nl`, …), and translate it.
2. Add the language to `LANGUAGES` in `crates/rustshot-i18n/src/lib.rs`, with its name written in that language.
3. Run the commands above.

Right-to-left languages (Arabic, Hebrew, Persian) are not supported yet, because the interface toolkit does not lay out right-to-left text. Chinese, Japanese and Korean also need CJK fallback fonts in `crates/rustshot-app/src/fonts.rs`.

## Adding text to the app

Never put user-facing text directly in the code. Add a message to the English file (and the others; the English text is an acceptable placeholder until someone translates it) and look it up with the `tr!` macro:

```rust
use rustshot_i18n::tr;

ui.label(tr!("settings-folder"));
ui.label(tr!("toast-saved", name = file_name));
```
