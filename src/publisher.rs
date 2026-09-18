//! File-addressed MPD artwork acquisition and notan-palette/1 publication.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use color_core::color::{oklab_to_oklch, rgb8_to_oklab};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::EffectiveConfig;
use crate::roles::{Role, SolverTier};
use crate::solver;

const MAX_ART_BYTES: usize = 32 * 1024 * 1024;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const REVISION: &str = match option_env!("RMPC_THEME_REVISION") {
    Some(revision) => revision,
    None => env!("CARGO_PKG_VERSION"),
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Admission {
    pub token: String,
    pub sequence: u64,
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub art_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Producer {
    name: &'static str,
    revision: &'static str,
    spec_revision: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleRecord {
    pub hex: String,
    pub oklch: [f64; 3],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias_of: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derived_from: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VerifiedPair {
    Contrast {
        fg: String,
        bg: String,
        floor: f64,
        measured: f64,
    },
    DeltaL {
        roles: [String; 2],
        floor: f64,
        measured: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterRecord {
    pub id: usize,
    pub share: f64,
    pub hex: String,
    pub oklch_extraction: [f64; 3],
}

#[derive(Debug, Clone, Serialize)]
pub struct PaletteArtifact {
    schema: &'static str,
    schema_version: &'static str,
    producer: Producer,
    features: Vec<&'static str>,
    sequence: u64,
    generated_at: String,
    status: &'static str,
    track: Track,
    #[serde(skip_serializing_if = "Option::is_none")]
    roles: Option<BTreeMap<String, RoleRecord>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verified_pairs: Option<Vec<VerifiedPair>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    clusters: Option<Vec<ClusterRecord>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheRecord {
    theme: String,
    roles: BTreeMap<String, RoleRecord>,
    verified_pairs: Vec<VerifiedPair>,
    clusters: Vec<ClusterRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishResult {
    Ok(String),
    NoArt,
    Failed,
    Superseded,
}

impl PublishResult {
    pub fn line(&self) -> String {
        match self {
            Self::Ok(hash) => format!("ok {hash}"),
            Self::NoArt => "no-art".to_owned(),
            Self::Failed => "failed".to_owned(),
            Self::Superseded => "superseded".to_owned(),
        }
    }
}

fn parent(path: &Path) -> Result<&Path, String> {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| format!("path needs a parent: {}", path.display()))
}

fn lock_path(config: &EffectiveConfig) -> Result<PathBuf, String> {
    Ok(parent(&config.palette_path)?.join("admission.lock"))
}

fn state_path(config: &EffectiveConfig) -> Result<PathBuf, String> {
    Ok(parent(&config.palette_path)?.join("admission.json"))
}

struct ShortLock(File);

impl ShortLock {
    fn acquire(config: &EffectiveConfig) -> Result<Self, String> {
        fs::create_dir_all(parent(&config.palette_path)?)
            .map_err(|e| format!("create palette directory: {e}"))?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path(config)?)
            .map_err(|e| format!("open admission lock: {e}"))?;
        // SAFETY: flock acts on this live file descriptor and is released on drop.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(format!(
                "lock admission: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(Self(file))
    }
}

impl Drop for ShortLock {
    fn drop(&mut self) {
        // SAFETY: same live descriptor acquired by ShortLock::acquire.
        unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}

fn read_admission(config: &EffectiveConfig) -> Result<Option<Admission>, String> {
    match fs::read(state_path(config)?) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| format!("parse admission state: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("read admission state: {e}")),
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::create_dir_all(parent(path)?).map_err(|e| format!("create {}: {e}", path.display()))?;
    let nonce = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let temp = path.with_file_name(format!(
        ".{}.{}.{}.{nonce}.tmp",
        path.file_name().unwrap().to_string_lossy(),
        std::process::id(),
        timestamp
    ));
    let mut out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| format!("create {}: {e}", temp.display()))?;
    let result = (|| {
        out.write_all(bytes)
            .map_err(|e| format!("write {}: {e}", temp.display()))?;
        out.sync_all()
            .map_err(|e| format!("sync {}: {e}", temp.display()))?;
        fs::rename(&temp, path).map_err(|e| format!("rename {}: {e}", path.display()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

/// Admit an event under a short shared lock, assigning a unique token and sequence.
pub fn admit_event(config: &EffectiveConfig, file: Option<String>) -> Result<Admission, String> {
    let _lock = ShortLock::acquire(config)?;
    let sequence = read_admission(config)?.map_or(1, |a| a.sequence.saturating_add(1));
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("system clock: {e}"))?
        .as_nanos();
    let file = file.filter(|file| !file.is_empty());
    let admission = Admission {
        token: format!("{}:{nanos}:{sequence}", std::process::id()),
        sequence,
        file,
    };
    let bytes = serde_json::to_vec(&admission).map_err(|e| e.to_string())?;
    atomic_write(&state_path(config)?, &bytes)?;
    Ok(admission)
}

fn artifact(admission: &Admission, track: Track, status: &'static str) -> PaletteArtifact {
    PaletteArtifact {
        schema: "notan-palette/1",
        schema_version: "1.0.0",
        producer: Producer {
            name: "rmpc-auto-theme",
            revision: REVISION,
            spec_revision: "0.13",
        },
        features: Vec::new(),
        sequence: admission.sequence,
        generated_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        status,
        track,
        roles: None,
        verified_pairs: None,
        clusters: None,
        error: None,
    }
}

fn publish(
    config: &EffectiveConfig,
    admission: &Admission,
    envelope: &PaletteArtifact,
    theme: Option<(&Path, &str)>,
) -> Result<bool, String> {
    let bytes = serde_json::to_vec_pretty(envelope).map_err(|e| format!("encode palette: {e}"))?;
    let _lock = ShortLock::acquire(config)?;
    let current = read_admission(config)?;
    if current.as_ref().is_none_or(|current| {
        current.token != admission.token
            || current.file != admission.file
            || current.sequence != admission.sequence
    }) {
        return Ok(false);
    }
    // Under the same short lock as admission: no newer event can enter between
    // the token check and either rename.
    if let Some((path, document)) = theme {
        atomic_write(path, document.as_bytes())?;
    }
    atomic_write(&config.palette_path, &bytes)?;
    Ok(true)
}

fn blank_track(file: Option<String>) -> Track {
    Track {
        file,
        artist: None,
        album: None,
        title: None,
        art_sha256: None,
    }
}

/// One admitted event. Handled failures publish `failed` rather than exiting nonzero.
pub fn publish_event(
    config: &EffectiveConfig,
    admission: &Admission,
    theme_output: Option<&Path>,
) -> Result<PublishResult, String> {
    let mut track = blank_track(admission.file.clone());
    let file = match admission.file.as_deref() {
        Some(file) if !file.is_empty() => file,
        _ => {
            return failed(
                config,
                admission,
                track,
                "admitted FILE is absent".to_owned(),
            )
        }
    };
    if file.contains(['\n', '\r']) {
        return failed(
            config,
            admission,
            track,
            "admitted FILE contains an MPD protocol line break".to_owned(),
        );
    }
    let mut mpd = match Mpd::connect() {
        Ok(mpd) => mpd,
        Err(error) => return failed(config, admission, track, error),
    };
    match mpd.track_info(file) {
        Ok(info) => {
            track.artist = info.get("Artist").cloned();
            track.album = info.get("Album").cloned();
            track.title = info.get("Title").cloned();
        }
        Err(error) => eprintln!("publisher: metadata unavailable for admitted FILE: {error}"),
    }
    let art = match mpd.art(file) {
        Ok(art) => art,
        Err(error) => return failed(config, admission, track, error),
    };
    match art {
        Some(bytes) => {
            let hash = format!("{:x}", Sha256::digest(&bytes));
            track.art_sha256 = Some(hash.clone());
            let base = match config.base_theme.as_deref() {
                Some(path) => match fs::read_to_string(path) {
                    Ok(base) => base,
                    Err(error) => {
                        return failed(
                            config,
                            admission,
                            track,
                            format!("read base theme: {error}"),
                        )
                    }
                },
                None => crate::DEFAULT_THEME.to_owned(),
            };
            let record = match cached_or_generate(config, &hash, &bytes, &base) {
                Ok(record) => record,
                Err(error) => return failed(config, admission, track, error),
            };
            let mut envelope = artifact(admission, track, "ok");
            envelope.features.push("selection");
            envelope.roles = Some(record.roles);
            envelope.verified_pairs = Some(record.verified_pairs);
            envelope.clusters = Some(record.clusters);
            let theme = theme_output.map(|path| (path, record.theme.as_str()));
            if publish(config, admission, &envelope, theme)? {
                Ok(PublishResult::Ok(hash))
            } else {
                Ok(PublishResult::Superseded)
            }
        }
        None => {
            let base = match config.base_theme.as_deref() {
                Some(path) => match fs::read_to_string(path) {
                    Ok(base) => base,
                    Err(error) => {
                        return failed(
                            config,
                            admission,
                            track,
                            format!("read base theme: {error}"),
                        )
                    }
                },
                None => crate::DEFAULT_THEME.to_owned(),
            };
            if let Err(error) = ron::de::from_str::<ron::Value>(&base) {
                return failed(
                    config,
                    admission,
                    track,
                    format!("parse base theme: {error}"),
                );
            }
            let mut envelope = artifact(admission, track, "no-art");
            envelope.clusters = Some(Vec::new());
            match no_art_bundle(&base, &config.floors) {
                Ok((roles, pairs)) => {
                    envelope.roles = Some(roles);
                    envelope.verified_pairs = Some(pairs);
                    envelope.features.push("selection");
                }
                Err(reason) => eprintln!("publisher: no-art role bundle absent: {reason}"),
            }
            let theme = theme_output.map(|path| (path, base.as_str()));
            if publish(config, admission, &envelope, theme)? {
                Ok(PublishResult::NoArt)
            } else {
                Ok(PublishResult::Superseded)
            }
        }
    }
}

fn failed(
    config: &EffectiveConfig,
    admission: &Admission,
    track: Track,
    error: String,
) -> Result<PublishResult, String> {
    eprintln!("publisher: {error}");
    let mut envelope = artifact(admission, track, "failed");
    envelope.error = Some(error);
    if publish(config, admission, &envelope, None)? {
        Ok(PublishResult::Failed)
    } else {
        Ok(PublishResult::Superseded)
    }
}

pub fn publish_current(
    config: &EffectiveConfig,
    theme_output: Option<&Path>,
) -> Result<PublishResult, String> {
    let file = match rmpc_current_file() {
        Ok(file) => file,
        Err(error) => {
            let admission = admit_event(config, None)?;
            return failed(config, &admission, blank_track(None), error);
        }
    };
    let admission = admit_event(config, file)?;
    publish_event(config, &admission, theme_output)
}

fn rmpc_current_file() -> Result<Option<String>, String> {
    let executable = std::env::var_os("RMPC_BIN").unwrap_or_else(|| "rmpc".into());
    let output = Command::new(executable)
        .arg("song")
        .output()
        .map_err(|error| format!("run rmpc song: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "rmpc song exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("parse rmpc song JSON: {error}"))?;
    Ok(value
        .get("file")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned))
}

/// Check admission under the same lock used for publication.
pub fn is_current(config: &EffectiveConfig, admission: &Admission) -> Result<bool, String> {
    let _lock = ShortLock::acquire(config)?;
    Ok(read_admission(config)?.as_ref().is_some_and(|current| {
        current.token == admission.token
            && current.sequence == admission.sequence
            && current.file == admission.file
    }))
}

/// Commit a staged generated document only if its event is still admitted.
pub fn commit_theme(
    config: &EffectiveConfig,
    admission: &Admission,
    staged: &Path,
    destination: &Path,
) -> Result<bool, String> {
    let _lock = ShortLock::acquire(config)?;
    if !read_admission(config)?.as_ref().is_some_and(|current| {
        current.token == admission.token
            && current.sequence == admission.sequence
            && current.file == admission.file
    }) {
        return Ok(false);
    }
    fs::create_dir_all(parent(destination)?).map_err(|e| format!("create theme directory: {e}"))?;
    fs::rename(staged, destination).map_err(|e| format!("commit theme: {e}"))?;
    Ok(true)
}

fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

fn rounded(value: f64, digits: i32) -> f64 {
    let factor = 10_f64.powi(digits);
    (value * factor).round() / factor
}

fn lch(rgb: [u8; 3]) -> [f64; 3] {
    let mut value = oklab_to_oklch(rgb8_to_oklab(rgb));
    if value[1] < 1e-4 {
        value[2] = 0.0;
    }
    [value[0] as f64, value[1] as f64, value[2] as f64]
}

fn role_record(
    rgb: [u8; 3],
    origin: Option<String>,
    tier: Option<String>,
    role: Role,
) -> RoleRecord {
    RoleRecord {
        hex: hex(rgb),
        oklch: lch(rgb),
        origin,
        tier,
        kind: if role == Role::Selection {
            "derived"
        } else {
            "base"
        }
        .to_owned(),
        alias_of: (role == Role::Inactive).then(|| "frame".to_owned()),
        derived_from: (role == Role::Selection).then(|| "active".to_owned()),
    }
}

fn pairs(
    colors: &BTreeMap<String, [u8; 3]>,
    floors: &solver::GuardrailFloors,
) -> Result<Vec<VerifiedPair>, String> {
    let mut result = Vec::new();
    for (fg, bg, floor) in [
        ("text", "background", floors.text_background),
        ("accent", "background", floors.accent_background),
        ("active", "background", floors.active_background),
        ("highlight_text", "active", floors.highlight_active),
        ("highlight_text", "selection", floors.highlight_selection),
        ("frame", "background", floors.frame_background),
    ] {
        let a = colors.get(fg).ok_or_else(|| format!("missing {fg}"))?;
        let b = colors.get(bg).ok_or_else(|| format!("missing {bg}"))?;
        let measured = solver::wcag_contrast(*a, *b) as f64;
        if measured + 1e-7 < floor as f64 {
            return Err(format!(
                "{fg}/{bg} measured {measured:.3} below floor {floor:.1}"
            ));
        }
        result.push(VerifiedPair::Contrast {
            fg: fg.to_owned(),
            bg: bg.to_owned(),
            floor: floor as f64,
            measured: rounded(measured, 3),
        });
    }
    let selection = colors.get("selection").ok_or("missing selection")?;
    let active = colors.get("active").ok_or("missing active")?;
    let delta = rgb8_to_oklab(*selection)[0] as f64 - rgb8_to_oklab(*active)[0] as f64;
    if delta + 1e-7 < floors.selection_active_delta_l as f64 {
        return Err(format!(
            "selection/active delta_l {delta:.6} below floor {:.6}",
            floors.selection_active_delta_l
        ));
    }
    result.push(VerifiedPair::DeltaL {
        roles: ["selection".to_owned(), "active".to_owned()],
        floor: floors.selection_active_delta_l as f64,
        measured: rounded(delta, 6),
    });
    Ok(result)
}

fn cache_path(config: &EffectiveConfig, art_hash: &str, base: &str) -> Result<PathBuf, String> {
    let mut hash = Sha256::new();
    hash.update(art_hash.as_bytes());
    hash.update(format!("{config:?}").as_bytes());
    hash.update(base.as_bytes());
    hash.update(REVISION.as_bytes());
    Ok(parent(&config.palette_path)?.join(format!("cache-{:x}.json", hash.finalize())))
}

fn cached_or_generate(
    config: &EffectiveConfig,
    art_hash: &str,
    art: &[u8],
    base: &str,
) -> Result<CacheRecord, String> {
    let path = cache_path(config, art_hash, base)?;
    if let Ok(bytes) = fs::read(&path) {
        if let Ok(record) = serde_json::from_slice::<CacheRecord>(&bytes) {
            eprintln!("publisher: analysis cache hit {art_hash}");
            return Ok(record);
        }
    }
    eprintln!("publisher: analysis cache miss {art_hash}");
    let generated = crate::generate_with_config(art, base, config)?;
    let mut roles = BTreeMap::new();
    let mut colors = BTreeMap::new();
    for role in Role::ALL {
        let assigned = generated.palette.get(role);
        let tier = match assigned.tier {
            SolverTier::Strict => "strict",
            SolverTier::Relaxed => "relaxed",
            SolverTier::Synthesized => "synthesized",
            SolverTier::Neutral => "neutral",
        };
        let key = role.as_str().replace('-', "_");
        colors.insert(key.clone(), assigned.rgb);
        let origin = (role != Role::Inactive).then(|| assigned.origin.clone());
        let tier = (role != Role::Inactive).then(|| tier.to_owned());
        roles.insert(key, role_record(assigned.rgb, origin, tier, role));
    }
    let verified_pairs = pairs(&colors, &config.floors)?;
    let clusters = generated
        .clusters
        .iter()
        .enumerate()
        .map(|(id, cluster)| {
            let rgb = [cluster.rgb.r, cluster.rgb.g, cluster.rgb.b];
            Ok(ClusterRecord {
                id,
                share: cluster.share,
                hex: hex(rgb),
                oklch_extraction: mapped_extraction_lch(cluster)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let record = CacheRecord {
        theme: generated.theme,
        roles,
        verified_pairs,
        clusters,
    };
    atomic_write(
        &path,
        &serde_json::to_vec(&record).map_err(|e| e.to_string())?,
    )?;
    Ok(record)
}

fn mapped_extraction_lch(cluster: &color_core::ClusterOut) -> Result<[f64; 3], String> {
    let (bytes, mapped) = color_core::color::oklab_to_srgb8_gamut_mapped_with_oklab(cluster.oklab);
    let paired = [cluster.rgb.r, cluster.rgb.g, cluster.rgb.b];
    if bytes != paired {
        return Err(format!(
            "mapped cluster bytes diverged from the paired display RGB: {bytes:?} != {paired:?}"
        ));
    }
    Ok(extraction_lch(mapped))
}

fn extraction_lch(mapped: [f32; 3]) -> [f64; 3] {
    let [l, a, b] = mapped.map(f64::from);
    let chroma = a.hypot(b);
    let hue = if chroma < 1e-4 {
        0.0
    } else {
        b.atan2(a).to_degrees().rem_euclid(360.0)
    };
    [l, chroma, hue]
}

fn theme_string_at<'a>(root: &'a ron::Value, path: &str) -> Option<&'a str> {
    let mut value = root;
    for segment in path.split('.') {
        value = match value {
            ron::Value::Map(map) => map.get(&ron::Value::String(segment.to_owned()))?,
            ron::Value::Option(Some(inner)) => inner,
            _ => return None,
        };
    }
    match value {
        ron::Value::String(text) => Some(text),
        ron::Value::Option(Some(inner)) => match inner.as_ref() {
            ron::Value::String(text) => Some(text),
            _ => None,
        },
        _ => None,
    }
}

fn parse_hex(text: &str) -> Option<[u8; 3]> {
    if text.len() != 7 || !text.starts_with('#') {
        return None;
    }
    Some([
        u8::from_str_radix(&text[1..3], 16).ok()?,
        u8::from_str_radix(&text[3..5], 16).ok()?,
        u8::from_str_radix(&text[5..7], 16).ok()?,
    ])
}

fn no_art_bundle(
    base: &str,
    floors: &solver::GuardrailFloors,
) -> Result<(BTreeMap<String, RoleRecord>, Vec<VerifiedPair>), String> {
    let tree: ron::Value = ron::de::from_str(base).map_err(|e| format!("parse base theme: {e}"))?;
    let paths = [
        (Role::Background, "background_color"),
        (Role::Text, "text_color"),
        (Role::Accent, "level_styles.info.fg"),
        (Role::Active, "highlighted_item_style.bg"),
        (Role::HighlightText, "highlighted_item_style.fg"),
        (Role::Selection, "current_item_style.bg"),
        (Role::Frame, "borders_style.fg"),
        (Role::HeaderAccent, "preview_label_style.fg"),
        (Role::Inactive, "borders_style.fg"),
    ];
    let mut colors = BTreeMap::new();
    let mut roles = BTreeMap::new();
    for (role, path) in paths {
        let text = theme_string_at(&tree, path)
            .ok_or_else(|| format!("{path} is not a literal string"))?;
        let rgb =
            parse_hex(text).ok_or_else(|| format!("{path} is terminal-dependent ({text})"))?;
        let key = role.as_str().replace('-', "_");
        colors.insert(key.clone(), rgb);
        roles.insert(
            key,
            role_record(
                rgb,
                (role != Role::Inactive).then(|| "stock-base".to_owned()),
                None,
                role,
            ),
        );
    }
    let verified_pairs = pairs(&colors, floors)?;
    Ok((roles, verified_pairs))
}

trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

struct Mpd {
    reader: BufReader<Box<dyn ReadWrite>>,
}

impl Mpd {
    fn connect() -> Result<Self, String> {
        let host = std::env::var("MPD_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
        let port = std::env::var("MPD_PORT").unwrap_or_else(|_| "6600".to_owned());
        let (password, host) = host
            .rsplit_once('@')
            .map_or((None, host.as_str()), |(pw, host)| (Some(pw), host));
        let stream: Box<dyn ReadWrite> = if host.starts_with('/') {
            let socket =
                UnixStream::connect(host).map_err(|e| format!("connect MPD socket: {e}"))?;
            socket
                .set_read_timeout(Some(Duration::from_secs(8)))
                .map_err(|e| e.to_string())?;
            socket
                .set_write_timeout(Some(Duration::from_secs(8)))
                .map_err(|e| e.to_string())?;
            Box::new(socket)
        } else {
            let port = port
                .parse::<u16>()
                .map_err(|e| format!("invalid MPD_PORT: {e}"))?;
            let addr = (host, port)
                .to_socket_addrs()
                .map_err(|e| format!("resolve MPD: {e}"))?
                .next()
                .ok_or("MPD address resolved to nothing")?;
            let socket = TcpStream::connect_timeout(&addr, Duration::from_secs(8))
                .map_err(|e| format!("connect MPD: {e}"))?;
            socket
                .set_read_timeout(Some(Duration::from_secs(8)))
                .map_err(|e| e.to_string())?;
            socket
                .set_write_timeout(Some(Duration::from_secs(8)))
                .map_err(|e| e.to_string())?;
            Box::new(socket)
        };
        let mut mpd = Self {
            reader: BufReader::new(stream),
        };
        let greeting = mpd.line()?;
        if !greeting.starts_with("OK MPD ") {
            return Err(format!("unexpected MPD greeting: {greeting}"));
        }
        if let Some(password) = password {
            mpd.command(&format!("password {}", quote(password)))?;
            mpd.fields()?;
        }
        Ok(mpd)
    }

    fn line(&mut self) -> Result<String, String> {
        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .map_err(|e| format!("read MPD: {e}"))?;
        if line.is_empty() {
            return Err("MPD connection closed".to_owned());
        }
        Ok(line.trim_end_matches(['\r', '\n']).to_owned())
    }

    fn command(&mut self, command: &str) -> Result<(), String> {
        self.reader
            .get_mut()
            .write_all(format!("{command}\n").as_bytes())
            .map_err(|e| format!("write MPD: {e}"))?;
        self.reader
            .get_mut()
            .flush()
            .map_err(|e| format!("flush MPD: {e}"))
    }

    fn fields(&mut self) -> Result<BTreeMap<String, String>, String> {
        let mut fields = BTreeMap::new();
        loop {
            let line = self.line()?;
            if line == "OK" {
                return Ok(fields);
            }
            if line.starts_with("ACK ") {
                return Err(line);
            }
            if let Some((key, value)) = line.split_once(": ") {
                fields
                    .entry(key.to_owned())
                    .or_insert_with(|| value.to_owned());
            }
        }
    }

    fn track_info(&mut self, file: &str) -> Result<BTreeMap<String, String>, String> {
        self.command(&format!("lsinfo {}", quote(file)))?;
        let fields = self.fields()?;
        if fields.get("file").map(String::as_str) != Some(file) {
            return Err("MPD metadata did not match admitted FILE".to_owned());
        }
        Ok(fields)
    }

    fn picture(&mut self, verb: &str, file: &str) -> Result<Option<Vec<u8>>, String> {
        let mut output = Vec::new();
        let mut expected = None;
        loop {
            self.command(&format!("{verb} {} {}", quote(file), output.len()))?;
            let mut count = 0;
            loop {
                let line = self.line()?;
                if line == "OK" {
                    break;
                }
                if line.starts_with("ACK ") {
                    // MPD's albumart handler uses NO_EXIST/50 with these exact
                    // messages for missing directory art. Other code-50 errors
                    // (e.g. "No database" or unknown stream size) are failures.
                    if output.is_empty() && album_art_absent_ack(verb, &line) {
                        return Ok(None);
                    }
                    return Err(format!("MPD {verb}: {line}"));
                }
                if let Some(value) = line.strip_prefix("size: ") {
                    let size = value
                        .parse::<usize>()
                        .map_err(|e| format!("MPD size: {e}"))?;
                    if size > MAX_ART_BYTES {
                        return Err(format!("MPD art exceeds {MAX_ART_BYTES} bytes"));
                    }
                    expected = Some(size);
                }
                if let Some(value) = line.strip_prefix("binary: ") {
                    count = value
                        .parse::<usize>()
                        .map_err(|e| format!("MPD binary size: {e}"))?;
                    if output.len().saturating_add(count) > MAX_ART_BYTES {
                        return Err("MPD art chunk exceeds size cap".to_owned());
                    }
                    let old = output.len();
                    output.resize(old + count, 0);
                    self.reader
                        .read_exact(&mut output[old..])
                        .map_err(|e| format!("read MPD art: {e}"))?;
                    let mut newline = [0];
                    self.reader
                        .read_exact(&mut newline)
                        .map_err(|e| format!("read MPD art delimiter: {e}"))?;
                    if newline[0] != b'\n' {
                        return Err("MPD art missing newline delimiter".to_owned());
                    }
                }
            }
            let Some(size) = expected else {
                return Ok(None);
            };
            if size == 0 {
                return Ok(None);
            }
            if output.len() >= size {
                return Ok(Some(output));
            }
            if count == 0 {
                return Err("MPD art transfer made no progress".to_owned());
            }
        }
    }

    fn art(&mut self, file: &str) -> Result<Option<Vec<u8>>, String> {
        match self.picture("albumart", file)? {
            Some(bytes) => Ok(Some(bytes)),
            None => self.picture("readpicture", file),
        }
    }
}

fn album_art_absent_ack(verb: &str, line: &str) -> bool {
    if verb != "albumart" || !line.starts_with("ACK [50@") {
        return false;
    }
    line.split_once("] {albumart} ")
        .is_some_and(|(_, message)| message == "No file exists" || message == "No art file exists")
}

fn quote(value: &str) -> String {
    let mut quoted = String::from("\"");
    for c in value.chars() {
        match c {
            '"' | '\\' => {
                quoted.push('\\');
                quoted.push(c);
            }
            '\n' | '\r' => quoted.push(' '),
            _ => quoted.push(c),
        }
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_core::{ClusterOut, RgbValue};

    fn cluster(lab: [f32; 3], rgb: [u8; 3]) -> ClusterOut {
        ClusterOut {
            count: 1,
            share: 1.0,
            centroid_space: lab,
            oklab: lab,
            oklch: color_core::color::oklab_to_oklch(lab),
            rgb: RgbValue {
                r: rgb[0],
                g: rgb[1],
                b: rgb[2],
            },
            hsv: [0.0; 3],
        }
    }

    #[test]
    fn mapped_extraction_coordinates_survive_artifact_json() {
        let samples = [
            [0.7189761, -0.17126995, 0.1321782],
            [0.9053503, -0.11228561, 0.18462622],
            [0.65, 0.4, 0.2],
            [0.5, 0.0, 0.0],
        ];
        for lab in samples {
            let (bytes, mapped) = color_core::color::oklab_to_srgb8_gamut_mapped_with_oklab(lab);
            let record = ClusterRecord {
                id: 0,
                share: 1.0,
                hex: hex(bytes),
                oklch_extraction: mapped_extraction_lch(&cluster(lab, bytes)).unwrap(),
            };
            let serialized = serde_json::to_vec(&record).unwrap();
            let decoded: ClusterRecord = serde_json::from_slice(&serialized).unwrap();
            let [l, c, h] = decoded.oklch_extraction;
            let back = [l, c * h.to_radians().cos(), c * h.to_radians().sin()];
            for (actual, expected) in back.into_iter().zip(mapped.map(f64::from)) {
                assert!(
                    (actual - expected).abs() < 1e-6,
                    "serialized extraction changed mapped coordinate for {lab:?}"
                );
            }
            assert_eq!(decoded.hex, hex(bytes));
        }
    }

    #[test]
    fn extraction_hue_is_zero_below_achromatic_threshold() {
        assert_eq!(extraction_lch([0.5, 0.00005, 0.0])[2], 0.0);
        assert_ne!(extraction_lch([0.5, 0.0, 0.00011])[2], 0.0);
    }

    #[test]
    fn astra_vectors_document_f32_chart_byte_hazard() {
        for (lab, paired) in [
            ([0.7189761, -0.17126995, 0.1321782], [58, 196, 50]),
            ([0.9053503, -0.11228561, 0.18462622], [203, 244, 15]),
        ] {
            let (bytes, mapped) = color_core::color::oklab_to_srgb8_gamut_mapped_with_oklab(lab);
            assert_eq!(bytes, paired);
            let rounded_chart = color_core::color::oklab_to_oklch(mapped);
            let reexpressed = color_core::color::oklch_to_oklab(rounded_chart);
            let reprojected = color_core::color::oklab_to_srgb8_gamut_mapped(reexpressed);
            assert_ne!(reprojected, bytes);
        }
    }
}
