//! Link classification and policy (R-LINK-01…06, S-LINK-01…04, S-EXEC-02/04). Pure: the
//! platform layer performs the action this module decides. Contains our own strict URL parser
//! (no `url` crate, §4.8).

// Byte indices are bounded by the preceding `find`/length checks.
#![allow(clippy::indexing_slicing)]

use std::fmt::Write as _;

/// Maximum serialized URL length (S-LINK-02).
pub const MAX_URL: usize = 2048;

/// Local extensions that may be opened with the default app after confirmation (S-LINK-03).
pub const OPEN_ALLOWLIST: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "webp", "tif", "tiff", "ico", "svg", "pdf", "txt", "log",
    "csv", "tsv", "json", "yaml", "yml", "toml", "xml", "docx", "xlsx", "pptx", "odt", "ods",
    "odp", "mp3", "wav", "ogg", "flac", "mp4", "webm", "mkv", "mov",
];

/// Markdown family extensions (opened in YAMDV).
pub const MARKDOWN_EXT: &[&str] = &["md", "markdown", "mdown", "mkd", "mkdn", "mdwn", "mdx"];

/// A parsed external URL (`http`, `https`, `mailto`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    /// Lower-case scheme.
    pub scheme: String,
    /// `user[:pass]` part (suspicious, S-LINK-02).
    pub userinfo: Option<String>,
    /// Host (lower-cased ASCII; Unicode kept as written for IDN display).
    pub host: String,
    /// Port.
    pub port: Option<u16>,
    /// Path + query + fragment, percent-encoded where needed.
    pub rest: String,
}

impl Url {
    /// Host contains non-ASCII characters (show the punycode form too, S-LINK-02).
    pub fn is_idn(&self) -> bool {
        !self.host.is_ascii() || self.host.split('.').any(|l| l.starts_with("xn--"))
    }

    /// Re-serialized URL handed to the default handler.
    pub fn serialize(&self) -> String {
        let mut s = String::with_capacity(self.rest.len() + self.host.len() + 16);
        s.push_str(&self.scheme);
        if self.scheme == "mailto" {
            s.push(':');
            s.push_str(&self.rest);
            return s;
        }
        s.push_str("://");
        if let Some(u) = &self.userinfo {
            s.push_str(u);
            s.push('@');
        }
        s.push_str(&self.host);
        if let Some(p) = self.port {
            let _ = write!(s, ":{p}");
        }
        s.push_str(&self.rest);
        s
    }
}

/// Why a URL was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlError {
    /// Scheme other than http/https/mailto.
    Scheme,
    /// Control characters, unescaped spaces or quotes, bad host/port, too long.
    Malformed,
}

fn percent_encode_rest(s: &str) -> Result<String, UrlError> {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_control() || c == ' ' || c == '"' || c == '\'' || c == '<' || c == '>' || c == '`' {
            return Err(UrlError::Malformed);
        }
        if c == '\\' {
            out.push_str("%5C");
        } else if c.is_ascii() {
            out.push(c);
        } else {
            let mut b = [0u8; 4];
            for byte in c.encode_utf8(&mut b).bytes() {
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
    Ok(out)
}

/// Strict parser for external URLs (S-LINK-01/02).
pub fn parse_url(s: &str) -> Result<Url, UrlError> {
    if s.len() > MAX_URL || s.chars().any(char::is_control) {
        return Err(UrlError::Malformed);
    }
    let colon = s.find(':').ok_or(UrlError::Scheme)?;
    let scheme = s[..colon].to_ascii_lowercase();
    if !scheme
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
    {
        return Err(UrlError::Scheme);
    }
    let after = &s[colon + 1..];
    match scheme.as_str() {
        "mailto" => {
            let rest = percent_encode_rest(after)?;
            let addr = rest.split('?').next().unwrap_or("");
            if !addr.contains('@') || addr.starts_with('@') || addr.ends_with('@') {
                return Err(UrlError::Malformed);
            }
            let host = addr.rsplit('@').next().unwrap_or("").to_ascii_lowercase();
            Ok(Url {
                scheme,
                userinfo: None,
                host,
                port: None,
                rest,
            })
        }
        "http" | "https" => {
            let body = after.strip_prefix("//").ok_or(UrlError::Malformed)?;
            let auth_end = body.find(['/', '?', '#']).unwrap_or(body.len());
            let (auth, rest) = body.split_at(auth_end);
            let (userinfo, hostport) = match auth.rfind('@') {
                Some(p) => (Some(auth[..p].to_string()), &auth[p + 1..]),
                None => (None, auth),
            };
            if userinfo
                .as_deref()
                .is_some_and(|u| u.contains([' ', '"', '\'']))
            {
                return Err(UrlError::Malformed);
            }
            let (host, port) = if hostport.starts_with('[') {
                let close = hostport.find(']').ok_or(UrlError::Malformed)?;
                let h = &hostport[..=close];
                let p = &hostport[close + 1..];
                let port = match p.strip_prefix(':') {
                    Some(n) if !n.is_empty() => Some(n.parse().map_err(|_| UrlError::Malformed)?),
                    Some(_) | None if p.is_empty() || p == ":" => None,
                    _ => return Err(UrlError::Malformed),
                };
                if !h[1..h.len() - 1]
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() || b == b':' || b == b'.')
                {
                    return Err(UrlError::Malformed);
                }
                (h.to_string(), port)
            } else {
                let (h, p) = match hostport.rfind(':') {
                    Some(i) => (&hostport[..i], Some(&hostport[i + 1..])),
                    None => (hostport, None),
                };
                let port = match p {
                    Some("") | None => None,
                    Some(n) => Some(n.parse::<u16>().map_err(|_| UrlError::Malformed)?),
                };
                (h.to_lowercase(), port)
            };
            if host.is_empty()
                || host.starts_with('.')
                || host.contains("..")
                || !host
                    .chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, '-' | '.' | '_' | '[' | ']' | ':'))
            {
                return Err(UrlError::Malformed);
            }
            Ok(Url {
                scheme,
                userinfo,
                host,
                port,
                rest: percent_encode_rest(rest)?,
            })
        }
        _ => Err(UrlError::Scheme),
    }
}

