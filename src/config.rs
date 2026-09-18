//! TOML configuration and command-line override plumbing.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::roles::{ToneBand, ToneBands};
use crate::solver::GuardrailFloors;

#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveConfig {
    pub base_theme: Option<PathBuf>,
    pub palette_path: PathBuf,
    pub quality: i8,
    pub osc11: bool,
    pub bands: ToneBands,
    pub floors: GuardrailFloors,
}

impl Default for EffectiveConfig {
    fn default() -> Self {
        Self {
            base_theme: None,
            palette_path: default_palette_path(),
            quality: 2,
            osc11: false,
            bands: ToneBands::default(),
            floors: GuardrailFloors::default(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct FileConfig {
    base_theme: Option<String>,
    palette_path: Option<String>,
    quality: Option<i8>,
    osc11: Option<bool>,
    bands: FileBands,
    floors: FileFloors,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct FileBands {
    background: Option<FileBand>,
    text: Option<FileBand>,
    accent: Option<FileBand>,
    active: Option<FileBand>,
    highlight_text: Option<FileBand>,
    selection: Option<FileBand>,
    frame: Option<FileBand>,
    header_accent: Option<FileBand>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct FileBand {
    target: Option<f32>,
    min: Option<f32>,
    max: Option<f32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct FileFloors {
    accent_background: Option<f32>,
    text_background: Option<f32>,
    active_background: Option<f32>,
    highlight_active: Option<f32>,
    highlight_selection: Option<f32>,
    selection_active_delta_l: Option<f32>,
    frame_background: Option<f32>,
    peer_delta_e: Option<f32>,
    hue_separation: Option<f32>,
}

impl EffectiveConfig {
    /// Load an explicit config, or the default config when it exists.
    pub fn load(explicit_path: Option<&Path>) -> Result<Self, String> {
        let path = match explicit_path {
            Some(path) => Some(path.to_path_buf()),
            None => default_config_path()?,
        };
        let Some(path) = path else {
            return Ok(Self::default());
        };
        if !path.exists() {
            if explicit_path.is_some() {
                return Err(format!("config file does not exist: {}", path.display()));
            }
            return Ok(Self::default());
        }
        let source = std::fs::read_to_string(&path)
            .map_err(|error| format!("read config {}: {error}", path.display()))?;
        Self::from_toml(&source).map_err(|error| format!("config {}: {error}", path.display()))
    }

    fn from_toml(source: &str) -> Result<Self, String> {
        let file: FileConfig = toml::from_str(source).map_err(|error| error.to_string())?;
        let mut config = Self::default();
        if let Some(path) = file.base_theme {
            config.base_theme = Some(expand_tilde(&path)?);
        }
        if let Some(path) = file.palette_path {
            config.palette_path = expand_tilde(&path)?;
        }
        if let Some(quality) = file.quality {
            config.quality = quality;
        }
        if let Some(osc11) = file.osc11 {
            config.osc11 = osc11;
        }
        merge_band(&mut config.bands.background, file.bands.background);
        merge_band(&mut config.bands.text, file.bands.text);
        merge_band(&mut config.bands.accent, file.bands.accent);
        merge_band(&mut config.bands.active, file.bands.active);
        merge_band(&mut config.bands.highlight_text, file.bands.highlight_text);
        merge_band(&mut config.bands.selection, file.bands.selection);
        merge_band(&mut config.bands.frame, file.bands.frame);
        merge_band(&mut config.bands.header_accent, file.bands.header_accent);
        macro_rules! floor {
            ($name:ident) => {
                if let Some(value) = file.floors.$name {
                    config.floors.$name = value;
                }
            };
        }
        floor!(accent_background);
        floor!(text_background);
        floor!(active_background);
        floor!(highlight_active);
        floor!(highlight_selection);
        floor!(selection_active_delta_l);
        floor!(frame_background);
        floor!(peer_delta_e);
        floor!(hue_separation);
        config.validate()?;
        Ok(config)
    }

    /// Apply one `KEY=VALUE` command-line override.
    pub fn apply_override(&mut self, assignment: &str) -> Result<(), String> {
        let (key, value) = assignment
            .split_once('=')
            .ok_or_else(|| format!("override must be KEY=VALUE: {assignment}"))?;
        match key {
            "base_theme" => self.base_theme = Some(expand_tilde(value)?),
            "palette_path" => self.palette_path = expand_tilde(value)?,
            "quality" => self.quality = parse(value, key)?,
            "osc11" => self.osc11 = parse(value, key)?,
            _ if key.starts_with("bands.") => self.set_band_value(key, value)?,
            _ if key.starts_with("floors.") => self.set_floor_value(key, value)?,
            _ => return Err(format!("unknown config key: {key}")),
        }
        Ok(())
    }

    pub fn print_value(&self, key: &str) -> Result<String, String> {
        let value = match key {
            "base_theme" => self
                .base_theme
                .as_ref()
                .map_or_else(String::new, |path| path.display().to_string()),
            "palette_path" => self.palette_path.display().to_string(),
            "quality" => self.quality.to_string(),
            "osc11" => self.osc11.to_string(),
            _ if key.starts_with("bands.") => self.band_value(key)?.to_string(),
            _ if key.starts_with("floors.") => self.floor_value(key)?.to_string(),
            _ => return Err(format!("unknown config key: {key}")),
        };
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.palette_path.is_absolute() {
            return Err(format!(
                "palette_path must be absolute, got {}",
                self.palette_path.display()
            ));
        }
        if let Some(path) = &self.base_theme {
            if !path.is_absolute() {
                return Err(format!(
                    "base_theme must expand to an absolute path, got {}",
                    path.display()
                ));
            }
        }
        if !(0..=4).contains(&self.quality) {
            return Err(format!(
                "quality must be from 0 through 4, got {}",
                self.quality
            ));
        }
        for (name, band) in self.named_bands() {
            if ![band.min, band.target, band.max]
                .into_iter()
                .all(|value| value.is_finite() && (0.0..=1.0).contains(&value))
                || band.min > band.target
                || band.target > band.max
            {
                return Err(format!(
                    "bands.{name} must satisfy 0 <= min <= target <= max <= 1"
                ));
            }
        }
        for (name, value) in [
            ("accent_background", self.floors.accent_background),
            ("text_background", self.floors.text_background),
            ("active_background", self.floors.active_background),
            ("highlight_active", self.floors.highlight_active),
            ("highlight_selection", self.floors.highlight_selection),
            ("frame_background", self.floors.frame_background),
        ] {
            if !value.is_finite() || !(1.0..=21.0).contains(&value) {
                return Err(format!("floors.{name} must be from 1 through 21"));
            }
        }
        if !self.floors.peer_delta_e.is_finite() || self.floors.peer_delta_e < 20.0 {
            return Err("floors.peer_delta_e must be at least the relaxed floor 20".into());
        }
        if !self.floors.hue_separation.is_finite()
            || !(20.0..=180.0).contains(&self.floors.hue_separation)
        {
            return Err("floors.hue_separation must be from 20 through 180".into());
        }
        if !self.floors.selection_active_delta_l.is_finite()
            || !(0.0..=1.0).contains(&self.floors.selection_active_delta_l)
        {
            return Err("floors.selection_active_delta_l must be from 0 through 1".into());
        }
        Ok(())
    }

    fn named_bands(&self) -> [(&'static str, ToneBand); 8] {
        [
            ("background", self.bands.background),
            ("text", self.bands.text),
            ("accent", self.bands.accent),
            ("active", self.bands.active),
            ("highlight_text", self.bands.highlight_text),
            ("selection", self.bands.selection),
            ("frame", self.bands.frame),
            ("header_accent", self.bands.header_accent),
        ]
    }

    fn set_band_value(&mut self, key: &str, value: &str) -> Result<(), String> {
        let mut parts = key.split('.');
        let _ = parts.next();
        let role = parts
            .next()
            .ok_or_else(|| format!("unknown config key: {key}"))?;
        let field = parts
            .next()
            .ok_or_else(|| format!("unknown config key: {key}"))?;
        if parts.next().is_some() {
            return Err(format!("unknown config key: {key}"));
        }
        let band =
            band_mut(&mut self.bands, role).ok_or_else(|| format!("unknown config key: {key}"))?;
        let parsed = parse(value, key)?;
        match field {
            "target" => band.target = parsed,
            "min" => band.min = parsed,
            "max" => band.max = parsed,
            _ => return Err(format!("unknown config key: {key}")),
        }
        Ok(())
    }

    fn band_value(&self, key: &str) -> Result<f32, String> {
        let mut parts = key.split('.');
        let _ = parts.next();
        let role = parts
            .next()
            .ok_or_else(|| format!("unknown config key: {key}"))?;
        let field = parts
            .next()
            .ok_or_else(|| format!("unknown config key: {key}"))?;
        if parts.next().is_some() {
            return Err(format!("unknown config key: {key}"));
        }
        let band =
            band_ref(&self.bands, role).ok_or_else(|| format!("unknown config key: {key}"))?;
        match field {
            "target" => Ok(band.target),
            "min" => Ok(band.min),
            "max" => Ok(band.max),
            _ => Err(format!("unknown config key: {key}")),
        }
    }

    fn set_floor_value(&mut self, key: &str, value: &str) -> Result<(), String> {
        let parsed = parse(value, key)?;
        match key {
            "floors.accent_background" => self.floors.accent_background = parsed,
            "floors.text_background" => self.floors.text_background = parsed,
            "floors.active_background" => self.floors.active_background = parsed,
            "floors.highlight_active" => self.floors.highlight_active = parsed,
            "floors.highlight_selection" => self.floors.highlight_selection = parsed,
            "floors.selection_active_delta_l" => self.floors.selection_active_delta_l = parsed,
            "floors.frame_background" => self.floors.frame_background = parsed,
            "floors.peer_delta_e" => self.floors.peer_delta_e = parsed,
            "floors.hue_separation" => self.floors.hue_separation = parsed,
            _ => return Err(format!("unknown config key: {key}")),
        }
        Ok(())
    }

    fn floor_value(&self, key: &str) -> Result<f32, String> {
        match key {
            "floors.accent_background" => Ok(self.floors.accent_background),
            "floors.text_background" => Ok(self.floors.text_background),
            "floors.active_background" => Ok(self.floors.active_background),
            "floors.highlight_active" => Ok(self.floors.highlight_active),
            "floors.highlight_selection" => Ok(self.floors.highlight_selection),
            "floors.selection_active_delta_l" => Ok(self.floors.selection_active_delta_l),
            "floors.frame_background" => Ok(self.floors.frame_background),
            "floors.peer_delta_e" => Ok(self.floors.peer_delta_e),
            "floors.hue_separation" => Ok(self.floors.hue_separation),
            _ => Err(format!("unknown config key: {key}")),
        }
    }
}

fn default_palette_path() -> PathBuf {
    if let Some(root) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(root).join("rmpc-auto-theme/palette/palette.json");
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".config/rmpc-auto-theme/palette/palette.json")
}

fn merge_band(target: &mut ToneBand, source: Option<FileBand>) {
    if let Some(source) = source {
        if let Some(value) = source.target {
            target.target = value;
        }
        if let Some(value) = source.min {
            target.min = value;
        }
        if let Some(value) = source.max {
            target.max = value;
        }
    }
}

fn band_ref<'a>(bands: &'a ToneBands, role: &str) -> Option<&'a ToneBand> {
    match role {
        "background" => Some(&bands.background),
        "text" => Some(&bands.text),
        "accent" => Some(&bands.accent),
        "active" => Some(&bands.active),
        "highlight_text" => Some(&bands.highlight_text),
        "selection" => Some(&bands.selection),
        "frame" => Some(&bands.frame),
        "header_accent" => Some(&bands.header_accent),
        _ => None,
    }
}

fn band_mut<'a>(bands: &'a mut ToneBands, role: &str) -> Option<&'a mut ToneBand> {
    match role {
        "background" => Some(&mut bands.background),
        "text" => Some(&mut bands.text),
        "accent" => Some(&mut bands.accent),
        "active" => Some(&mut bands.active),
        "highlight_text" => Some(&mut bands.highlight_text),
        "selection" => Some(&mut bands.selection),
        "frame" => Some(&mut bands.frame),
        "header_accent" => Some(&mut bands.header_accent),
        _ => None,
    }
}

fn parse<T: std::str::FromStr>(value: &str, key: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("invalid value for {key}: {value}"))
}

fn default_config_path() -> Result<Option<PathBuf>, String> {
    if let Some(root) = std::env::var_os("XDG_CONFIG_HOME") {
        if root.is_empty() || !Path::new(&root).is_absolute() {
            return Err("XDG_CONFIG_HOME must be an absolute, non-empty path".into());
        }
        return Ok(Some(
            PathBuf::from(root).join("rmpc-auto-theme/config.toml"),
        ));
    }
    let Some(home) = std::env::var_os("HOME") else {
        return Ok(None);
    };
    if home.is_empty() {
        return Err("HOME is empty".into());
    }
    Ok(Some(
        PathBuf::from(home).join(".config/rmpc-auto-theme/config.toml"),
    ))
}

fn expand_tilde(value: &str) -> Result<PathBuf, String> {
    if value == "~" || value.starts_with("~/") {
        let home = std::env::var_os("HOME").ok_or_else(|| "HOME is unset".to_string())?;
        if home.is_empty() {
            return Err("HOME is empty".into());
        }
        let suffix = value.strip_prefix("~/").unwrap_or("");
        Ok(PathBuf::from(home).join(suffix))
    } else if value.starts_with('~') {
        Err(format!("unsupported home path: {value}"))
    } else {
        Ok(PathBuf::from(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_config_merges_over_spec_defaults() {
        let config = EffectiveConfig::from_toml(
            "quality = 4\nosc11 = true\n[bands.accent]\ntarget = 0.75\n\
             [bands.selection]\ntarget = 0.76\n[floors]\nhighlight_selection = 5.0\n\
             selection_active_delta_l = 0.09\n",
        )
        .unwrap();
        assert_eq!(config.quality, 4);
        assert!(config.osc11);
        assert_eq!(config.bands.accent.target, 0.75);
        assert_eq!(config.bands.accent.min, 0.68);
        assert_eq!(config.bands.selection.target, 0.76);
        assert_eq!(config.bands.selection.min, 0.72);
        assert_eq!(config.floors.highlight_selection, 5.0);
        assert_eq!(config.floors.selection_active_delta_l, 0.09);
    }

    #[test]
    fn unknown_and_out_of_range_values_fail_loudly() {
        assert!(EffectiveConfig::from_toml("mystery = 1").is_err());
        assert!(EffectiveConfig::from_toml("quality = 5").is_err());
        assert!(EffectiveConfig::from_toml("[bands.text]\nmin = 0.95").is_err());
        assert!(EffectiveConfig::from_toml("[floors]\nselection_active_delta_l = 1.1").is_err());
    }

    #[test]
    fn every_tunable_scalar_has_override_and_query_support() {
        let mut config = EffectiveConfig::default();
        config.apply_override("bands.frame.target=0.53").unwrap();
        config
            .apply_override("bands.selection.target=0.76")
            .unwrap();
        config
            .apply_override("floors.accent_background=5.0")
            .unwrap();
        config
            .apply_override("floors.highlight_selection=5.0")
            .unwrap();
        config
            .apply_override("floors.selection_active_delta_l=0.09")
            .unwrap();
        assert_eq!(config.print_value("bands.frame.target").unwrap(), "0.53");
        assert_eq!(
            config.print_value("bands.selection.target").unwrap(),
            "0.76"
        );
        assert_eq!(config.print_value("floors.accent_background").unwrap(), "5");
        assert_eq!(
            config.print_value("floors.highlight_selection").unwrap(),
            "5"
        );
        assert_eq!(
            config
                .print_value("floors.selection_active_delta_l")
                .unwrap(),
            "0.09"
        );
    }
}
