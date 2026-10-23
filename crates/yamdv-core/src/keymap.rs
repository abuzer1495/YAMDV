//! Keymap (E.7, Appendix F, FUT-05): every rebindable action with its default chords, the
//! `typora` preset (F.6), user overrides from the `[shortcuts]` INI section, conflict
//! detection and the chord → default-chord translation used by the window layer.
//!
//! The window keeps its hard-wired handlers for the *default* chords; a pressed chord is looked
//! up here and, when bound to an action, rewritten to that action's first default chord. A
//! default chord that no longer belongs to its action is swallowed.

use crate::i18n::Lang;

/// Modifier + virtual-key combination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chord {
    /// Ctrl held.
    pub ctrl: bool,
    /// Shift held.
    pub shift: bool,
    /// Alt held.
    pub alt: bool,
    /// Windows virtual-key code.
    pub vk: u16,
}

/// Where an action applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Read and edit modes.
    Global,
    /// Edit modes (F2/F3) only.
    Edit,
}

/// One rebindable action.
#[derive(Debug, Clone, Copy)]
pub struct ActionDef {
    /// Stable snake_case id (E.7).
    pub id: &'static str,
    /// English label.
    pub en: &'static str,
    /// Turkish label.
    pub tr: &'static str,
    /// Default chords (`default` preset); the first one is the canonical chord.
    pub keys: &'static [&'static str],
    /// Scope.
    pub scope: Scope,
}

macro_rules! actions {
    ($($id:literal, $en:literal, $tr:literal, [$($k:literal),*], $s:ident;)*) => {
        /// Every action that has a binding today (F.1, F.3), in Settings order.
        pub const ACTIONS: &[ActionDef] = &[$(ActionDef {
            id: $id, en: $en, tr: $tr, keys: &[$($k),*], scope: Scope::$s,
        },)*];
    };
}