/// Classified link destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// In-document anchor (`#slug`, without `#`; percent-decoded).
    Anchor(String),
    /// External URL (validated).
    External(Url),
    /// Local path relative to the document folder (percent-decoded), with optional fragment.
    Local {
        /// Path as written (forward or back slashes).
        path: String,
        /// Fragment.
        fragment: Option<String>,
    },
    /// Inert scheme or malformed URL (R-LINK-06).
    Blocked,
}

/// Percent-decodes `s` (invalid sequences stay literal; non-UTF-8 results are lossy).
pub fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(h), Some(l)) = (hex(b[i + 1]), b.get(i + 2).and_then(|&c| hex(c)))
        {
            out.push(h * 16 + l);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Classifies a link destination (`wiki`: a `[[wikilink]]` target, R-LINK-03).
pub fn classify(dest: &str, wiki: bool) -> Target {
    let d = dest.trim();
    if let Some(f) = d.strip_prefix('#') {
        return Target::Anchor(percent_decode(f));
    }
    if wiki {
        let (page, frag) = match d.split_once('#') {
            Some((p, f)) => (p, Some(f.to_string())),
            None => (d, None),
        };
        if page.is_empty() {
            return frag.map_or(Target::Blocked, Target::Anchor);
        }
        let has_ext = page
            .rsplit_once('.')
            .is_some_and(|(_, e)| !e.is_empty() && e.len() <= 8 && !e.contains(['/', '\\', ' ']));
        let path = if has_ext {
            page.to_string()
        } else {
            format!("{page}.md")
        };
        return Target::Local {
            path,
            fragment: frag,
        };
    }
    // Scheme: letters then ':' before any '/', '?', '#' (a Windows drive letter is a path).
    let scheme_end = d.find(':').filter(|&p| {
        p > 1
            && d[..p]
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
            && d[..p]
                .bytes()
                .next()
                .is_some_and(|b| b.is_ascii_alphabetic())
    });
    if let Some(p) = scheme_end {
        let scheme = d[..p].to_ascii_lowercase();
        return match scheme.as_str() {
            "http" | "https" | "mailto" => parse_url(d).map_or(Target::Blocked, Target::External),
            _ => Target::Blocked,
        };
    }
    let drive = d.len() >= 2 && d.as_bytes()[0].is_ascii_alphabetic() && d.as_bytes()[1] == b':';
    if d.starts_with(['/', '\\']) || drive || d.is_empty() {
        // Network paths, protocol-relative URLs and absolute paths are never followed.
        return Target::Blocked;
    }
    let (path, fragment) = match d.split_once('#') {
        Some((p, f)) => (p, Some(percent_decode(f))),
        None => (d, None),
    };
    let path = path.split('?').next().unwrap_or("");
    if path.is_empty() {
        return fragment.map_or(Target::Blocked, Target::Anchor);
    }
    Target::Local {
        path: percent_decode(path),
        fragment,
    }
}

/// `general.open_external_links`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalMode {
    /// Ask (default).
    Ask,
    /// Open directly.
    Open,
    /// Never (copy only).
    Never,
}

/// What the platform should do for a local target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalKind {
    /// Markdown family → open in YAMDV.
    Markdown,
    /// Allowlisted document → confirm, then open with the default app.
    Allowlisted,
    /// Anything else → "Blocked for safety" with [Show in Explorer].
    Blocked,
}

