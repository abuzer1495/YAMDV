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
    DiagramUnsupported => "Diagram type not supported yet", "Bu diyagram türü henüz desteklenmiyor";
    DiagramTooLarge => "Diagram too large ({} nodes)", "Diyagram çok büyük ({} düğüm)";
    ContentTooComplex => "Content too complex", "İçerik çok karmaşık";
    MermaidError => "Mermaid syntax error at line {l}, column {c}: {m}", "Mermaid söz dizimi hatası, satır {l}, sütun {c}: {m}";
    MathErrorMsg => "Math error: {}", "Matematik hatası: {}";
    Cancel => "Cancel", "İptal";
    BinaryTitle => "Binary file", "İkili dosya";
    BinaryBody => "This looks like a binary file.", "Bu bir ikili dosya gibi görünüyor.";
    OpenAnyway => "Open anyway", "Yine de aç";
    DropFolders => "Drop Markdown files, not folders", "Klasör değil, Markdown dosyaları bırakın";
    Copied => "Copied", "Kopyalandı";
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
    LinkNotFound => "Link target not found: ", "Bağlantı hedefi bulunamadı: ";
    BlockedLinkType => "Blocked link type", "Engellenen bağlantı türü";
    LinkAskTitle => "Open external link?", "Dış bağlantı açılsın mı?";
    LinkCredentials => "Warning: this link contains a user name or password.", "Uyarı: bu bağlantı bir kullanıcı adı veya parola içeriyor.";
    LinkPunycode => "Punycode: ", "Punycode: ";
    CopyLink => "Copy link", "Bağlantıyı kopyala";
    OpenInBrowser => "Open in browser", "Tarayıcıda aç";
    OpenLocalTitle => "Open with default app?", "Varsayılan uygulamayla açılsın mı?";
    ShowFolderTitle => "Show in Explorer?", "Gezgin'de gösterilsin mi?";
    BlockedSafety => "Blocked for safety", "Güvenlik nedeniyle engellendi";
    ShowInExplorer => "Show in Explorer", "Gezgin'de göster";
    OpenButton => "Open", "Aç";
    FromInternet => "This file came from the internet", "Bu dosya internetten geldi";
    FrontMatterOne => "Front matter · 1 field", "Ön bilgi · 1 alan";
    FrontMatterN => "Front matter · {} fields", "Ön bilgi · {} alan";
    LabelDetails => "details", "ayrıntılar";
    Image => "image", "görsel";
    MCopy => "Copy", "Kopyala";
    MCopyMd => "Copy as Markdown", "Markdown olarak kopyala";
    MCopyLink => "Copy link address", "Bağlantı adresini kopyala";
    MSelectAll => "Select all", "Tümünü seç";
    MAppearance => "Appearance", "Görünüm";
    MThemeMode => "Theme mode", "Tema modu";
    MSystem => "System", "Sistem";
    MLight => "Light", "Açık";
    MDark => "Dark", "Koyu";
    MScheme => "Color scheme", "Renk şeması";
    MBodyFont => "Body font…", "Metin yazı tipi…";
    MFontSize => "Font size", "Yazı boyutu";
    MBigger => "Bigger", "Büyüt";
    MSmaller => "Smaller", "Küçült";
    MReset => "Reset", "Sıfırla";
    MColumn => "Column width", "Sütun genişliği";
    MWider => "Wider", "Genişlet";
    MNarrower => "Narrower", "Daralt";
    MShowOutline => "Show outline", "Ana hattı göster";
    MOutlinePos => "Outline position", "Ana hat konumu";
    MLeft => "Left", "Sol";
    MRight => "Right", "Sağ";
    MSingleNl => "Single newline as line break", "Tek satır sonu = satır kesmesi";
    MCodeNums => "Code line numbers", "Kod satır numaraları";
    MCodeWrap => "Code wrap", "Kodu kaydır";
    MExtEditor => "Open in external editor", "Harici düzenleyicide aç";
    MFavAdd => "Add to favorites", "Sık kullanılanlara ekle";
    MFavRemove => "Remove from favorites", "Sık kullanılanlardan çıkar";
    MFile => "File", "Dosya";
    MOpen => "Open…", "Aç…";
    MReload => "Reload", "Yeniden yükle";
    MShowExplorer => "Show in Explorer", "Gezginde göster";
    MCopyPath => "Copy file path", "Dosya yolunu kopyala";
    MSettings => "Settings", "Ayarlar";
    MSettingsOpen => "Settings…", "Ayarlar…";
    PanelSearch => "Search settings", "Ayarlarda ara";
    PanelResetSection => "Reset section", "Bölümü sıfırla";
    PanelResetAll => "Reset all…", "Tümünü sıfırla…";
    PanelNoMatch => "No settings match", "Eşleşen ayar yok";
    PanelResetAllTitle => "Reset all settings?", "Tüm ayarlar sıfırlansın mı?";
    PanelResetAllBody => "Every setting returns to its default. Shortcuts and custom colors are kept.", "Her ayar varsayılanına döner. Kısayollar ve özel renkler korunur.";
    PanelResetColorsTitle => "Reset all colors?", "Tüm renkler sıfırlansın mı?";
    PanelResetColorsBody => "Every custom color is removed from custom-theme.ini.", "Tüm özel renkler custom-theme.ini dosyasından kaldırılır.";
    PanelClearTitle => "Clear this data?", "Bu veriler temizlensin mi?";
    PanelClearBody => "This cannot be undone.", "Bu işlem geri alınamaz.";
    PanelReset => "Reset", "Sıfırla";
    PanelClear => "Clear", "Temizle";
    PanelDone => "Done", "Tamam";
    PanelPressKeys => "Press a key combination…", "Bir tuş bileşimine basın…";
    PanelUnbound => "Not set", "Atanmamış";
    PanelConflict => "Already used by: ", "Şunda kullanılıyor: ";
    PanelCaptureHint => "Esc cancels · Backspace clears", "Esc iptal · Geri tuşu temizler";
    PanelSearchFonts => "Search fonts", "Yazı tiplerinde ara";
    PanelFontMissing => "not installed, using ", "yüklü değil, kullanılan: ";
    PanelCleared => "Cleared", "Temizlendi";
    PanelLight => "Light", "Açık";
    PanelDark => "Dark", "Koyu";
    MLanguage => "Language", "Dil";
    MExtLinks => "Open external links", "Dış bağlantıları aç";
    MAsk => "Ask", "Sor";
    MDirect => "Direct", "Doğrudan";
    MNever => "Never", "Asla";
    MRestorePos => "Restore reading position", "Okuma konumunu hatırla";
    MSetDefault => "Set YAMDV as default app…", "YAMDV'yi varsayılan uygulama yap…";
    MCtxMenu => "\"Open with YAMDV\" in Explorer", "Gezginde \"YAMDV ile aç\"";
    MEditSettings => "Edit settings file", "Ayar dosyasını düzenle";
    MConfigFolder => "Open config folder", "Yapılandırma klasörünü aç";
    MShortcuts => "Keyboard shortcuts", "Klavye kısayolları";
    MAbout => "About YAMDV", "YAMDV hakkında";
    AboutTitle => "YAMDV", "YAMDV";
    Ok => "OK", "Tamam";
    ToastFontSize => "Font size: ", "Yazı boyutu: ";
    ToastColumn => "Column width: ", "Sütun genişliği: ";
    ToastFavAdded => "Added to favorites", "Sık kullanılanlara eklendi";
    ToastFavRemoved => "Removed from favorites", "Sık kullanılanlardan çıkarıldı";
    ToastPathCopied => "File path copied", "Dosya yolu kopyalandı";
    ToastLinkCopied => "Link address copied", "Bağlantı adresi kopyalandı";
    ToastNoEditor => "No external editor found", "Harici düzenleyici bulunamadı";
    ToastDeleted => "The file was deleted or moved", "Dosya silindi veya taşındı";
    ToastSaveFailed => "Settings could not be saved", "Ayarlar kaydedilemedi";
    ToastRegistered => "YAMDV is registered for Markdown files", "YAMDV Markdown dosyaları için kaydedildi";
    ToastDefaultApps => "Choose YAMDV for .md in Windows Settings", "Windows Ayarları'nda .md için YAMDV'yi seçin";
    ToastCtxOn => "Explorer entry added", "Gezgin girdisi eklendi";
    ToastCtxOff => "Explorer entry removed", "Gezgin girdisi kaldırıldı";
    ToastThemeInvalid => "custom-theme.ini has invalid values", "custom-theme.ini geçersiz değerler içeriyor";
    ToastMissing => "File not found: ", "Dosya bulunamadı: ";
    FindPlaceholder => "Find", "Bul";
    FindNone => "No results", "Sonuç yok";
    FindCase => "Aa", "Aa";
    FindWhole => "W", "K";
    FindRegex => ".*", ".*";
    ReplacePlaceholder => "Replace with", "Şununla değiştir";
    ReplaceOne => "Replace", "Değiştir";
    ReplaceAll => "Replace all", "Tümünü değiştir";
    FindBadRegex => "Invalid pattern", "Geçersiz desen";
    FindTooSlow => "Pattern too slow (stopped after 100 ms)", "Desen çok yavaş (100 ms sonra durduruldu)";
    Replaced => "Replacements:", "Değiştirilen:";
    TbBold => "Bold (Ctrl+B)", "Kalın (Ctrl+B)";
    TbItalic => "Italic (Ctrl+I)", "İtalik (Ctrl+I)";
    TbStrike => "Strikethrough (Ctrl+Shift+X)", "Üstü çizili (Ctrl+Shift+X)";
    TbHighlight => "Highlight (Ctrl+Shift+H)", "Vurgula (Ctrl+Shift+H)";
    TbCode => "Inline code (Ctrl+`)", "Satır içi kod (Ctrl+`)";
    TbLink => "Link (Ctrl+K)", "Bağlantı (Ctrl+K)";
    TbH1 => "Heading 1 (Ctrl+1)", "Başlık 1 (Ctrl+1)";
    TbH2 => "Heading 2 (Ctrl+2)", "Başlık 2 (Ctrl+2)";
    TbH3 => "Heading 3 (Ctrl+3)", "Başlık 3 (Ctrl+3)";
    TbQuote => "Quote (Ctrl+Shift+Q)", "Alıntı (Ctrl+Shift+Q)";
    TbBullet => "Bulleted list (Ctrl+Shift+8)", "Madde listesi (Ctrl+Shift+8)";
    TbNumbered => "Numbered list (Ctrl+Shift+7)", "Numaralı liste (Ctrl+Shift+7)";
    TbTask => "Task list (Ctrl+Shift+9)", "Görev listesi (Ctrl+Shift+9)";
    TbClear => "Clear formatting", "Biçimlendirmeyi temizle";
    RecoverBanner => "Unsaved changes from an earlier session:", "Önceki oturumdan kaydedilmemiş değişiklikler:";
    RecoverRestore => "Restore", "Geri yükle";
    RecoverDiscard => "Discard", "Vazgeç";
    JournalTooLarge => "Over 32 MB: unsaved changes are not journaled for recovery.", "32 MB üstü: kaydedilmemiş değişiklikler kurtarma için günlüğe yazılmıyor.";
    AssetFailed => "The file could not be copied next to the document.", "Dosya belgenin yanına kopyalanamadı.";
    SaveFirst => "Save the document first to paste images or drop files.", "Görüntü yapıştırmak veya dosya bırakmak için önce belgeyi kaydedin.";
    DropLargeTitle => "Large file", "Büyük dosya";
    DropLargeBody => "This file is larger than 50 MB. Copy it into the document's assets folder?", "Bu dosya 50 MB'tan büyük. Belgenin varlık klasörüne kopyalansın mı?";
    DropCopy => "Copy", "Kopyala";
    TabContents => "Contents", "İçindekiler";
    TabHistory => "History", "Geçmiş";
    TabFavorites => "Favorites", "Sık kullanılanlar";
    GroupSession => "Current session", "Bu oturum";
    GroupToday => "Today", "Bugün";
    GroupYesterday => "Yesterday", "Dün";
    Clear => "Clear", "Temizle";
    CurrentTag => "Current", "Geçerli";
    FavEmpty => "Press Ctrl+D to favorite this file", "Bu dosyayı eklemek için Ctrl+D'ye basın";
    TocEmpty => "No headings", "Başlık yok";
    WelcomeTitle => "Welcome to YAMDV", "YAMDV'ye hoş geldiniz";
    WelcomeOpen => "Open file…", "Dosya aç…";
    WelcomeRecent => "Recent files", "Son dosyalar";
    WelcomeIsDefault => "YAMDV is the default app for .md files", "YAMDV .md dosyaları için varsayılan uygulama";
    WelcomeNotDefault => "YAMDV is not the default app for .md files", "YAMDV .md dosyaları için varsayılan uygulama değil";
    WelcomeSetDefault => "Set as default", "Varsayılan yap";
    AboutVersion => "Version ", "Sürüm ";
    AboutExe => "Program: ", "Program: ";
    AboutConfig => "Settings: ", "Ayarlar: ";
    MEdit => "Edit", "Düzenle";
    MLiveEdit => "Live Edit", "Canlı Düzenleme";
    MSplitEdit => "Split Edit", "Bölünmüş Düzenleme";
    MSwapPanes => "Swap panes", "Bölmelerin yerini değiştir";
    MUndo => "Undo", "Geri al";
    MRedo => "Redo", "Yinele";
    MCut => "Cut", "Kes";
    MPaste => "Paste", "Yapıştır";
    MFormat => "Format", "Biçim";
    MBold => "Bold", "Kalın";
    MItalic => "Italic", "İtalik";
    MStrike => "Strikethrough", "Üstü çizili";
    MInlineCode => "Inline code", "Satır içi kod";
    MLink => "Link", "Bağlantı";
    MHeading1 => "Heading 1", "Başlık 1";
    MHeading2 => "Heading 2", "Başlık 2";
    MHeading3 => "Heading 3", "Başlık 3";
    MParagraph => "Paragraph", "Paragraf";
    MQuote => "Quote", "Alıntı";
    MBulletList => "Bullet list", "Madde işaretli liste";
    MNumberedList => "Numbered list", "Numaralı liste";
    MTaskList => "Task list", "Görev listesi";
    MAutosave => "Autosave", "Otomatik kaydet";
    MEditorFont => "Editor font", "Düzenleyici yazı tipi";
    MFontBody => "Body", "Metin";
    MFontCode => "Code", "Kod";
    MEditorLineNumbers => "Editor line numbers", "Düzenleyici satır numaraları";
    MEditorSoftWrap => "Editor soft wrap", "Düzenleyicide satır kaydırma";
    EditNotAvailable => "Editing is not available", "Düzenleme kullanılamıyor";
    EditReadOnly => "This file is read-only. You can save a copy and edit that.", "Bu dosya salt okunur. Bir kopyasını kaydedip onu düzenleyebilirsiniz.";
    EditLossy => "Some bytes of this file could not be decoded; saving would change them.", "Bu dosyanın bazı baytları çözülemedi; kaydetmek onları değiştirir.";
    EditPlain => "The file is shown as plain text and cannot be edited here.", "Dosya düz metin olarak gösteriliyor ve burada düzenlenemez.";
    EditVirtual => "Built-in documents cannot be edited.", "Yerleşik belgeler düzenlenemez.";
    SaveCopyAs => "Save a copy as…", "Kopyasını farklı kaydet…";
    ConvertEdit => "Convert to UTF-8 and edit", "UTF-8'e dönüştür ve düzenle";
    ConflictTitle => "The file changed on disk", "Dosya diskte değişti";
    ConflictBody => "Another program changed this file. Your edits are not saved yet.", "Başka bir program bu dosyayı değiştirdi. Düzenlemeleriniz henüz kaydedilmedi.";
    Overwrite => "Overwrite", "Üzerine yaz";
    ReloadDiscard => "Reload (discard mine)", "Yeniden yükle (benimkileri at)";
    SaveMineAs => "Save mine as…", "Benimkini farklı kaydet…";
    UnmappableTitle => "Cannot save in this encoding", "Bu kodlamayla kaydedilemiyor";
    UnmappableBody => "Some characters do not exist in the file's encoding.", "Bazı karakterler dosyanın kodlamasında yok.";
    ConvertSave => "Convert to UTF-8 and save", "UTF-8'e dönüştür ve kaydet";
    UnsavedTitle => "Unsaved changes", "Kaydedilmemiş değişiklikler";
    UnsavedBody => "Save your changes before closing?", "Kapatmadan önce değişiklikler kaydedilsin mi?";
    Save => "Save", "Kaydet";
    Discard => "Don't save", "Kaydetme";
    SaveFailed => "The file could not be saved", "Dosya kaydedilemedi";
    EditHelp => "Help (F1)", "Yardım (F1)";
    MExport => "Export", "Dışa aktar";
    MExportHtml => "Export HTML…", "HTML olarak dışa aktar…";
    MExportPdf => "Export PDF…", "PDF olarak dışa aktar…";
    MPrint => "Print…", "Yazdır…";
    MCopyTableCsv => "Copy table as CSV", "Tabloyu CSV olarak kopyala";
    MExportTableCsv => "Export table as CSV…", "Tabloyu CSV olarak dışa aktar…";
    PdfPrinterMissingTitle => "Export PDF", "PDF olarak dışa aktar";
    PdfPrinterMissing => "Microsoft Print to PDF is not installed — use Print… and choose a PDF printer.", "Microsoft Print to PDF yüklü değil — Yazdır… seçeneğini kullanıp bir PDF yazıcısı seçin.";
    Exported => "Exported", "Dışa aktarıldı";
    ExportFailed => "Export failed", "Dışa aktarma başarısız";
    PrintSent => "Sent to the printer", "Yazıcıya gönderildi";
    PrintFailed => "Printing failed", "Yazdırma başarısız";
    ExportBusy => "An export is already running", "Bir dışa aktarma zaten sürüyor";
    ExportCancelled => "Export cancelled", "Dışa aktarma iptal edildi";
    PrintCancelled => "Printing cancelled", "Yazdırma iptal edildi";
    JobCloseTitle => "An export is running", "Bir dışa aktarma sürüyor";
    JobCloseBody => "Closing this window cancels the export and removes the unfinished file.", "Bu pencereyi kapatmak dışa aktarmayı iptal eder ve bitmemiş dosyayı siler.";
    JobCloseCancel => "Cancel export and close", "Dışa aktarmayı iptal et ve kapat";
    JobCloseKeep => "Keep exporting", "Dışa aktarmaya devam et";
    Open => "Open", "Aç";
    PdfSaved => "PDF saved", "PDF kaydedildi";
    HtmlSaved => "HTML saved", "HTML kaydedildi";
    ShowInFolder => "Show in folder", "Klasörde göster";
    WelcomeInstall => "Install YAMDV for this user", "YAMDV'yi bu kullanıcı için yükle";
    InstallFailed => "Installation failed", "Yükleme başarısız";
    UninstallTitle => "Uninstall YAMDV", "YAMDV'yi kaldır";
    UninstallBody => "YAMDV, its registration and its Start-menu shortcut will be removed. Keep your settings, favorites and recent files?", "YAMDV, kaydı ve Başlat menüsü kısayolu kaldırılacak. Ayarlarınız, sık kullanılanlarınız ve son dosyalarınız saklansın mı?";
    UninstallKeep => "Uninstall, keep settings", "Kaldır, ayarları sakla";
    UninstallRemove => "Uninstall, remove settings", "Kaldır, ayarları sil";
    MSchemeCards => "Color scheme…", "Renk şeması…";
    MWordCount => "Word count", "Sözcük sayısı";
    MFullscreen => "Full screen", "Tam ekran";
    PillReadOnly => "Read-only", "Salt okunur";
    WordBadge => "{w} words · {c} characters", "{w} sözcük · {c} karakter";
    ZoomFont => "{} px", "{} px";
    ZoomColumn => "Column {} %", "Sütun %{}";
    BannerMotw => "This file came from the internet — links always ask", "Bu dosya internetten geldi — bağlantılar her zaman sorulur";
    BannerElevated => "Running as administrator is not recommended", "Yönetici olarak çalıştırmak önerilmez";
    BannerReadOnly => "This file is read-only", "Bu dosya salt okunur";
    BannerDecoding => "Not valid UTF-8 — shown as {}", "Geçerli UTF-8 değil — {} olarak gösteriliyor";
    BannerExportFailed => "Export failed — see the log for the reason", "Dışa aktarma başarısız — nedeni için günlüğe bakın";
    Dismiss => "Dismiss", "Kapat";
    TipMinimize => "Minimize", "Simge durumuna küçült";
    TipMaximize => "Maximize", "Ekranı kapla";
    TipRestore => "Restore down", "Önceki boyutuna getir";
    TipClose => "Close", "Kapat";
    TipMore => "More (⋯)", "Diğer (⋯)";
    TipWords => "{w} words · {c} characters · about {m} min read", "{w} sözcük · {c} karakter · yaklaşık {m} dk okuma";
    TipFold => "Fold or unfold · Alt+click: every heading of this level", "Katla veya aç · Alt+tık: bu düzeydeki tüm başlıklar";
    TipViewer => "Open full screen (double-click)", "Tam ekranda aç (çift tık)";
    ViewZoomOut => "Zoom out (−)", "Uzaklaştır (−)";
    ViewZoomIn => "Zoom in (+)", "Yakınlaştır (+)";
    ViewZoom => "Zoom level — click for fit / 100 %", "Yakınlaştırma — sığdır / %100 için tıklayın";
    ViewFit => "Fit to window (0)", "Pencereye sığdır (0)";
    ViewActual => "Actual size (1:1)", "Gerçek boyut (1:1)";
    ViewCopy => "Copy", "Kopyala";
    ViewClose => "Close (Esc)", "Kapat (Esc)";
    ViewTable => "Table · {r} × {c}", "Tablo · {r} × {c}";
    ExportCsv => "Export CSV", "CSV dışa aktar";
    CopiedImage => "Image copied", "Görüntü kopyalandı";
    CopiedCsv => "Table copied as CSV", "Tablo CSV olarak kopyalandı";
    TabFiles => "Files", "Dosyalar";
    FilesEmpty => "No Markdown files in this folder", "Bu klasörde Markdown dosyası yok";
    FilesMore => "…more", "…daha fazla";
    FilesNoFolder => "Open a file or drop a folder", "Bir dosya açın veya klasör bırakın";
    FilesNetwork => "Network folders are not listed", "Ağ klasörleri listelenmez";
    MFloating => "Floating", "Yüzen";
    MGoToHeading => "Go to heading…", "Başlığa git…";
    MMindMap => "View as mind map", "Zihin haritası olarak görüntüle";
    MInsert => "Insert", "Ekle";
    MInsTable => "Table…", "Tablo…";
    MInsCode => "Code block", "Kod bloğu";
    MInsCallout => "Callout", "Bilgi kutusu";
    MCalloutNote => "Note", "Not";
    MCalloutTip => "Tip", "İpucu";
    MCalloutImportant => "Important", "Önemli";
    MCalloutWarning => "Warning", "Uyarı";
    MCalloutCaution => "Caution", "Dikkat";
    MInsMath => "Math block", "Matematik bloğu";
    MInsMermaid => "Mermaid diagram", "Mermaid diyagramı";
    MInsRule => "Horizontal rule", "Yatay çizgi";
    MInsImage => "Image…", "Görsel…";
    MInsLink => "Link…", "Bağlantı…";
    MInsFootnote => "Footnote", "Dipnot";
    PaletteHint => "Type to filter headings", "Başlıkları süzmek için yazın";
    PaletteNone => "No matching heading", "Eşleşen başlık yok";
    TocTitle => "Contents", "İçindekiler";
    MindRight => "Right", "Sağ";
    MindBalanced => "Balanced", "Dengeli";
    MindCopied => "Mind map copied", "Zihin haritası kopyalandı";
    FilesPick => "Pick a file in the Files tab", "Dosyalar sekmesinden bir dosya seçin";
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