actions! {
    "open_file", "Open file", "Dosya aç", ["Ctrl+O"], Global;
    "close_window", "Close window", "Pencereyi kapat", ["Ctrl+W"], Global;
    "reload", "Reload", "Yeniden yükle", ["F5"], Global;
    "toggle_sidebar", "Show outline", "Ana hattı göster", ["Ctrl+\\"], Global;
    "toggle_favorite", "Add/remove favorite", "Sık kullanılana ekle/kaldır", ["Ctrl+D"], Global;
    "find", "Find", "Bul", ["Ctrl+F"], Global;
    "find_next", "Find next", "Sonrakini bul", ["Ctrl+G"], Global;
    "find_prev", "Find previous", "Öncekini bul", ["Ctrl+Shift+G"], Global;
    "font_bigger", "Bigger text", "Metni büyüt", ["Ctrl+=", "Ctrl+Num+", "Ctrl+Up"], Global;
    "font_smaller", "Smaller text", "Metni küçült", ["Ctrl+-", "Ctrl+Num-", "Ctrl+Down"], Global;
    "font_reset", "Reset text size", "Metin boyutunu sıfırla", ["Ctrl+0"], Global;
    "column_wider", "Wider column", "Sütunu genişlet", ["Alt+Up"], Global;
    "column_narrower", "Narrower column", "Sütunu daralt", ["Alt+Down"], Global;
    "column_reset", "Reset column width", "Sütun genişliğini sıfırla", ["Alt+0"], Global;
    "live_edit", "Live edit", "Canlı düzenleme", ["F2"], Global;
    "split_edit", "Split edit", "Bölünmüş düzenleme", ["F3"], Global;
    "external_editor", "Open in external editor", "Harici düzenleyicide aç", ["F4", "Ctrl+E"], Global;
    "export_pdf", "Export PDF", "PDF dışa aktar", ["Ctrl+Shift+E"], Global;
    "print", "Print", "Yazdır", ["Ctrl+P"], Global;
    "settings", "Settings", "Ayarlar", ["Ctrl+,"], Global;
    "shortcuts_help", "Keyboard shortcuts", "Klavye kısayolları", ["F1"], Global;
    "fullscreen", "Full screen", "Tam ekran", ["F11"], Global;
    "nav_back", "Back", "Geri", ["Alt+Left"], Global;
    "nav_forward", "Forward", "İleri", ["Alt+Right"], Global;
    "copy", "Copy", "Kopyala", ["Ctrl+C"], Global;
    "copy_markdown", "Copy as Markdown", "Markdown olarak kopyala", ["Ctrl+Shift+C"], Global;
    "select_all", "Select all", "Tümünü seç", ["Ctrl+A"], Global;
    "save", "Save", "Kaydet", ["Ctrl+S"], Edit;
    "undo", "Undo", "Geri al", ["Ctrl+Z"], Edit;
    "redo", "Redo", "Yinele", ["Ctrl+Y", "Ctrl+Shift+Z"], Edit;
    "cut", "Cut", "Kes", ["Ctrl+X"], Edit;
    "paste", "Paste", "Yapıştır", ["Ctrl+V"], Edit;
    "bold", "Bold", "Kalın", ["Ctrl+B"], Edit;
    "italic", "Italic", "İtalik", ["Ctrl+I"], Edit;
    "strikethrough", "Strikethrough", "Üstü çizili", ["Ctrl+Shift+X"], Edit;
    "inline_code", "Inline code", "Satır içi kod", ["Ctrl+`"], Edit;
    "link", "Link", "Bağlantı", ["Ctrl+K"], Edit;
    "heading_1", "Heading 1", "Başlık 1", ["Ctrl+1"], Edit;
    "heading_2", "Heading 2", "Başlık 2", ["Ctrl+2"], Edit;
    "heading_3", "Heading 3", "Başlık 3", ["Ctrl+3"], Edit;
    "heading_4", "Heading 4", "Başlık 4", ["Ctrl+4"], Edit;
    "heading_5", "Heading 5", "Başlık 5", ["Ctrl+5"], Edit;
    "heading_6", "Heading 6", "Başlık 6", ["Ctrl+6"], Edit;
    "paragraph", "Paragraph", "Paragraf", ["Ctrl+Shift+0"], Edit;
    "quote", "Quote", "Alıntı", ["Ctrl+Shift+Q"], Edit;
    "bullet_list", "Bulleted list", "Madde işaretli liste", ["Ctrl+Shift+8"], Edit;
    "numbered_list", "Numbered list", "Numaralı liste", ["Ctrl+Shift+7"], Edit;
    "task_list", "Task list", "Görev listesi", ["Ctrl+Shift+9"], Edit;
    "toggle_task", "Toggle task", "Görevi işaretle", ["Ctrl+Enter"], Edit;
    "code_block", "Code block", "Kod bloğu", ["Ctrl+Shift+K"], Edit;
    "insert_table", "Insert table", "Tablo ekle", ["Ctrl+T"], Edit;
    "heading_jump", "Go to heading", "Başlığa git", ["Ctrl+Shift+O"], Global;
    "mind_map", "Mind map", "Zihin haritası", ["Ctrl+Shift+M"], Global;
    "sidebar_contents", "Sidebar: Contents", "Kenar çubuğu: İçindekiler", ["Ctrl+Shift+1"], Global;
    "sidebar_history", "Sidebar: History", "Kenar çubuğu: Geçmiş", ["Ctrl+Shift+2"], Global;
    "sidebar_favorites", "Sidebar: Favorites", "Kenar çubuğu: Sık kullanılanlar", ["Ctrl+Shift+3"], Global;
    "sidebar_files", "Sidebar: Files", "Kenar çubuğu: Dosyalar", ["Ctrl+Shift+4"], Global;
    "focus_next_region", "Next region", "Sonraki bölge", ["F6"], Global;
    "focus_prev_region", "Previous region", "Önceki bölge", ["Shift+F6"], Global;
    "replace", "Find and replace", "Bul ve değiştir", ["Ctrl+H"], Global;
    "highlight", "Highlight", "Vurgula", ["Ctrl+Shift+H"], Edit;
    "duplicate_line", "Duplicate line", "Satırı çoğalt", ["Ctrl+Shift+D"], Edit;
    "select_line", "Select line", "Satırı seç", ["Ctrl+L"], Edit;
    "move_line_up", "Move line up", "Satırı yukarı taşı", ["Alt+Up"], Edit;
    "move_line_down", "Move line down", "Satırı aşağı taşı", ["Alt+Down"], Edit;
    "paste_plain", "Paste as plain text", "Düz metin olarak yapıştır", ["Ctrl+Shift+V"], Edit;
}

