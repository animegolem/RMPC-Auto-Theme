//! Span-preserving patching of the governed rmpc theme color fields.
//!
//! The base document is parsed for validation, but is never serialized again:
//! replacing only the selected literal spans keeps comments, pragmas, ordering,
//! and formatting byte-for-byte intact.

use std::collections::{BTreeSet, HashMap};

use crate::roles::{Role, RolePalette};
use serde::Deserialize;

/// The patched document and the concrete governed paths considered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchReport {
    pub output: String,
    pub patched_paths: Vec<String>,
    pub untouched_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct Binding {
    path: &'static str,
    role: Role,
}

const BINDINGS: &[Binding] = &[
    Binding {
        path: "background_color",
        role: Role::Background,
    },
    Binding {
        path: "header_background_color",
        role: Role::Background,
    },
    Binding {
        path: "modal_background_color",
        role: Role::Background,
    },
    Binding {
        path: "text_color",
        role: Role::Text,
    },
    Binding {
        path: "borders_style.fg",
        role: Role::Frame,
    },
    Binding {
        path: "highlight_border_style.fg",
        role: Role::Frame,
    },
    Binding {
        path: "current_item_style.fg",
        role: Role::HighlightText,
    },
    Binding {
        path: "current_item_style.bg",
        role: Role::Selection,
    },
    Binding {
        path: "highlighted_item_style.fg",
        role: Role::HighlightText,
    },
    Binding {
        path: "highlighted_item_style.bg",
        role: Role::Active,
    },
    Binding {
        path: "tab_bar.active_style.fg",
        role: Role::HighlightText,
    },
    Binding {
        path: "tab_bar.active_style.bg",
        role: Role::Active,
    },
    Binding {
        path: "tab_bar.inactive_style.fg",
        role: Role::Inactive,
    },
    Binding {
        path: "tab_bar.inactive_style.bg",
        role: Role::Background,
    },
    Binding {
        path: "preview_label_style.fg",
        role: Role::HeaderAccent,
    },
    Binding {
        path: "preview_metadata_group_style.fg",
        role: Role::HeaderAccent,
    },
    Binding {
        path: "level_styles.info.fg",
        role: Role::Accent,
    },
    Binding {
        path: "progress_bar.track_style.fg",
        role: Role::Frame,
    },
    Binding {
        path: "progress_bar.track_style.bg",
        role: Role::Background,
    },
    Binding {
        path: "progress_bar.elapsed_style.fg",
        role: Role::Active,
    },
    Binding {
        path: "progress_bar.elapsed_style.bg",
        role: Role::Background,
    },
    Binding {
        path: "progress_bar.thumb_style.fg",
        role: Role::Frame,
    },
    Binding {
        path: "progress_bar.thumb_style.bg",
        role: Role::Background,
    },
    Binding {
        path: "scrollbar.track_style.fg",
        role: Role::Frame,
    },
    Binding {
        path: "scrollbar.track_style.bg",
        role: Role::Background,
    },
    Binding {
        path: "scrollbar.ends_style.fg",
        role: Role::Frame,
    },
    Binding {
        path: "scrollbar.ends_style.bg",
        role: Role::Background,
    },
    Binding {
        path: "scrollbar.thumb_style.fg",
        role: Role::Frame,
    },
    Binding {
        path: "scrollbar.thumb_style.bg",
        role: Role::Background,
    },
];

const LEVEL_BACKGROUND_PATTERN: &str = "level_styles.*.bg";

/// Patch governed color string literals while preserving all other source bytes.
pub fn patch_theme(base: &str, palette: &RolePalette) -> Result<PatchReport, String> {
    validate_ron(base, "base theme")?;

    let tokens = lex(base)?;
    let mut walker = Walker::new(&tokens, palette);
    walker.walk_root()?;
    let mut findings = walker.finish();

    findings
        .replacements
        .sort_by_key(|replacement| replacement.start);
    for pair in findings.replacements.windows(2) {
        if pair[0].end > pair[1].start {
            return Err("internal emitter error: overlapping replacement spans".to_owned());
        }
    }

    let mut output = String::with_capacity(base.len());
    let mut cursor = 0;
    for replacement in &findings.replacements {
        output.push_str(&base[cursor..replacement.start]);
        output.push('"');
        output.push_str(&replacement.color);
        output.push('"');
        cursor = replacement.end;
    }
    output.push_str(&base[cursor..]);

    validate_ron(&output, "patched theme")?;
    Ok(PatchReport {
        output,
        patched_paths: findings.patched_paths,
        untouched_paths: findings.untouched_paths,
    })
}

