//! Settings panel model (FUT-31, B.8, prompt 012 §4.1/§4.2): sections, rows, option labels, the
//! search filter and the panel geometry. Pure; the window layer paints and routes input.
//!
//! Only settings whose feature exists today get a row; unimplemented keys stay in
//! `settings.ini` but are not shown.

use crate::dialog::Rect;
use yamdv_core::i18n::Lang;
use yamdv_core::settings::{self, Kind};

/// Panel size before clamping (DIPs).
pub const PANEL_W: f32 = 720.0;
/// Panel height before clamping.
pub const PANEL_H: f32 = 560.0;
/// Margin kept to the window edge on each side (window − 48).
pub const MARGIN: f32 = 24.0;
/// Header height (draggable).
pub const HEADER_H: f32 = 48.0;
/// Navigation column width.
pub const NAV_W: f32 = 180.0;
/// Navigation item height.
pub const NAV_ITEM_H: f32 = 32.0;
/// Search box height.
pub const SEARCH_H: f32 = 30.0;
/// Row height (label + description).
pub const ROW_H: f32 = 56.0;
/// Section heading height (search results).
pub const HEAD_H: f32 = 32.0;
/// Content padding.
pub const PAD: f32 = 20.0;

/// Panel sections in navigation order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    /// Language, title, links, positions, window geometry.
    General,
    /// Mode, scheme, corners, zebra, custom colours.
    Appearance,
    /// Fonts and text metrics.
    Typography,
    /// Outline sidebar.
    Sidebar,
    /// Read-mode rendering.
    Reading,
    /// Edit modes.
    Editing,
    /// Keymap.
    Shortcuts,
    /// External editor.
    ExternalEditor,
    /// PDF, print, HTML, CSV.
    Export,
    /// Default app, context menu, config folder.
    Files,
    /// Offline statement, log, clearing data.
    Privacy,
    /// Version, commit, licence, paths.
    About,
}

/// Every section, in order.
pub const SECTIONS: [Section; 12] = [
    Section::General,
    Section::Appearance,
    Section::Typography,
    Section::Sidebar,
    Section::Reading,
    Section::Editing,
    Section::Shortcuts,
    Section::ExternalEditor,
    Section::Export,
    Section::Files,
    Section::Privacy,
    Section::About,
];

/// Localized section title.
pub fn section_title(s: Section, lang: Lang) -> &'static str {
    let (en, tr) = match s {
        Section::General => ("General", "Genel"),
        Section::Appearance => ("Appearance", "Görünüm"),
        Section::Typography => ("Typography & Layout", "Yazı ve Yerleşim"),
        Section::Sidebar => ("Sidebar", "Kenar çubuğu"),
        Section::Reading => ("Reading", "Okuma"),
        Section::Editing => ("Editing", "Düzenleme"),
        Section::Shortcuts => ("Shortcuts", "Kısayollar"),
        Section::ExternalEditor => ("External editor", "Harici düzenleyici"),
        Section::Export => ("Export & Print", "Dışa aktarma ve Yazdırma"),
        Section::Files => ("Files & Integration", "Dosyalar ve Entegrasyon"),
        Section::Privacy => ("Privacy & Security", "Gizlilik ve Güvenlik"),
        Section::About => ("About", "Hakkında"),
    };
    pick(lang, en, tr)
}

fn pick(lang: Lang, en: &'static str, tr: &'static str) -> &'static str {
    match lang {
        Lang::En => en,
        Lang::Tr => tr,
    }
}

/// How a row is edited.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Control {
    /// Boolean switch.
    Toggle,
    /// Up to four options side by side.
    Segmented,
    /// Option list in a popup.
    Dropdown,
    /// Numeric slider (`step` in setting units).
    Slider {
        /// Lower bound.
        min: f64,
        /// Upper bound.
        max: f64,
        /// Step.
        step: f64,
    },
    /// Single-line text input.
    Text,
    /// Searchable font picker (`true` = monospace first).
    Font(bool),
    /// Push button (the row key names the action).
    Button,
    /// Read-only value; click copies it (About paths).
    Info,
}

