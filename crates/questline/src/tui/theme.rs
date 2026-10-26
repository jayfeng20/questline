//! Colour themes for the dashboard, picked with `questline --theme`.

use ratatui::style::Color;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum Theme {
    #[default]
    Default,
    GruvboxDark,
    CatppuccinFrappe,
}

impl Theme {
    pub fn palette(self) -> &'static Palette {
        match self {
            Self::Default => &DEFAULT,
            Self::GruvboxDark => &GRUVBOX_DARK,
            Self::CatppuccinFrappe => &CATPPUCCIN_FRAPPE,
        }
    }
}

/// Every colour the dashboard draws with, by role.
pub struct Palette {
    /// `Color::Reset` keeps the terminal's own.
    pub bg: Color,
    pub fg: Color,
    /// Header labels, the logo, the `:` prompt, the current crumb, the module being checked.
    pub accent: Color,
    /// Borders, key hints, the other crumbs.
    pub frame: Color,
    /// Text on the crumbs.
    pub on_frame: Color,
    pub title: Color,
    /// The `(scope)` after a table title.
    pub scope: Color,
    /// Column headers and the row count.
    pub heading: Color,
    /// Key hint descriptions, not-started rows.
    pub muted: Color,
    /// The sort tag, hints, pending rows.
    pub faint: Color,
    /// The `/` filter, info messages.
    pub info: Color,
    pub completed: Color,
    pub in_progress: Color,
    pub stale: Color,
    pub failed: Color,
    pub queued: Color,
}

pub const DEFAULT: Palette = Palette {
    bg: Color::Reset,
    fg: Color::Reset,
    accent: Color::Rgb(255, 165, 0),
    frame: Color::Rgb(30, 144, 255),
    on_frame: Color::Black,
    title: Color::Cyan,
    scope: Color::Magenta,
    heading: Color::White,
    muted: Color::Gray,
    faint: Color::DarkGray,
    info: Color::LightBlue,
    completed: Color::Green,
    in_progress: Color::Cyan,
    stale: Color::Yellow,
    failed: Color::Red,
    queued: Color::Magenta,
};

/// <https://github.com/morhetz/gruvbox>
pub const GRUVBOX_DARK: Palette = Palette {
    bg: Color::from_u32(0x282828),
    fg: Color::from_u32(0xebdbb2),
    accent: Color::from_u32(0xfe8019),
    frame: Color::from_u32(0x83a598),
    on_frame: Color::from_u32(0x282828),
    title: Color::from_u32(0x8ec07c),
    scope: Color::from_u32(0xd3869b),
    heading: Color::from_u32(0xfbf1c7),
    muted: Color::from_u32(0xbdae93),
    faint: Color::from_u32(0x928374),
    info: Color::from_u32(0x83a598),
    completed: Color::from_u32(0xb8bb26),
    in_progress: Color::from_u32(0x8ec07c),
    stale: Color::from_u32(0xfabd2f),
    failed: Color::from_u32(0xfb4934),
    queued: Color::from_u32(0xd3869b),
};

/// <https://catppuccin.com/palette>
pub const CATPPUCCIN_FRAPPE: Palette = Palette {
    bg: Color::from_u32(0x303446),
    fg: Color::from_u32(0xc6d0f5),
    accent: Color::from_u32(0xef9f76),
    frame: Color::from_u32(0xbabbf1),
    on_frame: Color::from_u32(0x232634),
    title: Color::from_u32(0x99d1db),
    scope: Color::from_u32(0xca9ee6),
    heading: Color::from_u32(0xc6d0f5),
    muted: Color::from_u32(0xa5adce),
    faint: Color::from_u32(0x838ba7),
    info: Color::from_u32(0x8caaee),
    completed: Color::from_u32(0xa6d189),
    in_progress: Color::from_u32(0x99d1db),
    stale: Color::from_u32(0xe5c890),
    failed: Color::from_u32(0xe78284),
    queued: Color::from_u32(0xca9ee6),
};