/// F.6 `typora` preset: (action id, chords) replacing the defaults.
pub const TYPORA: &[(&str, &str)] = &[
    ("split_edit", "Ctrl+/"),
    ("toggle_sidebar", "Ctrl+Shift+L"),
    ("inline_code", "Ctrl+Shift+`"),
    ("strikethrough", "Alt+Shift+5"),
    ("bullet_list", "Ctrl+Shift+]"),
    ("numbered_list", "Ctrl+Shift+["),
    ("paragraph", "Ctrl+0"),
    ("font_reset", "Ctrl+Shift+Alt+0"),
];

const NAMED: &[(&str, u16)] = &[
    ("Enter", 0x0D),
    ("Space", 0x20),
    ("Tab", 0x09),
    ("Esc", 0x1B),
    ("Backspace", 0x08),
    ("Delete", 0x2E),
    ("Insert", 0x2D),
    ("Home", 0x24),
    ("End", 0x23),
    ("PgUp", 0x21),
    ("PgDn", 0x22),
    ("Up", 0x26),
    ("Down", 0x28),
    ("Left", 0x25),
    ("Right", 0x27),
    ("Num+", 0x6B),
    ("Num-", 0x6D),
    (";", 0xBA),
    ("=", 0xBB),
    (",", 0xBC),
    ("-", 0xBD),
    (".", 0xBE),
    ("/", 0xBF),
    ("`", 0xC0),
    ("[", 0xDB),
    ("\\", 0xDC),
    ("]", 0xDD),
    ("'", 0xDE),
];

/// Two actions compete for a chord only within one scope: an edit action may reuse a global
/// chord, and wins in edit modes (e.g. `Alt+↑` moves the line in F2/F3, widens the column in read).
fn same_scope(i: usize, j: usize) -> bool {
    match (ACTIONS.get(i), ACTIONS.get(j)) {
        (Some(a), Some(b)) => a.scope == b.scope,
        _ => true,
    }
}

/// Display name of a key (`None` for keys that cannot be bound).
pub fn key_name(vk: u16) -> Option<String> {
    match vk {
        0x30..=0x39 | 0x41..=0x5A => char::from_u32(u32::from(vk)).map(String::from),
        0x60..=0x69 => Some(format!("Num{}", vk - 0x60)),
        0x70..=0x87 => Some(format!("F{}", vk - 0x6F)),
        _ => NAMED
            .iter()
            .find(|(_, v)| *v == vk)
            .map(|(n, _)| (*n).to_string()),
    }
}

impl Chord {
    /// Parses `Ctrl+Shift+E` style text (case-insensitive; arrows also as `↑↓←→`).
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let mut c = Self {
            ctrl: false,
            shift: false,
            alt: false,
            vk: 0,
        };
        // A trailing "+" key ("Ctrl++") is not used; "Num+" is handled as a whole.
        let mut rest = s;
        loop {
            let lower = rest.to_ascii_lowercase();
            if let Some(r) = lower.strip_prefix("ctrl+") {
                c.ctrl = true;
                rest = &rest[rest.len() - r.len()..];
            } else if let Some(r) = lower.strip_prefix("shift+") {
                c.shift = true;
                rest = &rest[rest.len() - r.len()..];
            } else if let Some(r) = lower.strip_prefix("alt+") {
                c.alt = true;
                rest = &rest[rest.len() - r.len()..];
            } else {
                break;
            }
        }
        let key = match rest {
            "↑" => "Up",
            "↓" => "Down",
            "←" => "Left",
            "→" => "Right",
            k => k,
        };
        c.vk = if let [b] = key.as_bytes()
            && b.is_ascii_alphanumeric()
        {
            u16::from(b.to_ascii_uppercase())
        } else if let Some(n) = key
            .strip_prefix(['F', 'f'])
            .and_then(|n| n.parse::<u16>().ok())
            .filter(|n| (1..=24).contains(n))
        {
            0x6F + n
        } else if let Some(n) = key
            .strip_prefix("Num")
            .and_then(|n| n.parse::<u16>().ok())
            .filter(|n| *n <= 9)
        {
            0x60 + n
        } else {
            NAMED
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(key))
                .map(|(_, v)| *v)?
        };
        Some(c)
    }

    /// `Ctrl+Shift+Alt+Key` text (arrows shown as `↑↓←→`).
    pub fn display(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("Ctrl+");
        }
        if self.shift {
            s.push_str("Shift+");
        }
        if self.alt {
            s.push_str("Alt+");
        }
        let k = key_name(self.vk).unwrap_or_else(|| format!("0x{:02X}", self.vk));
        s.push_str(match k.as_str() {
            "Up" => "\u{2191}",
            "Down" => "\u{2193}",
            "Left" => "\u{2190}",
            "Right" => "\u{2192}",
            k => k,
        });
        s
    }

    /// Canonical INI text (ASCII arrows).
    pub fn text(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("Ctrl+");
        }
        if self.shift {
            s.push_str("Shift+");
        }
        if self.alt {
            s.push_str("Alt+");
        }
        s.push_str(&key_name(self.vk).unwrap_or_default());
        s
    }

    /// A chord the capture field accepts: a bindable key plus Ctrl or Alt, or a function key.
    pub fn bindable(&self) -> bool {
        key_name(self.vk).is_some()
            && !matches!(self.vk, 0x10..=0x12)
            && (self.ctrl || self.alt || (0x70..=0x87).contains(&self.vk))
    }
}

