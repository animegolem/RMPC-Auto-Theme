//! Semantic colors produced by the album-art role solver.

/// A semantic color slot in an rmpc theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Background,
    Text,
    Accent,
    Active,
    HighlightText,
    Frame,
    HeaderAccent,
    Inactive,
}

impl Role {
    pub const ALL: [Self; 8] = [
        Self::Background,
        Self::Text,
        Self::Accent,
        Self::Active,
        Self::HighlightText,
        Self::Frame,
        Self::HeaderAccent,
        Self::Inactive,
    ];

    pub const fn tone_band(self) -> ToneBand {
        match self {
            Self::Background => BACKGROUND_TONE_BAND,
            Self::Text => TEXT_TONE_BAND,
            Self::Accent => ACCENT_TONE_BAND,
            Self::Active => ACTIVE_TONE_BAND,
            Self::HighlightText => HIGHLIGHT_TEXT_TONE_BAND,
            Self::Frame | Self::Inactive => FRAME_TONE_BAND,
            Self::HeaderAccent => HEADER_ACCENT_TONE_BAND,
        }
    }

    pub const fn tone_band_in(self, bands: &ToneBands) -> ToneBand {
        match self {
            Self::Background => bands.background,
            Self::Text => bands.text,
            Self::Accent => bands.accent,
            Self::Active => bands.active,
            Self::HighlightText => bands.highlight_text,
            Self::Frame | Self::Inactive => bands.frame,
            Self::HeaderAccent => bands.header_accent,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Text => "text",
            Self::Accent => "accent",
            Self::Active => "active",
            Self::HighlightText => "highlight-text",
            Self::Frame => "frame",
            Self::HeaderAccent => "header-accent",
            Self::Inactive => "inactive",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::Background => 0,
            Self::Text => 1,
            Self::Accent => 2,
            Self::Active => 3,
            Self::HighlightText => 4,
            Self::Frame => 5,
            Self::HeaderAccent => 6,
            Self::Inactive => 7,
        }
    }
}

/// The preferred lightness and permitted OKLab lightness interval for a role.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToneBand {
    pub target: f32,
    pub min: f32,
    pub max: f32,
}

impl ToneBand {
    pub const fn contains(self, lightness: f32) -> bool {
        lightness >= self.min && lightness <= self.max
    }
}

pub const BACKGROUND_TONE_BAND: ToneBand = ToneBand {
    target: 0.13,
    min: 0.10,
    max: 0.16,
};
pub const TEXT_TONE_BAND: ToneBand = ToneBand {
    target: 0.88,
    min: 0.82,
    max: 0.93,
};
pub const ACCENT_TONE_BAND: ToneBand = ToneBand {
    target: 0.74,
    min: 0.68,
    max: 0.80,
};
pub const ACTIVE_TONE_BAND: ToneBand = ToneBand {
    target: 0.65,
    min: 0.60,
    max: 0.70,
};
pub const HIGHLIGHT_TEXT_TONE_BAND: ToneBand = ToneBand {
    target: 0.14,
    min: 0.10,
    max: 0.18,
};
pub const FRAME_TONE_BAND: ToneBand = ToneBand {
    target: 0.52,
    min: 0.48,
    max: 0.58,
};
pub const HEADER_ACCENT_TONE_BAND: ToneBand = ToneBand {
    target: 0.76,
    min: 0.70,
    max: 0.82,
};

/// Effective per-role lightness search bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToneBands {
    pub background: ToneBand,
    pub text: ToneBand,
    pub accent: ToneBand,
    pub active: ToneBand,
    pub highlight_text: ToneBand,
    pub frame: ToneBand,
    pub header_accent: ToneBand,
}

impl Default for ToneBands {
    fn default() -> Self {
        Self {
            background: BACKGROUND_TONE_BAND,
            text: TEXT_TONE_BAND,
            accent: ACCENT_TONE_BAND,
            active: ACTIVE_TONE_BAND,
            highlight_text: HIGHLIGHT_TEXT_TONE_BAND,
            frame: FRAME_TONE_BAND,
            header_accent: HEADER_ACCENT_TONE_BAND,
        }
    }
}

/// The constraint pass that produced an assigned color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverTier {
    Strict,
    Relaxed,
    Synthesized,
    Neutral,
}

/// One final, gamut-mapped semantic color plus its provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct AssignedColor {
    pub role: Role,
    pub rgb: [u8; 3],
    pub oklab: [f32; 3],
    pub oklch: [f32; 3],
    pub origin: String,
    pub confidence: f32,
    pub tier: SolverTier,
}

/// The complete set of primary semantic colors.
#[derive(Debug, Clone, PartialEq)]
pub struct RolePalette {
    colors: [AssignedColor; 8],
}

impl RolePalette {
    pub(crate) fn new(colors: [AssignedColor; 8]) -> Self {
        Self { colors }
    }

    /// Construct a palette from role-ordered assignments.
    ///
    /// The array must follow [`Role::ALL`]. `Inactive` is always rewritten as
    /// an exact color alias of `Frame`, irrespective of its supplied value.
    pub fn from_assignments(mut colors: [AssignedColor; 8]) -> Result<Self, String> {
        for (index, role) in Role::ALL.into_iter().enumerate() {
            if colors[index].role != role {
                return Err(format!(
                    "assignment {index} is {:?}, expected {role:?}",
                    colors[index].role
                ));
            }
        }
        let frame = colors[Role::Frame.index()].clone();
        colors[Role::Inactive.index()] = AssignedColor {
            role: Role::Inactive,
            origin: format!("alias:{}", frame.origin),
            ..frame
        };
        Ok(Self::new(colors))
    }

    pub fn get(&self, role: Role) -> &AssignedColor {
        &self.colors[role.index()]
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &AssignedColor> {
        self.colors.iter()
    }
}
