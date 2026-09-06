use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct Sandbox(PathBuf);

impl Sandbox {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "rmpc-auto-theme-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self, child: &str) -> PathBuf {
        self.0.join(child)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn executable(path: &Path, source: &str) {
    fs::write(path, source).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn hook_command(sandbox: &Sandbox, album_rc: i32) -> Command {
    let fake_rmpc = sandbox.path("rmpc");
    let fake_gen = sandbox.path("gen");
    executable(
        &fake_rmpc,
        r#"#!/bin/sh
if [ "$1" = albumart ]; then
  [ "${ALBUM_RC:-0}" -eq 0 ] && cp "$FAKE_ART" "$3"
  exit "${ALBUM_RC:-0}"
fi
if [ "$1" = remote ]; then cat >"$APPLIED"; printf 'apply\n' >>"$CALLS"; exit "${REMOTE_RC:-0}"; fi
exit 9
"#,
    );
    executable(
        &fake_gen,
        r#"#!/bin/sh
case ${1:-} in
  --print-config) printf '%s\n' "${OSC11:-false}"; exit 0 ;;
  --print-base-theme) cat "$BASE_THEME"; exit 0 ;;
esac
printf 'generate\n' >>"$GEN_CALLS"
[ "${GEN_FAIL:-0}" -eq 0 ] || exit 7
[ "${GEN_DELAY:-0}" -eq 0 ] || sleep "$GEN_DELAY"
cat "$GENERATED_THEME"
"#,
    );
    let home = sandbox.path("home");
    let config = sandbox.path("config");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&config).unwrap();
    let mut command = Command::new(format!(
        "{}/hook/on_song_change.sh",
        env!("CARGO_MANIFEST_DIR")
    ));
    command
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", config)
        .env("RMPC_BIN", fake_rmpc)
        .env("RMPC_THEME_GEN", fake_gen)
        .env("FAKE_ART", sandbox.path("art"))
        .env("GENERATED_THEME", sandbox.path("generated.ron"))
        .env("BASE_THEME", sandbox.path("base.ron"))
        .env("APPLIED", sandbox.path("applied.ron"))
        .env("CALLS", sandbox.path("calls"))
        .env("GEN_CALLS", sandbox.path("gen-calls"))
        .env("ALBUM_RC", album_rc.to_string())
        .env("PID", "123");
    command
}

fn seed_themes(sandbox: &Sandbox) {
    fs::write(sandbox.path("art"), b"same album art").unwrap();
    fs::write(
        sandbox.path("generated.ron"),
        b"(background_color: \"#112233\")\n",
    )
    .unwrap();
    fs::write(
        sandbox.path("base.ron"),
        b"(background_color: \"#010203\")\n",
    )
    .unwrap();
}

#[test]
fn hook_short_circuits_same_art_and_applies_atomically() {
    let sandbox = Sandbox::new("same-art");
    seed_themes(&sandbox);
    assert!(hook_command(&sandbox, 0).status().unwrap().success());
    assert!(hook_command(&sandbox, 0).status().unwrap().success());
    assert_eq!(
        fs::read_to_string(sandbox.path("gen-calls")).unwrap(),
        "generate\n"
    );
    assert_eq!(
        fs::read_to_string(sandbox.path("config/rmpc/themes/auto-theme.ron")).unwrap(),
        "(background_color: \"#112233\")\n"
    );
    assert_eq!(
        fs::read_to_string(sandbox.path("calls")).unwrap(),
        "apply\n"
    );
}

#[test]
fn hook_resets_to_base_on_exit_two_and_clears_hash() {
    let sandbox = Sandbox::new("no-art");
    seed_themes(&sandbox);
    let state = sandbox.path("config/rmpc-auto-theme");
    fs::create_dir_all(&state).unwrap();
    fs::write(state.join("art.sha256"), "old").unwrap();
    assert!(hook_command(&sandbox, 2).status().unwrap().success());
    assert!(!state.join("art.sha256").exists());
    assert_eq!(
        fs::read(sandbox.path("applied.ron")).unwrap(),
        fs::read(sandbox.path("base.ron")).unwrap()
    );
}

#[test]
fn hook_swallows_generator_failure_without_touching_theme() {
    let sandbox = Sandbox::new("generator-failure");
    seed_themes(&sandbox);
    let theme = sandbox.path("config/rmpc/themes/auto-theme.ron");
    fs::create_dir_all(theme.parent().unwrap()).unwrap();
    fs::write(&theme, "old theme\n").unwrap();
    let status = hook_command(&sandbox, 0)
        .env("GEN_FAIL", "1")
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(fs::read_to_string(theme).unwrap(), "old theme\n");
    assert!(!sandbox.path("calls").exists());
}

#[test]
fn hook_commits_no_hash_when_ipc_apply_fails() {
    let sandbox = Sandbox::new("ipc-failure");
    seed_themes(&sandbox);
    let status = hook_command(&sandbox, 0)
        .env("REMOTE_RC", "1")
        .status()
        .unwrap();
    assert!(status.success());
    assert!(!sandbox.path("config/rmpc-auto-theme/art.sha256").exists());
}