fn validate_ron(source: &str, label: &str) -> Result<(), String> {
    let mut deserializer = ron::de::Deserializer::from_str(source)
        .map_err(|error| format!("invalid {label}: {error}"))?;
    serde::de::IgnoredAny::deserialize(&mut deserializer)
        .map_err(|error| format!("invalid {label}: {error}"))?;
    deserializer
        .end()
        .map_err(|error| format!("invalid {label}: {error}"))
}

#[derive(Debug)]
struct Replacement {
    start: usize,
    end: usize,
    color: String,
}

#[derive(Debug)]
struct Findings {
    replacements: Vec<Replacement>,
    patched_paths: Vec<String>,
    untouched_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenKind {
    Ident(String),
    String,
    Open(char),
    Close(char),
    Colon,
    Comma,
    Other,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    start: usize,
    end: usize,
}

fn lex(source: &str) -> Result<Vec<Token>, String> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            byte if byte.is_ascii_whitespace() => index += 1,
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index = skip_block_comment(bytes, index)?;
            }
            b'"' => {
                let end = scan_quoted(bytes, index)?;
                tokens.push(Token {
                    kind: TokenKind::String,
                    start: index,
                    end,
                });
                index = end;
            }
            b'r' if raw_string_hashes(bytes, index).is_some() => {
                let end = scan_raw_string(bytes, index)?;
                tokens.push(Token {
                    kind: TokenKind::String,
                    start: index,
                    end,
                });
                index = end;
            }
            b'(' | b'[' | b'{' => {
                tokens.push(Token {
                    kind: TokenKind::Open(bytes[index] as char),
                    start: index,
                    end: index + 1,
                });
                index += 1;
            }
            b')' | b']' | b'}' => {
                tokens.push(Token {
                    kind: TokenKind::Close(bytes[index] as char),
                    start: index,
                    end: index + 1,
                });
                index += 1;
            }
            b':' => {
                tokens.push(Token {
                    kind: TokenKind::Colon,
                    start: index,
                    end: index + 1,
                });
                index += 1;
            }
            b',' => {
                tokens.push(Token {
                    kind: TokenKind::Comma,
                    start: index,
                    end: index + 1,
                });
                index += 1;
            }
            byte if is_ident_start(byte) => {
                let start = index;
                index += 1;
                while index < bytes.len() && is_ident_continue(bytes[index]) {
                    index += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::Ident(source[start..index].to_owned()),
                    start,
                    end: index,
                });
            }
            _ => {
                let start = index;
                index += 1;
                tokens.push(Token {
                    kind: TokenKind::Other,
                    start,
                    end: index,
                });
            }
        }
    }
    Ok(tokens)
}

fn skip_block_comment(bytes: &[u8], start: usize) -> Result<usize, String> {
    let mut depth = 1usize;
    let mut index = start + 2;
    while index + 1 < bytes.len() {
        match (&bytes[index..index + 2], depth) {
            (b"/*", _) => {
                depth += 1;
                index += 2;
            }
            (b"*/", 1) => return Ok(index + 2),
            (b"*/", _) => {
                depth -= 1;
                index += 2;
            }
            _ => index += 1,
        }
    }
    Err("unterminated block comment in base theme".to_owned())
}

fn scan_quoted(bytes: &[u8], start: usize) -> Result<usize, String> {
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'"' => return Ok(index + 1),
            _ => index += 1,
        }
    }
    Err("unterminated string in base theme".to_owned())
}

fn raw_string_hashes(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'r') {
        return None;
    }
    let mut index = start + 1;
    while bytes.get(index) == Some(&b'#') {
        index += 1;
    }
    (bytes.get(index) == Some(&b'"')).then_some(index - start - 1)
}

