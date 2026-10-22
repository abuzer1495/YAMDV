// Generated from docs/UI_SPEC.md §B.8.2 (incl. patch P-08b) by a one-off script; keep in sync.
// `shortcuts.<action_id>` overrides are handled separately (Phase 004).

use crate::settings::{Def, Kind};

/// Every `settings.ini` key of §B.8.2 with its type, range and default.
pub const DEFS: &[Def] = &[
    Def {
        key: "general.language",
        kind: Kind::Enum(&["system", "en", "tr"]),
        default: "system",
    },
    Def {
        key: "general.title_format",
        kind: Kind::Enum(&["filename", "fullpath"]),
        default: "filename",
    },
    Def {
        key: "general.open_external_links",
        kind: Kind::Enum(&["ask", "direct", "never"]),
        default: "ask",
    },
    Def {
        key: "general.restore_reading_position",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "general.remember_window_geometry",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "appearance.mode",
        kind: Kind::Enum(&["system", "light", "dark"]),
        default: "system",
    },
    Def {
        key: "appearance.scheme",
        kind: Kind::Enum(&[
            "default",
            "github",
            "glacier",
            "forest",
            "sunset",
            "typewriter",
            "editorial",
            "violet",
            "lychee",
            "mint",
            "neon",
        ]),
        default: "default",
    },
    Def {
        key: "appearance.corner_style",
        kind: Kind::Enum(&["rounded", "sharp"]),
        default: "rounded",
    },
    Def {
        key: "appearance.table_zebra",
        kind: Kind::Bool,
        default: "false",
    },
    Def {
        key: "appearance.use_custom_colors",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "typography.body_font",
        kind: Kind::Str,
        default: "Segoe UI",
    },
    Def {
        key: "typography.code_font",
        kind: Kind::Str,
        default: "Cascadia Mono",
    },
    Def {
        key: "typography.font_size",
        kind: Kind::Int { min: 10, max: 32 },
        default: "16",
    },
    Def {
        key: "typography.line_height",
        kind: Kind::Float { min: 1.2, max: 2.2 },
        default: "1.6",
    },
    Def {
        key: "typography.paragraph_spacing",
        kind: Kind::Int { min: 0, max: 48 },
        default: "16",
    },
    Def {
        key: "typography.column_width_pct",
        kind: Kind::Int { min: 30, max: 100 },
        default: "65",
    },
    Def {
        key: "sidebar.visible",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "sidebar.position",
        kind: Kind::Enum(&["left", "right", "left-floating", "right-floating"]),
        default: "left",
    },
    Def {
        key: "sidebar.width",
        kind: Kind::Int { min: 180, max: 480 },
        default: "260",
    },
    Def {
        key: "sidebar.toc_font_size",
        kind: Kind::Int { min: 11, max: 18 },
        default: "13",
    },
    Def {
        key: "sidebar.toc_max_depth",
        kind: Kind::Int { min: 1, max: 6 },
        default: "4",
    },
    Def {
        key: "sidebar.show_history",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "sidebar.show_favorites",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "sidebar.show_files",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "reading.single_newline_break",
        kind: Kind::Bool,
        default: "false",
    },
    Def {
        key: "reading.smart_punctuation",
        kind: Kind::Bool,
        default: "false",
    },
    Def {
        key: "reading.subscript_tilde",
        kind: Kind::Bool,
        default: "false",
    },
    Def {
        key: "reading.code_line_numbers",
        kind: Kind::Bool,
        default: "false",
    },
    Def {
        key: "reading.code_wrap",
        kind: Kind::Bool,
        default: "false",
    },
    Def {
        key: "reading.front_matter",
        kind: Kind::Enum(&["collapsed", "expanded", "hidden"]),
        default: "collapsed",
    },
    Def {
        key: "reading.render_math",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "reading.render_mermaid",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "reading.render_html",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "reading.emoji_shortcodes",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "reading.animate_gifs",
        kind: Kind::Bool,
        default: "false",
    },
    Def {
        key: "reading.smooth_scrolling",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "reading.scrollbar_autohide",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "reading.word_count_badge",
        kind: Kind::Bool,
        default: "false",
    },
    Def {
        key: "reading.task_toggle_in_read_mode",
        kind: Kind::Bool,
        default: "false",
    },
    Def {
        key: "editing.autosave",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "editing.autosave_delay_ms",
        kind: Kind::Int {
            min: 300,
            max: 5000,
        },
        default: "800",
    },
    Def {
        key: "editing.assets_dir",
        kind: Kind::Str,
        default: "assets",
    },
    Def {
        key: "editing.editor_font",
        kind: Kind::Enum(&["body", "code"]),
        default: "body",
    },
    Def {
        key: "editing.editor_line_numbers",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "editing.editor_soft_wrap",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "editing.editor_syntax_tint",
        kind: Kind::Bool,
        default: "false",
    },
    Def {
        key: "editing.split_ratio",
        kind: Kind::Float { min: 0.2, max: 0.8 },
        default: "0.5",
    },
    Def {
        key: "editing.split_swapped",
        kind: Kind::Bool,
        default: "false",
    },
    Def {
        key: "editing.format_toolbar",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "editing.auto_format_tables",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "editing.snippets",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "editing.auto_pair",
        kind: Kind::Bool,
        default: "false",
    },
    Def {
        key: "editing.list_indent",
        kind: Kind::Enum(&["2", "4", "tab"]),
        default: "2",
    },
    Def {
        key: "shortcuts.preset",
        kind: Kind::Enum(&["default", "typora"]),
        default: "default",
    },
    Def {
        key: "external_editor.mode",
        kind: Kind::Enum(&["auto", "custom"]),
        default: "auto",
    },
    Def {
        key: "external_editor.custom_exe",
        kind: Kind::Str,
        default: "",
    },
    Def {
        key: "external_editor.custom_args",
        kind: Kind::Str,
        default: "{file}",
    },
    Def {
        key: "export.pdf_colors",
        kind: Kind::Enum(&["light", "theme"]),
        default: "light",
    },
    Def {
        key: "export.pdf_page_size",
        kind: Kind::Enum(&["auto", "a4", "letter"]),
        default: "auto",
    },
    Def {
        key: "export.pdf_margins_mm",
        kind: Kind::Int { min: 5, max: 40 },
        default: "18",
    },
    Def {
        key: "export.pdf_page_numbers",
        kind: Kind::Bool,
        default: "false",
    },
    Def {
        key: "export.print_colors",
        kind: Kind::Enum(&["light", "theme"]),
        default: "light",
    },
    Def {
        key: "export.html_colors",
        kind: Kind::Enum(&["light", "theme"]),
        default: "light",
    },
    Def {
        key: "export.html_include_toc",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "export.csv_formula_guard",
        kind: Kind::Bool,
        default: "true",
    },
    Def {
        key: "integration.context_menu",
        kind: Kind::Bool,
        default: "false",
    },
    Def {
        key: "privacy.diagnostic_log",
        kind: Kind::Bool,
        default: "false",
    },
];