fn parse_list(s: &str) -> Vec<Chord> {
    // "Ctrl+," splits into "Ctrl+" and "": re-join the comma key.
    s.split(',')
        .collect::<Vec<_>>()
        .join("\u{1}")
        .replace("+\u{1}", "+,")
        .split('\u{1}')
        .filter_map(Chord::parse)
        .collect()
}

/// Preset + overrides; `binds[i]` belongs to `ACTIONS[i]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Keymap {
    /// `default` or `typora`.
    pub preset: String,
    binds: Vec<Vec<Chord>>,
}

/// Result of [`Keymap::translate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Translate {
    /// Not a keymap chord: handle as is.
    Pass,
    /// A default chord whose action moved elsewhere: ignore.
    Swallow,
    /// Bound: handle as this (default) chord instead.
    To(Chord),
}

/// Index of the action `id`.
pub fn index(id: &str) -> Option<usize> {
    ACTIONS.iter().position(|a| a.id == id)
}

/// Localized label of an action.
pub fn label(a: &ActionDef, lang: Lang) -> &'static str {
    match lang {
        Lang::En => a.en,
        Lang::Tr => a.tr,
    }
}

fn defaults(a: &ActionDef) -> Vec<Chord> {
    a.keys.iter().filter_map(|k| Chord::parse(k)).collect()
}

/// Chords of action `i` under `preset` (no overrides).
pub fn preset_binds(preset: &str, i: usize) -> Vec<Chord> {
    let Some(a) = ACTIONS.get(i) else {
        return Vec::new();
    };
    if preset == "typora"
        && let Some((_, k)) = TYPORA.iter().find(|(id, _)| *id == a.id)
    {
        return parse_list(k);
    }
    defaults(a)
}