/// One row of a section.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Row {
    /// Section.
    pub section: Section,
    /// Setting key (`section.key`) or an action id (`btn.*` / `info.*`).
    pub key: &'static str,
    /// English label.
    pub en: &'static str,
    /// Turkish label.
    pub tr: &'static str,
    /// English description.
    pub den: &'static str,
    /// Turkish description.
    pub dtr: &'static str,
}

macro_rules! rows {
    ($($s:ident $k:literal, $en:literal, $tr:literal, $den:literal, $dtr:literal;)*) => {
        /// Static rows in section order (colours and shortcuts are added by the window layer).
        pub const ROWS: &[Row] = &[$(Row {
            section: Section::$s, key: $k, en: $en, tr: $tr, den: $den, dtr: $dtr,
        },)*];
    };
}

rows! {
    General "general.language", "Language", "Dil", "Interface language", "Arayüz dili";
    General "general.title_format", "Window title", "Pencere başlığı", "File name or full path", "Dosya adı veya tam yol";
    General "general.open_external_links", "External links", "Dış bağlantılar", "What clicking a web link does", "Web bağlantısına tıklayınca ne olacağı";
    General "general.restore_reading_position", "Restore reading position", "Okuma konumunu geri yükle", "Reopen files where you left off", "Dosyaları kaldığınız yerden açar";
    General "general.remember_window_geometry", "Remember window size", "Pencere boyutunu hatırla", "Reuse the last window position and size", "Son pencere konumunu ve boyutunu kullanır";
    Appearance "appearance.mode", "Theme mode", "Tema modu", "Follow Windows, or force light or dark", "Windows'u izle ya da açık/koyu seç";
    Appearance "appearance.scheme", "Color scheme", "Renk şeması", "Document colors", "Belge renkleri";
    Appearance "appearance.corner_style", "Corners", "Köşeler", "Rounded or sharp corners", "Yuvarlak veya keskin köşeler";
    Appearance "appearance.table_zebra", "Striped tables", "Çizgili tablolar", "Alternate row shading", "Satırları dönüşümlü gölgeler";
    Appearance "appearance.use_custom_colors", "Use custom colors", "Özel renkleri kullan", "Apply the colors below (custom-theme.ini)", "Aşağıdaki renkleri uygular (custom-theme.ini)";
    Appearance "btn.reset_colors", "Reset all colors", "Tüm renkleri sıfırla", "Remove every custom color", "Tüm özel renkleri kaldırır";
    Typography "typography.body_font", "Body font", "Metin yazı tipi", "Font of the document text", "Belge metninin yazı tipi";
    Typography "typography.code_font", "Code font", "Kod yazı tipi", "Font of code blocks", "Kod bloklarının yazı tipi";
    Typography "typography.font_size", "Font size", "Yazı boyutu", "Body text size in pixels", "Metin boyutu (piksel)";
    Typography "typography.line_height", "Line height", "Satır yüksekliği", "Spacing between lines", "Satırlar arası boşluk";
    Typography "typography.paragraph_spacing", "Paragraph spacing", "Paragraf aralığı", "Space between paragraphs", "Paragraflar arası boşluk";
    Typography "typography.column_width_pct", "Column width", "Sütun genişliği", "Text column as % of the window", "Metin sütunu (pencerenin %'si)";
    Sidebar "sidebar.visible", "Show outline", "Ana hattı göster", "Table of contents beside the document", "Belgenin yanında içindekiler";
    Sidebar "sidebar.position", "Position", "Konum", "Docked or floating, left or right", "Sabit veya yüzen, sol veya sağ";
    Sidebar "sidebar.width", "Width", "Genişlik", "Sidebar width in pixels", "Kenar çubuğu genişliği (piksel)";
    Sidebar "sidebar.toc_font_size", "Outline font size", "Ana hat yazı boyutu", "Text size of outline entries", "Ana hat öğelerinin boyutu";
    Sidebar "sidebar.toc_max_depth", "Outline depth", "Ana hat derinliği", "Deepest heading level listed", "Listelenen en derin başlık düzeyi";
    Sidebar "sidebar.show_history", "History tab", "Geçmiş sekmesi", "Recently opened files in the sidebar", "Kenar çubuğunda son açılan dosyalar";
    Sidebar "sidebar.show_favorites", "Favorites tab", "Sık kullanılanlar sekmesi", "Starred files in the sidebar", "Kenar çubuğunda yıldızlı dosyalar";
    Sidebar "sidebar.show_files", "Files tab", "Dosyalar sekmesi", "Markdown files in the document's folder", "Belgenin klasöründeki Markdown dosyaları";
    Reading "reading.single_newline_break", "Single newline = line break", "Tek satır sonu = satır kırma", "Treat every newline as a line break", "Her satır sonunu kırma sayar";
    Reading "reading.smart_punctuation", "Smart punctuation", "Akıllı noktalama", "Curly quotes and dashes", "Kıvrık tırnak ve tireler";
    Reading "reading.subscript_tilde", "~subscript~", "~alt simge~", "Single tildes mark subscript", "Tek tilde alt simge yapar";
    Reading "reading.code_line_numbers", "Code line numbers", "Kod satır numaraları", "Numbers beside code blocks", "Kod bloklarında satır numarası";
    Reading "reading.code_wrap", "Wrap code", "Kodu kaydır", "Wrap long code lines", "Uzun kod satırlarını kaydırır";
    Reading "reading.front_matter", "Front matter", "Ön bilgi", "YAML header display", "YAML üst bilgisinin gösterimi";
    Reading "reading.render_math", "Render math", "Matematiği işle", "Typeset $…$ formulas", "$…$ formüllerini dizer";
    Reading "reading.render_mermaid", "Render diagrams", "Diyagramları işle", "Draw Mermaid code blocks", "Mermaid kod bloklarını çizer";
    Reading "reading.animate_gifs", "Animate GIFs", "GIF'leri oynat", "Play animated GIFs while visible (first frame when off)", "Görünürken hareketli GIF'leri oynatır (kapalıyken ilk kare)";
    Reading "reading.smooth_scrolling", "Smooth scrolling", "Yumuşak kaydırma", "Animate wheel and key scrolling", "Tekerlek ve tuş kaydırmasını canlandırır";
    Reading "reading.scrollbar_autohide", "Auto-hide scrollbar", "Kaydırma çubuğunu gizle", "Show the scrollbar only while scrolling", "Kaydırma çubuğunu yalnızca kaydırırken gösterir";
    Reading "reading.word_count_badge", "Word count", "Kelime sayısı", "Badge with words and reading time", "Kelime ve okuma süresi rozeti";
    Reading "reading.task_toggle_in_read_mode", "Tick tasks while reading", "Okurken görevleri işaretle", "Clicking a checkbox edits the file", "Onay kutusuna tıklamak dosyayı düzenler";
    Editing "editing.autosave", "Autosave", "Otomatik kaydet", "Save edits automatically", "Düzenlemeleri otomatik kaydeder";
    Editing "editing.autosave_delay_ms", "Autosave delay", "Otomatik kaydetme gecikmesi", "Milliseconds after the last keystroke", "Son tuştan sonra milisaniye";
    Editing "editing.editor_font", "Editor font", "Düzenleyici yazı tipi", "Body or code font in the editor", "Düzenleyicide metin veya kod yazı tipi";
    Editing "editing.editor_line_numbers", "Editor line numbers", "Düzenleyici satır numaraları", "Line numbers in Split edit", "Bölünmüş düzenlemede satır numaraları";
    Editing "editing.editor_soft_wrap", "Soft wrap", "Yumuşak kaydırma (satır)", "Wrap long lines in the editor", "Düzenleyicide uzun satırları kaydırır";
    Editing "editing.split_ratio", "Split ratio", "Bölme oranı", "Editor share of the window", "Düzenleyicinin pencere payı";
    Editing "editing.split_swapped", "Swap panes", "Bölmeleri değiştir", "Preview on the left", "Önizleme solda";
    Editing "editing.list_indent", "List indent", "Liste girintisi", "Spaces or a tab per level", "Düzey başına boşluk veya sekme";
    Shortcuts "shortcuts.preset", "Preset", "Hazır ayar", "Base keymap; your changes apply on top", "Temel tuş haritası; değişiklikleriniz üstüne uygulanır";
    Shortcuts "btn.reset_shortcuts", "Reset to preset", "Hazır ayara sıfırla", "Remove every custom shortcut", "Tüm özel kısayolları kaldırır";
    ExternalEditor "external_editor.mode", "Editor", "Düzenleyici", "Automatic detection or a custom program", "Otomatik algılama veya özel program";
    ExternalEditor "external_editor.custom_exe", "Program", "Program", "Full path of the editor (.exe)", "Düzenleyicinin tam yolu (.exe)";
    ExternalEditor "external_editor.custom_args", "Arguments", "Bağımsız değişkenler", "{file} {line} are replaced", "{file} {line} yerine konur";
    Export "export.pdf_colors", "PDF colors", "PDF renkleri", "Light paper or the current theme", "Açık kâğıt veya geçerli tema";
    Export "export.pdf_page_size", "PDF page size", "PDF sayfa boyutu", "Automatic follows the region", "Otomatik bölgeyi izler";
    Export "export.pdf_margins_mm", "PDF margins", "PDF kenar boşlukları", "Millimetres on every side", "Her kenarda milimetre";
    Export "export.pdf_page_numbers", "Page numbers", "Sayfa numaraları", "Number PDF and printed pages", "PDF ve basılı sayfaları numaralar";
    Export "export.print_colors", "Print colors", "Yazdırma renkleri", "Light paper or the current theme", "Açık kâğıt veya geçerli tema";
    Export "export.html_colors", "HTML colors", "HTML renkleri", "Light or the current theme", "Açık veya geçerli tema";
    Export "export.html_include_toc", "HTML table of contents", "HTML içindekiler", "Add an outline to exported HTML", "Dışa aktarılan HTML'ye ana hat ekler";
    Export "export.csv_formula_guard", "CSV formula guard", "CSV formül koruması", "Prefix cells that start with = + - @", "= + - @ ile başlayan hücrelere önek koyar";
    Files "btn.default_app", "Default app for .md", "Varsayılan .md uygulaması", "Open Windows settings to choose YAMDV", "YAMDV'yi seçmek için Windows ayarlarını açar";
    Files "integration.context_menu", "Explorer context menu", "Gezgin bağlam menüsü", "\"Open with YAMDV\" on Markdown files", "Markdown dosyalarında \"YAMDV ile aç\"";
    Files "btn.config_folder", "Open config folder", "Yapılandırma klasörünü aç", "settings.ini, state, custom-theme.ini", "settings.ini, durum, custom-theme.ini";
    Files "btn.edit_custom_theme", "Edit custom-theme.ini", "custom-theme.ini dosyasını düzenle", "Opens the color file in your editor", "Renk dosyasını düzenleyicinizde açar";
    Files "btn.edit_settings", "Edit settings.ini", "settings.ini dosyasını düzenle", "Opens the settings file in your editor", "Ayar dosyasını düzenleyicinizde açar";
    Files "info.portable", "Portable mode", "Taşınabilir mod", "yamdv.portable beside the exe keeps data next to it", "Exe yanındaki yamdv.portable verileri yanında tutar";
    Files "btn.install", "Install", "Yükle", "Copy YAMDV to your user programs folder", "YAMDV'yi kullanıcı programları klasörüne kopyalar";
    Privacy "info.offline", "Offline", "Çevrimdışı", "YAMDV never connects to the network", "YAMDV hiçbir zaman ağa bağlanmaz";
    Privacy "privacy.diagnostic_log", "Diagnostic log", "Tanılama günlüğü", "Write yamdv.log to %LOCALAPPDATA%\\YAMDV\\logs (no document text)", "%LOCALAPPDATA%\\YAMDV\\logs içine yamdv.log yazar (belge metni yok)";
    Privacy "btn.clear_history", "Clear history", "Geçmişi temizle", "Forget recently opened files", "Son açılan dosyaları unutur";
    Privacy "btn.clear_favorites", "Clear favorites", "Sık kullanılanları temizle", "Remove every favorite", "Tüm sık kullanılanları kaldırır";
    Privacy "btn.clear_positions", "Clear reading positions", "Okuma konumlarını temizle", "Forget where you left off", "Kaldığınız yerleri unutur";
    About "info.version", "Version", "Sürüm", "", "";
    About "info.commit", "Commit", "Commit", "", "";
    About "info.license", "License", "Lisans", "", "";
    About "info.config_path", "Config folder", "Yapılandırma klasörü", "Click to copy", "Kopyalamak için tıklayın";
    About "info.exe_path", "Program", "Program", "Click to copy", "Kopyalamak için tıklayın";
}

