// Generated from docs/UI_SPEC.md §B.3.1–§B.3.4 by a one-off script; keep in sync.
use crate::theme::{Callout, DARK_BASE, GITHUB_DARK, LIGHT_SHARED, Palette, Syntax};

/// Light palettes in R-THEME-02 order.
pub const LIGHT: [Palette; 11] = [
    Palette {
        bg: 0xffffff,
        text: 0x333333,
        heading: 0x333333,
        link: 0x2563eb,
        accent: 0x2563eb,
        quote_bar: 0xdddddd,
        quote_text: 0x666666,
        code_bg: 0xf5f5f5,
        inline_code_bg: 0xf0f0f0,
        table_header_bg: 0xf5f5f5,
        ..LIGHT_SHARED
    }, // Default
    Palette {
        bg: 0xffffff,
        text: 0x1f2328,
        heading: 0x1f2328,
        link: 0x0969da,
        accent: 0x0969da,
        quote_bar: 0xd1d9e0,
        quote_text: 0x59636e,
        code_bg: 0xf6f8fa,
        inline_code_bg: 0xeff1f3,
        table_header_bg: 0xf6f8fa,
        ..LIGHT_SHARED
    }, // GitHub
    Palette {
        bg: 0xf6f9fe,
        text: 0x1a2538,
        heading: 0x2563eb,
        link: 0x2563eb,
        accent: 0x2563eb,
        quote_bar: 0x2563eb,
        quote_text: 0x6e7684,
        code_bg: 0xe8f0fe,
        inline_code_bg: 0xe6edfc,
        table_header_bg: 0xe8f0fe,
        ..LIGHT_SHARED
    }, // Glacier
    Palette {
        bg: 0xf5faf5,
        text: 0x1a2e1a,
        heading: 0x008a2e,
        link: 0x00822b,
        accent: 0x008a2e,
        quote_bar: 0x008a2e,
        quote_text: 0x6d7c6d,
        code_bg: 0xe8f8e8,
        inline_code_bg: 0xe2f1e6,
        table_header_bg: 0xe8f8e8,
        ..LIGHT_SHARED
    }, // Forest
    Palette {
        bg: 0xfef8f0,
        text: 0x2c1a0a,
        heading: 0xb45309,
        link: 0xb45309,
        accent: 0xb45309,
        quote_bar: 0xb45309,
        quote_text: 0x7c6e61,
        code_bg: 0xfaf0e0,
        inline_code_bg: 0xf8ecde,
        table_header_bg: 0xfaf0e0,
        ..LIGHT_SHARED
    }, // Sunset
    Palette {
        bg: 0xf5f0e8,
        text: 0x2c2416,
        heading: 0x8b6914,
        link: 0x896714,
        accent: 0x8b6914,
        quote_bar: 0x8b6914,
        quote_text: 0x787166,
        code_bg: 0xede5d8,
        inline_code_bg: 0xede5d8,
        table_header_bg: 0xede5d8,
        ..LIGHT_SHARED
    }, // Typewriter
    Palette {
        bg: 0xfaf8f5,
        text: 0x2c2416,
        heading: 0x8b4513,
        link: 0x8b4513,
        accent: 0x8b4513,
        quote_bar: 0x8b4513,
        quote_text: 0x7a746b,
        code_bg: 0xf5f0e8,
        inline_code_bg: 0xf1eae3,
        table_header_bg: 0xf5f0e8,
        ..LIGHT_SHARED
    }, // Editorial
    Palette {
        bg: 0xfaf5fe,
        text: 0x271b30,
        heading: 0x7209b7,
        link: 0x7209b7,
        accent: 0x7209b7,
        quote_bar: 0x7209b7,
        quote_text: 0x776e7f,
        code_bg: 0xf4eafb,
        inline_code_bg: 0xefe3f8,
        table_header_bg: 0xf4eafb,
        ..LIGHT_SHARED
    }, // Violet
    Palette {
        bg: 0xfff5f5,
        text: 0x2b2726,
        heading: 0xe63946,
        link: 0xdb1c2a,
        accent: 0xe63946,
        quote_bar: 0xe63946,
        quote_text: 0x7c7575,
        code_bg: 0xffeeee,
        inline_code_bg: 0xfde6e7,
        table_header_bg: 0xffeeee,
        ..LIGHT_SHARED
    }, // Lychee
    Palette {
        bg: 0xf5fbf8,
        text: 0x1a3a2a,
        heading: 0x0ea271,
        link: 0x0b815a,
        accent: 0x10b981,
        quote_bar: 0x10b981,
        quote_text: 0x6d8378,
        code_bg: 0xe8f8f0,
        inline_code_bg: 0xe3f6ef,
        table_header_bg: 0xe8f8f0,
        ..LIGHT_SHARED
    }, // Mint
    Palette {
        bg: 0xf0f4f8,
        text: 0x1a1a2e,
        heading: 0x0077b6,
        link: 0x0072ae,
        accent: 0x0077b6,
        quote_bar: 0x0077b6,
        quote_text: 0x6b6d7b,
        code_bg: 0xe2e8f0,
        inline_code_bg: 0xddeaf3,
        table_header_bg: 0xe2e8f0,
        ..LIGHT_SHARED
    }, // Neon
];

