//! Greenshot-style filename patterns.
//!
//! Supported placeholders:
//! - `${YYYY}` `${MM}` `${DD}` `${hh}` `${mm}` `${ss}` `${ms}`
//! - `${title}` (window title, or "Screenshot"), `${user}`, `${hostname}`
//! - `${capturetime:d"yyyy-MM-dd HH_mm_ss"}` using .NET-style date tokens,
//!   which is Greenshot's default pattern.

use chrono::{DateTime, Local};

pub const DEFAULT_PATTERN: &str = "${capturetime:d\"yyyy-MM-dd HH_mm_ss\"}-${title}";

pub struct PatternContext<'a> {
    pub time: DateTime<Local>,
    pub title: &'a str,
}

/// Expand a pattern into a filename stem (no extension), with characters
/// that are illegal in filenames removed.
pub fn expand(pattern: &str, ctx: &PatternContext) -> String {
    let mut out = String::new();
    let mut rest = pattern;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = find_close(after) else {
            out.push_str(&rest[start..]);
            rest = "";
            break;
        };
        out.push_str(&sanitize(&expand_token(&after[..end], ctx)));
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    let out = sanitize(&out);
    let trimmed = out.trim_matches(|c: char| c == '-' || c == '_' || c.is_whitespace());
    if trimmed.is_empty() {
        "screenshot".into()
    } else {
        trimmed.to_string()
    }
}

/// Find the `}` closing a placeholder, skipping any inside quotes.
fn find_close(s: &str) -> Option<usize> {
    let mut quoted = false;
    for (i, c) in s.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '}' if !quoted => return Some(i),
            _ => {}
        }
    }
    None
}

fn expand_token(token: &str, ctx: &PatternContext) -> String {
    let t = ctx.time;
    match token {
        "YYYY" => t.format("%Y").to_string(),
        "MM" => t.format("%m").to_string(),
        "DD" => t.format("%d").to_string(),
        "hh" => t.format("%H").to_string(),
        "mm" => t.format("%M").to_string(),
        "ss" => t.format("%S").to_string(),
        "ms" => format!("{:03}", t.timestamp_subsec_millis()),
        "title" => {
            if ctx.title.trim().is_empty() {
                "Screenshot".into()
            } else {
                ctx.title.trim().to_string()
            }
        }
        "user" => std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_default(),
        "hostname" | "domain" => hostname(),
        _ => {
            if let Some(fmt) = token.strip_prefix("capturetime:d") {
                let fmt = fmt.trim_matches('"');
                t.format(&dotnet_to_chrono(fmt)).to_string()
            } else if token == "capturetime" {
                t.format("%Y-%m-%d %H_%M_%S").to_string()
            } else {
                String::new()
            }
        }
    }
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_default()
}

/// Translate the common .NET date format tokens Greenshot users know.
fn dotnet_to_chrono(fmt: &str) -> String {
    const TOKENS: &[(&str, &str)] = &[
        ("yyyy", "%Y"),
        ("yy", "%y"),
        ("MMMM", "%B"),
        ("MMM", "%b"),
        ("MM", "%m"),
        ("dddd", "%A"),
        ("ddd", "%a"),
        ("dd", "%d"),
        ("HH", "%H"),
        ("hh", "%I"),
        ("mm", "%M"),
        ("ss", "%S"),
        ("fff", "%3f"),
        ("tt", "%p"),
    ];
    let mut out = String::new();
    let mut s = fmt;
    'outer: while !s.is_empty() {
        for (from, to) in TOKENS {
            if let Some(rest) = s.strip_prefix(from) {
                out.push_str(to);
                s = rest;
                continue 'outer;
            }
        }
        let c = s.chars().next().unwrap();
        if c == '%' {
            out.push_str("%%");
        } else {
            out.push(c);
        }
        s = &s[c.len_utf8()..];
    }
    out
}

fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') && !c.is_control())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ctx(title: &str) -> PatternContext<'_> {
        PatternContext { time: Local.with_ymd_and_hms(2026, 3, 7, 9, 5, 2).unwrap(), title }
    }

    #[test]
    fn greenshot_default() {
        assert_eq!(expand(DEFAULT_PATTERN, &ctx("My: Window")), "2026-03-07 09_05_02-My Window");
    }

    #[test]
    fn empty_title_uses_screenshot() {
        assert_eq!(expand(DEFAULT_PATTERN, &ctx("")), "2026-03-07 09_05_02-Screenshot");
    }

    #[test]
    fn simple_tokens() {
        assert_eq!(expand("shot_${YYYY}${MM}${DD}_${hh}${mm}${ss}", &ctx("x")), "shot_20260307_090502");
    }

    #[test]
    fn unknown_and_unterminated() {
        assert_eq!(expand("a${nope}b", &ctx("x")), "ab");
        assert_eq!(expand("a${YYYY", &ctx("x")), "a${YYYY");
        assert_eq!(expand("", &ctx("x")), "screenshot");
    }
}