/// Localized label / description of a row.
pub fn row_text(r: &Row, lang: Lang) -> (&'static str, &'static str) {
    match lang {
        Lang::En => (r.en, r.den),
        Lang::Tr => (r.tr, r.dtr),
    }
}

/// The control of a row, derived from the setting kind.
pub fn control(r: &Row) -> Control {
    if r.key.starts_with("btn.") {
        return Control::Button;
    }
    if r.key.starts_with("info.") {
        return Control::Info;
    }
    match r.key {
        "typography.body_font" => return Control::Font(false),
        "typography.code_font" => return Control::Font(true),
        "appearance.scheme" => return Control::Dropdown,
        _ => {}
    }
    match settings::def(r.key).map(|d| d.kind) {
        Some(Kind::Bool) => Control::Toggle,
        Some(Kind::Enum(v)) if v.len() <= 3 => Control::Segmented,
        Some(Kind::Enum(_)) => Control::Dropdown,
        Some(Kind::Int { min, max }) => {
            #[allow(clippy::cast_precision_loss)]
            let (min, max) = (min as f64, max as f64);
            let step = if max - min > 2000.0 { 100.0 } else { 1.0 };
            Control::Slider { min, max, step }
        }
        Some(Kind::Float { min, max }) => Control::Slider {
            min,
            max,
            step: 0.05,
        },
        Some(Kind::Str) | None => Control::Text,
    }
}