fn scan_raw_string(bytes: &[u8], start: usize) -> Result<usize, String> {
    let hashes = raw_string_hashes(bytes, start).expect("caller checks raw-string prefix");
    let mut index = start + hashes + 2;
    while index < bytes.len() {
        if bytes[index] == b'"'
            && bytes.get(index + 1..index + 1 + hashes)
                == Some(&bytes[start + 1..start + 1 + hashes])
        {
            return Ok(index + 1 + hashes);
        }
        index += 1;
    }
    Err("unterminated raw string in base theme".to_owned())
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_continue(byte: u8) -> bool {
    is_ident_start(byte) || byte.is_ascii_digit()
}

struct Walker<'a> {
    tokens: &'a [Token],
    index: usize,
    palette: &'a RolePalette,
    replacements: Vec<Replacement>,
    states: HashMap<String, PathState>,
    level_children: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathState {
    Patched,
    Untouched,
}

impl<'a> Walker<'a> {
    fn new(tokens: &'a [Token], palette: &'a RolePalette) -> Self {
        Self {
            tokens,
            index: 0,
            palette,
            replacements: Vec::new(),
            states: HashMap::new(),
            level_children: BTreeSet::new(),
        }
    }

    fn walk_root(&mut self) -> Result<(), String> {
        while self.index < self.tokens.len() {
            self.walk_value(&[])?;
            if self.peek_kind() == Some(&TokenKind::Comma) {
                self.index += 1;
            }
        }
        Ok(())
    }

    fn walk_value(&mut self, path: &[String]) -> Result<(), String> {
        let Some(token) = self.tokens.get(self.index) else {
            return Ok(());
        };
        match &token.kind {
            TokenKind::String => {
                let token = token.clone();
                self.index += 1;
                self.observe_string(path, token.start, token.end);
            }
            TokenKind::Ident(identifier) => {
                let is_none = identifier == "None";
                self.index += 1;
                if is_none {
                    self.observe_none(path);
                }
                if matches!(self.peek_kind(), Some(TokenKind::Open('('))) {
                    self.walk_group(path, '(', ')')?;
                }
            }
            TokenKind::Open('(') => self.walk_group(path, '(', ')')?,
            TokenKind::Open('[') => self.walk_group(path, '[', ']')?,
            TokenKind::Open('{') => self.walk_group(path, '{', '}')?,
            TokenKind::Close(_) | TokenKind::Comma => {}
            _ => self.index += 1,
        }
        Ok(())
    }

    fn walk_group(&mut self, path: &[String], open: char, close: char) -> Result<(), String> {
        match self.peek_kind() {
            Some(TokenKind::Open(actual)) if *actual == open => self.index += 1,
            _ => return Err(format!("internal emitter error: expected `{open}`")),
        }

        while self.index < self.tokens.len() {
            if matches!(self.peek_kind(), Some(TokenKind::Close(actual)) if *actual == close) {
                self.index += 1;
                return Ok(());
            }

            let field = match (
                self.peek_kind(),
                self.tokens.get(self.index + 1).map(|t| &t.kind),
            ) {
                (Some(TokenKind::Ident(field)), Some(TokenKind::Colon)) => Some(field.clone()),
                _ => None,
            };
            if let Some(field) = field {
                self.index += 2;
                let mut child_path = path.to_vec();
                child_path.push(field.clone());
                if path.len() == 1 && path[0] == "level_styles" {
                    self.level_children.insert(field);
                }
                self.walk_value(&child_path)?;
            } else {
                self.walk_value(path)?;
            }

            if self.peek_kind() == Some(&TokenKind::Comma) {
                self.index += 1;
            }
        }
        Err(format!("internal emitter error: unclosed `{open}` group"))
    }

    fn observe_string(&mut self, path: &[String], start: usize, end: usize) {
        let path = path.join(".");
        let Some(role) = role_for_path(&path) else {
            return;
        };
        let rgb = self.palette.get(role).rgb;
        self.replacements.push(Replacement {
            start,
            end,
            color: format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]),
        });
        self.states.insert(path, PathState::Patched);
    }

    fn observe_none(&mut self, path: &[String]) {
        let path = path.join(".");
        if role_for_path(&path).is_some() {
            self.states.entry(path).or_insert(PathState::Untouched);
        }
    }

    fn finish(mut self) -> Findings {
        for binding in BINDINGS {
            self.states
                .entry(binding.path.to_owned())
                .or_insert(PathState::Untouched);
        }
        if self.level_children.is_empty() {
            self.states
                .entry(LEVEL_BACKGROUND_PATTERN.to_owned())
                .or_insert(PathState::Untouched);
        } else {
            for child in self.level_children {
                let path = format!("level_styles.{child}.bg");
                self.states.entry(path).or_insert(PathState::Untouched);
            }
        }

        let mut patched_paths = Vec::new();
        let mut untouched_paths = Vec::new();
        let mut ordered_paths: Vec<_> = self.states.into_iter().collect();
        ordered_paths.sort_by(|(left, _), (right, _)| left.cmp(right));
        for (path, state) in ordered_paths {
            match state {
                PathState::Patched => patched_paths.push(path),
                PathState::Untouched => untouched_paths.push(path),
            }
        }
        Findings {
            replacements: self.replacements,
            patched_paths,
            untouched_paths,
        }
    }

    fn peek_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.index).map(|token| &token.kind)
    }
}