/// Dark palettes in the same order.
pub const DARK: [Palette; 11] = [
    Palette {
        heading: 0xd4d4d4,
        link: 0x4da5ec,
        accent: 0x4da5ec,
        quote_bar: 0x555555,
        code_bg: 0x2d2d2d,
        table_header_bg: 0x2d2d2d,
        ..DARK_BASE
    }, // Default
    GITHUB_DARK, // GitHub
    Palette {
        heading: 0x608df0,
        link: 0x608df0,
        accent: 0x608df0,
        quote_bar: 0x4966a6,
        code_bg: 0x31353d,
        table_header_bg: 0x323740,
        ..DARK_BASE
    }, // Glacier
    Palette {
        heading: 0x5ff290,
        link: 0x5ff290,
        accent: 0x5ff290,
        quote_bar: 0x48a868,
        code_bg: 0x313d35,
        table_header_bg: 0x324137,
        ..DARK_BASE
    }, // Forest
    Palette {
        heading: 0xf29e5f,
        link: 0xf29e5f,
        accent: 0xf29e5f,
        quote_bar: 0xa87148,
        code_bg: 0x3d3631,
        table_header_bg: 0x413832,
        ..DARK_BASE
    }, // Sunset
    Palette {
        heading: 0xe9c467,
        link: 0xe9c467,
        accent: 0xe9c467,
        quote_bar: 0xa28a4d,
        code_bg: 0x3c3932,
        table_header_bg: 0x403c33,
        ..DARK_BASE
    }, // Typewriter
    Palette {
        heading: 0xea9d66,
        link: 0xea9d66,
        accent: 0xea9d66,
        quote_bar: 0xa3714d,
        code_bg: 0x3c3632,
        table_header_bg: 0x403833,
        ..DARK_BASE
    }, // Editorial
    Palette {
        heading: 0xb85ff2,
        link: 0xb85ff2,
        accent: 0xb85ff2,
        quote_bar: 0x8248a8,
        code_bg: 0x38313d,
        table_header_bg: 0x3b3241,
        ..DARK_BASE
    }, // Violet
    Palette {
        heading: 0xec656f,
        link: 0xec656f,
        accent: 0xec656f,
        quote_bar: 0xa44c53,
        code_bg: 0x3c3132,
        table_header_bg: 0x403334,
        ..DARK_BASE
    }, // Lychee
    Palette {
        heading: 0x5ff1c1,
        link: 0x5ff1c1,
        accent: 0x5ff1c1,
        quote_bar: 0x48a788,
        code_bg: 0x313d39,
        table_header_bg: 0x32413c,
        ..DARK_BASE
    }, // Mint
    Palette {
        heading: 0x5fbff2,
        link: 0x5fbff2,
        accent: 0x5fbff2,
        quote_bar: 0x4887a8,
        code_bg: 0x31393d,
        table_header_bg: 0x323c41,
        ..DARK_BASE
    }, // Neon
];

/// Callout colors (B.3.3).
pub const CALLOUTS: [Callout; 5] = [
    Callout {
        name: "note",
        light: 0x0969da,
        light_bg: 0xddf4ff,
        dark: 0x4493f8,
        dark_bg: 0x232c38,
    },
    Callout {
        name: "tip",
        light: 0x1a7f37,
        light_bg: 0xe8faf0,
        dark: 0x3fb950,
        dark_bg: 0x202d22,
    },
    Callout {
        name: "important",
        light: 0x8250df,
        light_bg: 0xf6f0ff,
        dark: 0xab7df8,
        dark_bg: 0x2f2938,
    },
    Callout {
        name: "warning",
        light: 0x9a6700,
        light_bg: 0xfffbe8,
        dark: 0xd29922,
        dark_bg: 0x342d1e,
    },
    Callout {
        name: "caution",
        light: 0xcf222e,
        light_bg: 0xffebe9,
        dark: 0xf85149,
        dark_bg: 0x382423,
    },
];

/// Syntax palette (B.3.4, single-color classes).
pub const SYNTAX: [Syntax; 10] = [
    Syntax {
        class: "default text",
        light: 0x24292e,
        dark: 0xc9d1d9,
    },
    Syntax {
        class: "comment",
        light: 0x6a737d,
        dark: 0x8b949e,
    },
    Syntax {
        class: "keyword, meta/preprocessor",
        light: 0xd73a49,
        dark: 0xff7b72,
    },
    Syntax {
        class: "string, regex",
        light: 0x032f62,
        dark: 0xa5d6ff,
    },
    Syntax {
        class: "number, constant, builtin, attribute",
        light: 0x005cc5,
        dark: 0x79c0ff,
    },
    Syntax {
        class: "function, class/title",
        light: 0x6f42c1,
        dark: 0xd2a8ff,
    },
    Syntax {
        class: "type, variable (special)",
        light: 0xe36209,
        dark: 0xffa657,
    },
    Syntax {
        class: "tag",
        light: 0x22863a,
        dark: 0x7ee787,
    },
    Syntax {
        class: "operator, punctuation",
        light: 0x24292e,
        dark: 0xc9d1d9,
    },
    Syntax {
        class: "markdown heading in code",
        light: 0x005cc5,
        dark: 0x79c0ff,
    },
];