/// Enum options of a setting.
pub fn options(key: &str) -> &'static [&'static str] {
    match settings::def(key).map(|d| d.kind) {
        Some(Kind::Enum(v)) => v,
        _ => &[],
    }
}

/// Localized label of an enum value.
pub fn option_label(key: &str, value: &str, lang: Lang) -> String {
    let t = |en: &'static str, tr: &'static str| pick(lang, en, tr).to_string();
    match (key, value) {
        ("general.language", "system") | ("appearance.mode", "system") => t("System", "Sistem"),
        ("general.language", "en") => "English".into(),
        ("general.language", "tr") => "Türkçe".into(),
        (_, "filename") => t("File name", "Dosya adı"),
        (_, "fullpath") => t("Full path", "Tam yol"),
        (_, "ask") => t("Ask", "Sor"),
        (_, "direct") => t("Open", "Aç"),
        (_, "never") => t("Never", "Asla"),
        (_, "light") => t("Light", "Açık"),
        (_, "dark") => t("Dark", "Koyu"),
        (_, "theme") => t("Theme", "Tema"),
        (_, "rounded") => t("Rounded", "Yuvarlak"),
        (_, "sharp") => t("Sharp", "Keskin"),
        (_, "left") => t("Left", "Sol"),
        (_, "right") => t("Right", "Sağ"),
        (_, "left-floating") => t("Left, floating", "Sol, yüzen"),
        (_, "right-floating") => t("Right, floating", "Sağ, yüzen"),
        (_, "collapsed") => t("Collapsed", "Daraltılmış"),
        (_, "expanded") => t("Expanded", "Açık"),
        (_, "hidden") => t("Hidden", "Gizli"),
        (_, "body") => t("Body", "Metin"),
        (_, "code") => t("Code", "Kod"),
        (_, "tab") => t("Tab", "Sekme"),
        ("editing.list_indent", n) => format!("{n} {}", pick(lang, "spaces", "boşluk")),
        (_, "auto") => t("Automatic", "Otomatik"),
        (_, "custom") => t("Custom", "Özel"),
        (_, "a4") => "A4".into(),
        (_, "letter") => "Letter".into(),
        (_, "default") if key == "shortcuts.preset" => t("Default", "Varsayılan"),
        (_, "typora") => "Typora".into(),
        (_, v) => {
            let mut c = v.chars();
            c.next()
                .map(|f| f.to_uppercase().chain(c).collect())
                .unwrap_or_default()
        }
    }
}