impl Keymap {
    /// Builds the keymap from the preset and `(action id, chord list)` overrides; an empty list
    /// unbinds. Unknown ids and unparsable chords are ignored.
    pub fn new<'a>(preset: &str, overrides: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        let preset = if preset == "typora" {
            "typora"
        } else {
            "default"
        };
        let mut binds: Vec<Vec<Chord>> = (0..ACTIONS.len())
            .map(|i| preset_binds(preset, i))
            .collect();
        for (id, v) in overrides {
            if let Some(slot) = index(id).and_then(|i| binds.get_mut(i)) {
                let list = parse_list(v);
                if list.is_empty() && !v.trim().is_empty() {
                    continue;
                }
                *slot = list;
            }
        }
        Self {
            preset: preset.to_string(),
            binds,
        }
    }

    /// Chords of action `i`.
    pub fn binds(&self, i: usize) -> &[Chord] {
        self.binds.get(i).map_or(&[], Vec::as_slice)
    }

    /// Display text of action `id`'s first chord (`""` when unbound or unknown).
    pub fn shown(&self, id: &str) -> String {
        index(id)
            .and_then(|i| self.binds(i).first())
            .map(Chord::display)
            .unwrap_or_default()
    }

    /// Whether action `i` differs from the preset.
    pub fn overridden(&self, i: usize) -> bool {
        self.binds(i) != preset_binds(&self.preset, i).as_slice()
    }

    /// The action (other than `except`) that already uses `c`.
    pub fn conflict(&self, c: Chord, except: usize) -> Option<usize> {
        (0..ACTIONS.len())
            .find(|&i| i != except && same_scope(i, except) && self.binds(i).contains(&c))
    }

    /// Id of the action bound to `c` (edit actions first in edit modes), F-27.
    pub fn action_of(&self, c: Chord, edit: bool) -> Option<&'static str> {
        let find = |scope: Scope| {
            ACTIONS
                .iter()
                .enumerate()
                .find(|(i, a)| a.scope == scope && self.binds(*i).contains(&c))
                .map(|(_, a)| a.id)
        };
        if edit {
            find(Scope::Edit).or_else(|| find(Scope::Global))
        } else {
            find(Scope::Global)
        }
    }

    /// Binds action `i` to `chords`; returns the `[shortcuts]` INI value to store, or the index of
    /// the conflicting action (the map is left unchanged).
    pub fn set(&mut self, i: usize, chords: Vec<Chord>) -> Result<String, usize> {
        if let Some(other) = chords.iter().find_map(|c| self.conflict(*c, i)) {
            return Err(other);
        }
        let text = chords
            .iter()
            .map(Chord::text)
            .collect::<Vec<_>>()
            .join(", ");
        if let Some(slot) = self.binds.get_mut(i) {
            *slot = chords;
        }
        Ok(text)
    }

    /// All conflicting pairs (i, j), i < j.
    pub fn conflicts(&self) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for i in 0..ACTIONS.len() {
            for c in self.binds(i) {
                if let Some(j) =
                    (i + 1..ACTIONS.len()).find(|&j| same_scope(i, j) && self.binds(j).contains(c))
                {
                    out.push((i, j));
                }
            }
        }
        out
    }

    /// Maps a pressed chord to the default chord the hard-wired handlers understand. In edit
    /// modes edit actions win over global ones.
    pub fn translate(&self, c: Chord, edit: bool) -> Translate {
        let find = |scope: Scope| {
            (0..ACTIONS.len()).find(|&i| {
                ACTIONS.get(i).is_some_and(|a| a.scope == scope) && self.binds(i).contains(&c)
            })
        };
        let hit = if edit {
            find(Scope::Edit).or_else(|| find(Scope::Global))
        } else {
            find(Scope::Global)
        };
        if let Some(i) = hit {
            return ACTIONS
                .get(i)
                .and_then(|a| defaults(a).first().copied())
                .map_or(Translate::Pass, Translate::To);
        }
        let is_default = ACTIONS
            .iter()
            .any(|a| (edit || a.scope == Scope::Global) && defaults(a).contains(&c));
        if is_default {
            Translate::Swallow
        } else {
            Translate::Pass
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(s: &str) -> Chord {
        Chord::parse(s).expect(s)
    }

    #[test]
    fn parse_display_round_trip() {
        for a in ACTIONS {
            for k in a.keys {
                let c = ch(k);
                assert_eq!(Chord::parse(&c.text()), Some(c), "{k}");
                assert!(c.bindable(), "{k}");
            }
        }
        for (_, k) in TYPORA {
            assert!(ch(k).bindable());
        }
        assert_eq!(ch("Ctrl+Shift+E").display(), "Ctrl+Shift+E");
        assert_eq!(ch("alt+up").display(), "Alt+\u{2191}");
        assert_eq!(ch("Ctrl+,").vk, 0xBC);
        assert_eq!(
            ch("Ctrl+Alt+F"),
            Chord {
                ctrl: true,
                shift: false,
                alt: true,
                vk: 0x46
            }
        );
        assert!(Chord::parse("Ctrl+Nope").is_none());
        assert!(!ch("A").bindable());
        assert!(ch("F7").bindable());
        assert_eq!(parse_list("Ctrl+,, F4"), vec![ch("Ctrl+,"), ch("F4")]);
    }

    #[test]
    fn presets_have_no_conflicts_and_typora_diffs() {
        let d = Keymap::new("default", []);
        assert!(d.conflicts().is_empty(), "{:?}", d.conflicts());
        let t = Keymap::new("typora", []);
        assert!(t.conflicts().is_empty(), "{:?}", t.conflicts());
        assert_eq!(t.shown("split_edit"), "Ctrl+/");
        assert_eq!(t.shown("font_reset"), "Ctrl+Shift+Alt+0");
        assert_eq!(d.shown("split_edit"), "F3");
        let changed = (0..ACTIONS.len())
            .filter(|&i| t.binds(i) != d.binds(i))
            .count();
        assert_eq!(changed, TYPORA.len());
        assert!(!t.overridden(0));
    }

    #[test]
    fn overrides_conflicts_and_translation() {
        let mut k = Keymap::new(
            "default",
            [("find", "Ctrl+Alt+F"), ("print", ""), ("x", "F9")],
        );
        let find = index("find").unwrap();
        assert!(k.overridden(find));
        assert_eq!(k.shown("print"), "");
        // Rebinding find works; old Ctrl+F is swallowed; unrelated keys pass.
        assert_eq!(
            k.translate(ch("Ctrl+Alt+F"), false),
            Translate::To(ch("Ctrl+F"))
        );
        assert_eq!(k.translate(ch("Ctrl+F"), false), Translate::Swallow);
        assert_eq!(k.translate(ch("Ctrl+P"), false), Translate::Swallow);
        assert_eq!(k.translate(ch("Ctrl+J"), false), Translate::Pass);
        assert_eq!(
            k.translate(ch("Ctrl+O"), false),
            Translate::To(ch("Ctrl+O"))
        );
        // Conflicts are refused and leave the map unchanged.
        let open = index("open_file").unwrap();
        assert_eq!(
            k.set(open, vec![ch("Ctrl+D")]),
            Err(index("toggle_favorite").unwrap())
        );
        assert_eq!(k.shown("open_file"), "Ctrl+O");
        assert_eq!(
            k.set(open, vec![ch("Ctrl+Shift+J")]).as_deref(),
            Ok("Ctrl+Shift+J")
        );
        // Edit actions only translate in edit mode.
        assert_eq!(k.translate(ch("Ctrl+B"), false), Translate::Pass);
        assert_eq!(k.translate(ch("Ctrl+B"), true), Translate::To(ch("Ctrl+B")));
        let t = Keymap::new("typora", []);
        assert_eq!(
            t.translate(ch("Ctrl+0"), true),
            Translate::To(ch("Ctrl+Shift+0"))
        );
        assert_eq!(t.translate(ch("Ctrl+0"), false), Translate::Swallow);
        assert_eq!(
            t.translate(ch("Ctrl+Shift+Alt+0"), false),
            Translate::To(ch("Ctrl+0"))
        );
        assert_eq!(t.translate(ch("Ctrl+/"), false), Translate::To(ch("F3")));
    }

    #[test]
    fn f27_nav_keys_are_actions_and_scopes_overlay() {
        let mut k = Keymap::new("default", [("heading_jump", "Ctrl+Shift+P")]);
        assert_eq!(k.action_of(ch("Ctrl+Shift+P"), false), Some("heading_jump"));
        assert_eq!(k.action_of(ch("Ctrl+Shift+O"), false), None);
        assert_eq!(k.translate(ch("Ctrl+Shift+O"), false), Translate::Swallow);
        for (id, c) in [
            ("mind_map", "Ctrl+Shift+M"),
            ("sidebar_files", "Ctrl+Shift+4"),
            ("focus_next_region", "F6"),
            ("focus_prev_region", "Shift+F6"),
        ] {
            assert_eq!(k.action_of(ch(c), false), Some(id));
            assert_eq!(Keymap::new("typora", []).action_of(ch(c), true), Some(id));
        }
        // Alt+Up: column in read mode, line move in edit modes; no conflict across scopes.
        assert_eq!(k.action_of(ch("Alt+Up"), false), Some("column_wider"));
        assert_eq!(k.action_of(ch("Alt+Up"), true), Some("move_line_up"));
        assert!(k.conflicts().is_empty());
        let pal = index("heading_jump").unwrap();
        assert_eq!(
            k.set(pal, vec![ch("Ctrl+D")]),
            Err(index("toggle_favorite").unwrap())
        );
    }
}
