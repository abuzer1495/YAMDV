//! UI strings (E.13, R-I18N-01/02): `Msg` enum + compile-time EN/TR tables.

/// UI language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    /// English.
    En,
    /// Türkçe.
    Tr,
}

macro_rules! messages {
    ($($id:ident => $en:expr, $tr:expr;)*) => {
        /// Every UI string used by Phase 002.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Msg { $(#[doc = $en] $id,)* }
        /// All message ids (for completeness tests).
        pub const ALL: &[Msg] = &[$(Msg::$id,)*];
        /// Localized text of `m`.
        pub fn tr(lang: Lang, m: Msg) -> &'static str {
            match (lang, m) {
                $((Lang::En, Msg::$id) => $en, (Lang::Tr, Msg::$id) => $tr,)*
            }
        }
    };
}

messages! {
    EmptyState => "Drop a Markdown file here or press Ctrl+O", "Bir Markdown dosyasını buraya bırakın veya Ctrl+O'ya basın";
    CannotOpen => "Cannot open file", "Dosya açılamıyor";
    NotFound => "The file does not exist.", "Dosya bulunamadı.";
    InvalidPath => "This path is not allowed.", "Bu yola izin verilmiyor.";
    ReadError => "The file could not be read.", "Dosya okunamadı.";
    TooLarge => "The file is larger than 256 MiB and cannot be opened.", "Dosya 256 MiB'tan büyük ve açılamaz.";
    LargeTitle => "Large file", "Büyük dosya";
    LargeBody => "This file is larger than 20 MiB.", "Bu dosya 20 MiB'tan büyük.";
    OpenPlain => "Open as plain text (fast)", "Düz metin olarak aç (hızlı)";
    RenderAnyway => "Render anyway", "Yine de işle";
    Cancel => "Cancel", "İptal";
    BinaryTitle => "Binary file", "İkili dosya";
    BinaryBody => "This looks like a binary file.", "Bu bir ikili dosya gibi görünüyor.";
    OpenAnyway => "Open anyway", "Yine de aç";
    DropFolders => "Drop Markdown files, not folders", "Klasör değil, Markdown dosyaları bırakın";
    DialogHostFailed => "The file dialog could not be opened", "Dosya penceresi açılamadı";
    FilterMarkdown => "Markdown", "Markdown";
    FilterText => "Text", "Metin";
    FilterAll => "All files", "Tüm dosyalar";
    CalloutNote => "Note", "Not";
    CalloutTip => "Tip", "İpucu";
    CalloutImportant => "Important", "Önemli";
    CalloutWarning => "Warning", "Uyarı";
    CalloutCaution => "Caution", "Dikkat";
    LabelTable => "table", "tablo";
    LabelHtml => "html", "html";
    LabelMath => "math", "matematik";
    LabelFootnotes => "footnotes", "dipnotlar";
    LabelFrontMatter => "front matter", "ön bilgi";
    LabelDetails => "details", "ayrıntılar";
    Image => "image", "görsel";
}

/// Resolves the UI language from the setting (`system`/`en`/`tr`) and the user's UI language
/// tag (e.g. from `GetUserDefaultUILanguage` → `tr-TR`). ASCII case-insensitive (R-I18N-04).
pub fn resolve(setting: &str, system_tag: &str) -> Lang {
    if setting.eq_ignore_ascii_case("tr") {
        return Lang::Tr;
    }
    if setting.eq_ignore_ascii_case("en") {
        return Lang::En;
    }
    let t = system_tag.as_bytes();
    if t.get(..2).is_some_and(|p| p.eq_ignore_ascii_case(b"tr"))
        && matches!(t.get(2), None | Some(b'-' | b'_'))
    {
        Lang::Tr
    } else {
        Lang::En
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r_i18n_02_complete() {
        for m in ALL {
            assert!(!tr(Lang::En, *m).is_empty(), "{m:?}");
            assert!(!tr(Lang::Tr, *m).is_empty(), "{m:?}");
        }
    }

    #[test]
    fn r_i18n_01_resolution() {
        assert_eq!(resolve("system", "tr-TR"), Lang::Tr);
        assert_eq!(resolve("system", "TR"), Lang::Tr);
        assert_eq!(resolve("system", "en-US"), Lang::En);
        assert_eq!(resolve("system", "tru"), Lang::En);
        assert_eq!(resolve("EN", "tr-TR"), Lang::En);
        assert_eq!(resolve("Tr", "de-DE"), Lang::Tr);
    }
}