/// Folds text for search: lowercase, Turkish letters and common accents to ASCII.
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .filter_map(|c| match c {
            '\u{307}' => None,
            'ı' => Some('i'),
            'ş' => Some('s'),
            'ç' => Some('c'),
            'ğ' => Some('g'),
            'ö' | 'ó' | 'ô' => Some('o'),
            'ü' | 'ú' | 'û' => Some('u'),
            'â' | 'á' | 'à' => Some('a'),
            'é' | 'è' | 'ê' => Some('e'),
            'î' | 'í' => Some('i'),
            c => Some(c),
        })
        .collect()
}

/// A searchable entry: a static row or a dynamic one (colour token, shortcut action).
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// Section the entry lives in.
    pub section: Section,
    /// Label shown.
    pub label: String,
    /// Further searchable text (description, key, current binding …).
    pub extra: String,
}

/// Whether `item` matches `query` (every whitespace-separated word must occur in the section
/// title, label or extra text; empty query matches everything).
pub fn matches(item: &Item, query: &str, lang: Lang) -> bool {
    let hay = fold(&format!(
        "{} {} {}",
        section_title(item.section, lang),
        item.label,
        item.extra
    ));
    fold(query).split_whitespace().all(|w| hay.contains(w))
}

/// Indices of the items matching `query`.
pub fn filter(items: &[Item], query: &str, lang: Lang) -> Vec<usize> {
    (0..items.len())
        .filter(|&i| items.get(i).is_some_and(|it| matches(it, query, lang)))
        .collect()
}