#[test]
fn hook_writes_osc11_only_when_enabled_and_tty_is_writable() {
    let sandbox = Sandbox::new("osc11");
    seed_themes(&sandbox);
    let tty = sandbox.path("tty");
    fs::write(&tty, "").unwrap();
    assert!(hook_command(&sandbox, 0)
        .env("OSC11", "true")
        .env("RMPC_AUTO_THEME_TTY", &tty)
        .status()
        .unwrap()
        .success());
    assert_eq!(fs::read(tty).unwrap(), b"\x1b]11;#112233\x07");
}

#[test]
fn hook_caps_its_single_log_file() {
    let sandbox = Sandbox::new("log-cap");
    seed_themes(&sandbox);
    let state = sandbox.path("config/rmpc-auto-theme");
    fs::create_dir_all(&state).unwrap();
    fs::write(state.join("hook.log"), vec![b'x'; 300_000]).unwrap();
    assert!(hook_command(&sandbox, 3).status().unwrap().success());
    assert!(fs::metadata(state.join("hook.log")).unwrap().len() <= 132_000);
}

#[test]
fn hook_only_publishes_the_latest_overlapping_event() {
    let sandbox = Sandbox::new("overlap");
    seed_themes(&sandbox);
    let mut first = hook_command(&sandbox, 0);
    first.env("GEN_DELAY", "0.3");
    let mut child = first.spawn().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(80));
    fs::write(sandbox.path("art"), b"new art").unwrap();
    fs::write(
        sandbox.path("generated.ron"),
        b"(background_color: \"#abcdef\")\n",
    )
    .unwrap();
    assert!(hook_command(&sandbox, 0).status().unwrap().success());
    assert!(child.wait().unwrap().success());
    assert_eq!(
        fs::read_to_string(sandbox.path("config/rmpc/themes/auto-theme.ron")).unwrap(),
        "(background_color: \"#abcdef\")\n"
    );
    assert_eq!(
        fs::read_to_string(sandbox.path("calls")).unwrap(),
        "apply\n"
    );
}

fn install_command(sandbox: &Sandbox, wire: bool) -> Command {
    let fake = sandbox.path("source-bin");
    executable(
        &fake,
        r##"#!/bin/sh
case ${1:-} in
  --print-config) printf '\n' ;;
  --print-base-theme) printf '(background_color: "#010203")\n' ;;
esac
"##,
    );
    let home = sandbox.path("home");
    let config = sandbox.path("config");
    let prefix = sandbox.path("prefix");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&config).unwrap();
    let mut command = Command::new(format!("{}/install.sh", env!("CARGO_MANIFEST_DIR")));
    if wire {
        command.arg("--wire-hook");
    }
    command
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", config)
        .env("RMPC_AUTO_THEME_PREFIX", prefix)
        .env("RMPC_AUTO_THEME_SKIP_BUILD", "1")
        .env("RMPC_THEME_GEN_SOURCE", fake);
    command
}

#[test]
fn installer_requires_consent_then_wires_once_with_backup() {
    let sandbox = Sandbox::new("installer");
    let live = sandbox.path("config/rmpc/config.ron");
    let themes = sandbox.path("config/rmpc/themes");
    fs::create_dir_all(&themes).unwrap();
    fs::write(themes.join("base.ron"), "(background_color: \"#999999\")\n").unwrap();
    let original = "(\n    theme: \"base\",\n    on_song_change: None,\n)\n";
    fs::write(&live, original).unwrap();

    assert!(install_command(&sandbox, false).status().unwrap().success());
    assert_eq!(fs::read_to_string(&live).unwrap(), original);
    assert!(install_command(&sandbox, true).status().unwrap().success());
    let wired = fs::read_to_string(&live).unwrap();
    assert!(wired.contains("theme: \"auto-theme\""));
    assert!(wired.contains("on_song_change: [\""));
    let backups = fs::read_dir(live.parent().unwrap())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .contains("config.ron.backup.")
        })
        .count();
    assert_eq!(backups, 1);
    assert!(install_command(&sandbox, true).status().unwrap().success());
    let backups_after = fs::read_dir(live.parent().unwrap())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .contains("config.ron.backup.")
        })
        .count();
    assert_eq!(backups_after, 1);
}

#[test]
fn installer_refuses_an_existing_non_none_hook() {
    let sandbox = Sandbox::new("installer-refuse");
    let live = sandbox.path("config/rmpc/config.ron");
    fs::create_dir_all(live.parent().unwrap()).unwrap();
    let original = "(\n  theme: \"base\",\n  on_song_change: [\"other\"],\n)\n";
    fs::write(&live, original).unwrap();
    let status = install_command(&sandbox, true).status().unwrap();
    assert!(!status.success());
    assert_eq!(fs::read_to_string(live).unwrap(), original);
}