/// Local kind of a file name by extension (S-LINK-03).
pub fn local_kind(name: &str) -> LocalKind {
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    // Trailing dots/spaces or ADS (`:`) never count as an allowlisted extension.
    if name.ends_with(['.', ' '])
        || name
            .rsplit(['/', '\\'])
            .next()
            .is_some_and(|f| f.contains(':'))
    {
        return LocalKind::Blocked;
    }
    if MARKDOWN_EXT.contains(&ext.as_str()) {
        LocalKind::Markdown
    } else if OPEN_ALLOWLIST.contains(&ext.as_str()) {
        LocalKind::Allowlisted
    } else {
        LocalKind::Blocked
    }
}

/// Decision for an external URL (R-LINK-04, S-LINK-04): `true` = ask first.
pub fn must_ask(mode: ExternalMode, motw_internet: bool) -> bool {
    motw_internet || mode == ExternalMode::Ask
}

/// `ZoneId` of a `Zone.Identifier` stream's contents (S-LINK-04).
pub fn zone_id(stream: &str) -> Option<u32> {
    stream
        .lines()
        .filter_map(|l| l.trim().strip_prefix("ZoneId="))
        .find_map(|v| v.trim().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s_link_01_schemes() {
        assert!(matches!(
            classify("https://example.com/a b", false),
            Target::Blocked
        ));
        assert!(
            matches!(classify("HTTPS://Example.COM/x", false), Target::External(u) if u.host == "example.com")
        );
        for bad in [
            "javascript:alert(1)",
            "JaVaScRiPt:x",
            "vbscript:x",
            "data:text/html,x",
            "file://host/share",
            "ms-settings:x",
            "search-ms:query=x",
            "custom:thing",
            "//evil.com/x",
            "\\\\server\\share\\x.md",
        ] {
            assert_eq!(classify(bad, false), Target::Blocked, "{bad}");
        }
        assert!(matches!(
            classify("mailto:a@b.org?subject=hi", false),
            Target::External(_)
        ));
        assert_eq!(classify("mailto:nobody", false), Target::Blocked);
    }

    #[test]
    fn s_link_02_parse_and_serialize() {
        let u =
            parse_url("https://user:pw@例え.jp:8443/パス?q=1#f").unwrap_or_else(|_| unreachable!());
        assert_eq!(u.userinfo.as_deref(), Some("user:pw"));
        assert!(u.is_idn());
        assert_eq!(u.port, Some(8443));
        assert_eq!(
            u.serialize(),
            "https://user:pw@例え.jp:8443/%E3%83%91%E3%82%B9?q=1#f"
        );
        assert_eq!(parse_url("http://a.com/\"x"), Err(UrlError::Malformed));
        assert_eq!(parse_url("http://a.com/x\u{7}"), Err(UrlError::Malformed));
        assert_eq!(
            parse_url(&format!("http://a.com/{}", "a".repeat(MAX_URL))),
            Err(UrlError::Malformed)
        );
        assert_eq!(parse_url("http://a..com/"), Err(UrlError::Malformed));
        assert_eq!(parse_url("http://a.com:99999/"), Err(UrlError::Malformed));
        assert!(parse_url("http://[::1]:8080/x").is_ok());
        assert!(
            !parse_url("https://xn--e1afmkfd.xn--p1ai")
                .map(|u| u.userinfo.is_some())
                .unwrap_or(true)
        );
    }

    #[test]
    fn r_link_02_03_local_and_wiki() {
        assert_eq!(
            classify("./docs/Read%20Me.md#Setup", false),
            Target::Local {
                path: "./docs/Read Me.md".into(),
                fragment: Some("Setup".into())
            }
        );
        assert_eq!(
            classify("#my-heading", false),
            Target::Anchor("my-heading".into())
        );
        assert_eq!(
            classify("Page#Heading", true),
            Target::Local {
                path: "Page.md".into(),
                fragment: Some("Heading".into())
            }
        );
        assert_eq!(classify("#Heading", true), Target::Anchor("Heading".into()));
        assert_eq!(classify("C:/x/y.md", false), Target::Blocked);
        assert_eq!(classify("c:\\x\\y.md", false), Target::Blocked);
    }

    #[test]
    fn s_link_03_local_kinds() {
        assert_eq!(local_kind("a.MD"), LocalKind::Markdown);
        assert_eq!(local_kind("pic.png"), LocalKind::Allowlisted);
        for b in [
            "run.exe",
            "x.bat",
            "y.lnk",
            "z.ps1",
            "doc.pdf.",
            "a.png:evil",
            "noext",
        ] {
            assert_eq!(local_kind(b), LocalKind::Blocked, "{b}");
        }
    }

    #[test]
    fn s_link_04_motw() {
        assert_eq!(
            zone_id("[ZoneTransfer]\r\nZoneId=3\r\nReferrerUrl=x"),
            Some(3)
        );
        assert_eq!(zone_id("garbage"), None);
        assert!(must_ask(ExternalMode::Open, true));
        assert!(!must_ask(ExternalMode::Open, false));
        assert!(must_ask(ExternalMode::Ask, false));
    }
}