/// Static rows as search items (labels in `lang`, key and description as extra text).
pub fn static_items(lang: Lang) -> Vec<Item> {
    ROWS.iter()
        .map(|r| {
            let (l, d) = row_text(r, lang);
            let opts: Vec<String> = options(r.key)
                .iter()
                .map(|v| option_label(r.key, v, lang))
                .collect();
            Item {
                section: r.section,
                label: l.to_string(),
                extra: format!("{d} {} {}", r.key, opts.join(" ")),
            }
        })
        .collect()
}

/// Localized label of a colour token (`yamdv_core::colors::TOKENS`).
pub fn token_label(token: &str, lang: Lang) -> &'static str {
    let (en, tr) = match token {
        "bg" => ("Background", "Arka plan"),
        "text" => ("Text", "Metin"),
        "heading" => ("Headings", "Başlıklar"),
        "link" => ("Links", "Bağlantılar"),
        "accent" => ("Accent", "Vurgu"),
        "quote_bar" => ("Quote bar", "Alıntı çubuğu"),
        "code_bg" => ("Code background", "Kod arka planı"),
        "border" => ("Borders", "Kenarlıklar"),
        "table_header_bg" => ("Table header", "Tablo başlığı"),
        "zebra_bg" => ("Table stripes", "Tablo şeritleri"),
        "selection" => ("Selection", "Seçim"),
        _ => ("Color", "Renk"),
    };
    pick(lang, en, tr)
}

/// Panel geometry for a client area of `w`×`h`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geo {
    /// The panel card.
    pub panel: Rect,
    /// Header (title, close button; drag handle).
    pub header: Rect,
    /// Close button.
    pub close: Rect,
    /// Search box above the navigation.
    pub search: Rect,
    /// Navigation list (below the search box).
    pub nav: Rect,
    /// Scrolling content area.
    pub content: Rect,
}