fn role_for_path(path: &str) -> Option<Role> {
    BINDINGS
        .iter()
        .find_map(|binding| (binding.path == path).then_some(binding.role))
        .or_else(|| {
            let mut parts = path.split('.');
            (parts.next() == Some("level_styles")
                && parts.next().is_some()
                && parts.next() == Some("bg")
                && parts.next().is_none())
            .then_some(Role::Background)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roles::{AssignedColor, SolverTier};

    fn palette() -> RolePalette {
        let colors = Role::ALL.map(|role| {
            let rgb = match role {
                Role::Background => [0x10, 0x11, 0x12],
                Role::Text => [0xe0, 0xe1, 0xe2],
                Role::Accent => [0x21, 0x43, 0x65],
                Role::Active => [0x76, 0x54, 0x32],
                Role::HighlightText => [0x01, 0x23, 0x45],
                Role::Selection => [0xba, 0xdc, 0x0d],
                Role::Frame => [0x77, 0x88, 0x99],
                Role::HeaderAccent => [0xab, 0xcd, 0xef],
                Role::Inactive => [0xff, 0x00, 0xff],
            };
            AssignedColor {
                role,
                rgb,
                oklab: [0.5, 0.0, 0.0],
                oklch: [0.5, 0.0, 250.0],
                origin: "emitter-test".to_owned(),
                confidence: 1.0,
                tier: SolverTier::Strict,
            }
        });
        RolePalette::from_assignments(colors).expect("valid role order")
    }

    #[test]
    fn preserves_custom_layout_comments_and_unmapped_component_colors() {
        let base = r##"#![enable(implicit_some)]
(
  // spacing and this comment are user-owned
  background_color : "navy", /* retain me */
  text_color:"white",
  components: {"custom": (style: (fg: "red", bg: "blue"))},
)
"##;
        let expected = r##"#![enable(implicit_some)]
(
  // spacing and this comment are user-owned
  background_color : "#101112", /* retain me */
  text_color:"#e0e1e2",
  components: {"custom": (style: (fg: "red", bg: "blue"))},
)
"##;

        let report = patch_theme(base, &palette()).expect("valid theme");
        assert_eq!(report.output, expected);
        assert!(report
            .patched_paths
            .contains(&"background_color".to_owned()));
        assert!(report.patched_paths.contains(&"text_color".to_owned()));
        assert!(report.output.contains("fg: \"red\", bg: \"blue\""));
    }

    #[test]
    fn patches_level_wildcards_and_reports_none_and_absent_children() {
        let base = r##"(
            level_styles: (
                info: (fg: "old-accent", bg: "old-bg"),
                warn: (fg: "yellow", bg: None),
                error: (fg: "red"),
            ),
        )"##;
        let report = patch_theme(base, &palette()).expect("valid theme");

        assert!(report
            .output
            .contains("info: (fg: \"#214365\", bg: \"#101112\")"));
        assert!(report.output.contains("warn: (fg: \"yellow\", bg: None)"));
        assert!(report
            .patched_paths
            .contains(&"level_styles.info.fg".to_owned()));
        assert!(report
            .patched_paths
            .contains(&"level_styles.info.bg".to_owned()));
        assert!(report
            .untouched_paths
            .contains(&"level_styles.warn.bg".to_owned()));
        assert!(report
            .untouched_paths
            .contains(&"level_styles.error.bg".to_owned()));
    }

    #[test]
    fn none_and_absent_fixed_mappings_are_left_byte_identical_and_reported() {
        let base = "(background_color: None, progress_bar: (track_style: None))";
        let report = patch_theme(base, &palette()).expect("valid theme");

        assert_eq!(report.output, base);
        assert!(report
            .untouched_paths
            .contains(&"background_color".to_owned()));
        assert!(report.untouched_paths.contains(&"text_color".to_owned()));
        assert!(report
            .untouched_paths
            .contains(&"progress_bar.track_style.fg".to_owned()));
        assert!(report
            .untouched_paths
            .contains(&"progress_bar.track_style.bg".to_owned()));
    }

    #[test]
    fn patches_progress_and_scrollbar_nested_paths() {
        let base = r##"(
            progress_bar: (
                track_style: (fg: "a", bg: "b"),
                elapsed_style: (fg: "c", bg: "d"),
                thumb_style: (fg: "e", bg: "f"),
            ),
            scrollbar: (
                track_style: (fg: "g", bg: "h"),
                ends_style: (fg: "i", bg: "j"),
                thumb_style: (fg: "k", bg: "l"),
            ),
        )"##;
        let report = patch_theme(base, &palette()).expect("valid theme");

        assert!(report
            .output
            .contains("track_style: (fg: \"#778899\", bg: \"#101112\")"));
        assert!(report
            .output
            .contains("elapsed_style: (fg: \"#765432\", bg: \"#101112\")"));
        assert!(report
            .output
            .contains("thumb_style: (fg: \"#778899\", bg: \"#101112\")"));
        assert_eq!(
            report
                .patched_paths
                .iter()
                .filter(|path| path.starts_with("progress_bar."))
                .count(),
            6
        );
        assert_eq!(
            report
                .patched_paths
                .iter()
                .filter(|path| path.starts_with("scrollbar."))
                .count(),
            6
        );
    }

    #[test]
    fn output_and_reports_are_deterministic() {
        let base = r##"(
            tab_bar: (
                active_style: (fg: "a", bg: "b"),
                inactive_style: (fg: "c", bg: "d"),
            ),
        )"##;
        let first = patch_theme(base, &palette()).expect("first patch");
        let second = patch_theme(base, &palette()).expect("second patch");
        assert_eq!(first, second);
        assert!(first
            .output
            .contains("inactive_style: (fg: \"#778899\", bg: \"#101112\")"));
    }

    #[test]
    fn rejects_invalid_ron_before_walking() {
        let error = patch_theme("(background_color: \"unterminated)", &palette())
            .expect_err("syntax error must fail");
        assert!(error.starts_with("invalid base theme:"), "{error}");
    }

    #[test]
    fn path_precision_does_not_patch_similarly_named_or_component_local_fields() {
        let base = r##"(
            nested: (background_color: "not-top-level"),
            components: {
                "progress_bar": (track_style: (fg: "component-fg", bg: "component-bg")),
            },
            progress_bar: (track_style: (fg: "governed-fg", bg: "governed-bg")),
        )"##;
        let report = patch_theme(base, &palette()).expect("valid theme");

        assert!(report
            .output
            .contains("background_color: \"not-top-level\""));
        assert!(report
            .output
            .contains("fg: \"component-fg\", bg: \"component-bg\""));
        assert!(report.output.contains("fg: \"#778899\", bg: \"#101112\""));
    }
}