/// Geometry of the panel: 720×560 clamped to the window minus 48, centred, offset by `drag`.
pub fn geo(w: f32, h: f32, drag: (f32, f32)) -> Geo {
    let pw = PANEL_W.min(w - 2.0 * MARGIN).max(320.0);
    let ph = PANEL_H.min(h - 2.0 * MARGIN).max(240.0);
    let max_dx = ((w - pw) / 2.0).max(0.0);
    let max_dy = ((h - ph) / 2.0).max(0.0);
    let x = ((w - pw) / 2.0 + drag.0.clamp(-max_dx, max_dx)).round();
    let y = ((h - ph) / 2.0 + drag.1.clamp(-max_dy, max_dy)).round();
    let panel = Rect { x, y, w: pw, h: ph };
    let header = Rect {
        x,
        y,
        w: pw,
        h: HEADER_H,
    };
    let close = Rect {
        x: x + pw - 40.0,
        y: y + 8.0,
        w: 32.0,
        h: 32.0,
    };
    let nav_w = NAV_W.min(pw * 0.4);
    let search = Rect {
        x: x + 8.0,
        y: y + HEADER_H,
        w: nav_w - 16.0,
        h: SEARCH_H,
    };
    let nav = Rect {
        x,
        y: y + HEADER_H + SEARCH_H + 8.0,
        w: nav_w,
        h: ph - HEADER_H - SEARCH_H - 8.0,
    };
    let content = Rect {
        x: x + nav_w,
        y: y + HEADER_H,
        w: pw - nav_w,
        h: ph - HEADER_H,
    };
    Geo {
        panel,
        header,
        close,
        search,
        nav,
        content,
    }
}

/// Snaps a slider value to its step and range.
pub fn snap(v: f64, min: f64, max: f64, step: f64) -> f64 {
    let s = if step > 0.0 {
        min + ((v - min) / step).round() * step
    } else {
        v
    };
    (s.clamp(min, max) * 1000.0).round() / 1000.0
}

/// Text of a slider value for the INI (integers without a fraction).
pub fn slider_text(v: f64, step: f64) -> String {
    if step >= 1.0 {
        format!("{v:.0}")
    } else {
        let t = format!("{v:.2}");
        t.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// Font list filter: names containing the folded query; monospace names first when `mono_first`.
pub fn font_filter(fonts: &[(String, bool)], query: &str, mono_first: bool) -> Vec<usize> {
    let q = fold(query);
    let mut out: Vec<usize> = (0..fonts.len())
        .filter(|&i| fonts.get(i).is_some_and(|(n, _)| fold(n).contains(&q)))
        .collect();
    if mono_first {
        out.sort_by_key(|&i| !fonts.get(i).is_some_and(|f| f.1));
    }
    out
}

/// Range of virtualized list rows visible in a viewport (`first..end`).
pub fn visible_rows(scroll: f32, view_h: f32, row_h: f32, n: usize) -> std::ops::Range<usize> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let first = (scroll.max(0.0) / row_h).floor() as usize;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let count = (view_h / row_h).ceil() as usize + 1;
    first.min(n)..(first + count).min(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_token_has_a_label() {
        for t in yamdv_core::colors::TOKENS {
            assert_ne!(token_label(t, Lang::En), "Color", "{t}");
            assert_ne!(token_label(t, Lang::Tr), "Renk", "{t}");
        }
    }

    #[test]
    fn every_section_has_rows_and_every_row_has_a_known_key() {
        for s in SECTIONS {
            assert!(ROWS.iter().any(|r| r.section == s), "{s:?}");
            assert!(!section_title(s, Lang::Tr).is_empty());
        }
        for r in ROWS {
            assert!(
                r.key.starts_with("btn.")
                    || r.key.starts_with("info.")
                    || settings::def(r.key).is_some(),
                "{}",
                r.key
            );
            assert!(!r.en.is_empty() && !r.tr.is_empty(), "{}", r.key);
            for v in options(r.key) {
                assert!(!option_label(r.key, v, Lang::Tr).is_empty());
            }
        }
        // Rows are grouped by section in navigation order.
        let order: Vec<usize> = ROWS
            .iter()
            .map(|r| SECTIONS.iter().position(|s| *s == r.section).unwrap())
            .collect();
        assert!(order.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn controls_follow_kinds() {
        let row = |k: &str| *ROWS.iter().find(|r| r.key == k).unwrap();
        assert_eq!(control(&row("reading.code_wrap")), Control::Toggle);
        assert_eq!(control(&row("appearance.mode")), Control::Segmented);
        assert_eq!(control(&row("appearance.scheme")), Control::Dropdown);
        assert_eq!(control(&row("sidebar.position")), Control::Dropdown);
        assert_eq!(control(&row("typography.code_font")), Control::Font(true));
        assert_eq!(control(&row("external_editor.custom_exe")), Control::Text);
        assert_eq!(control(&row("btn.clear_history")), Control::Button);
        assert!(matches!(
            control(&row("typography.line_height")),
            Control::Slider { step, .. } if (step - 0.05).abs() < 1e-9
        ));
        assert!(matches!(
            control(&row("editing.autosave_delay_ms")),
            Control::Slider { .. }
        ));
    }

    #[test]
    fn search_filters_and_folds_turkish() {
        let items = static_items(Lang::Tr);
        let hits = filter(&items, "satir numara", Lang::Tr);
        assert!(hits.len() >= 2, "{hits:?}");
        assert!(
            hits.iter()
                .all(|&i| fold(&items[i].label).contains("satir"))
        );
        // Section titles match, keys match, empty query matches everything.
        let gizlilik = filter(&items, "GİZLİLİK", Lang::Tr);
        assert!(
            gizlilik
                .iter()
                .all(|&i| items[i].section == Section::Privacy)
        );
        assert!(!gizlilik.is_empty());
        assert_eq!(filter(&items, "", Lang::Tr).len(), items.len());
        let en = static_items(Lang::En);
        let pdf = filter(&en, "pdf margins", Lang::En);
        assert_eq!(pdf.len(), 1);
        assert_eq!(en[pdf[0]].label, "PDF margins");
        assert!(filter(&en, "zzzz", Lang::En).is_empty());
        assert!(!filter(&en, "export.csv_formula_guard", Lang::En).is_empty());
        assert_eq!(fold("İŞÇĞÖÜı"), "iscgoui");
    }

    #[test]
    fn geometry_clamps_and_drags() {
        let g = geo(1200.0, 800.0, (0.0, 0.0));
        assert_eq!((g.panel.w, g.panel.h), (720.0, 560.0));
        assert_eq!(g.panel.x, 240.0);
        let small = geo(600.0, 400.0, (0.0, 0.0));
        assert_eq!((small.panel.w, small.panel.h), (552.0, 352.0));
        let d = geo(1200.0, 800.0, (5000.0, -5000.0));
        assert_eq!(d.panel.x + d.panel.w, 1200.0);
        assert_eq!(d.panel.y, 0.0);
        assert_eq!(g.content.x, g.panel.x + NAV_W);
        assert_eq!(g.header.h, HEADER_H);
    }

    #[test]
    fn sliders_fonts_and_virtual_rows() {
        assert!((snap(1.637, 1.2, 2.2, 0.05) - 1.65).abs() < 1e-9);
        assert_eq!(snap(99.0, 10.0, 32.0, 1.0), 32.0);
        assert_eq!(slider_text(1.65, 0.05), "1.65");
        assert_eq!(slider_text(1.5, 0.05), "1.5");
        assert_eq!(slider_text(16.0, 1.0), "16");
        let fonts = vec![
            ("Segoe UI".to_string(), false),
            ("Consolas".to_string(), true),
            ("Cascadia Mono".to_string(), true),
            ("Calibri".to_string(), false),
        ];
        assert_eq!(font_filter(&fonts, "", true), vec![1, 2, 0, 3]);
        assert_eq!(font_filter(&fonts, "ca", false), vec![2, 3]);
        assert_eq!(visible_rows(0.0, 100.0, 28.0, 3), 0..3);
        assert_eq!(visible_rows(280.0, 100.0, 28.0, 1000), 10..15);
    }
}
