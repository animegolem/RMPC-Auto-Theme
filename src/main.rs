//! rmpc-theme-gen: album art on stdin, complete rmpc theme on stdout.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use rmpc_theme_gen::config::EffectiveConfig;
use rmpc_theme_gen::publisher::{self, Admission};

#[derive(Clone, Copy, PartialEq, Eq)]
enum PublisherMode {
    Admit,
    PublishEvent,
    PublishCurrent,
    IsCurrent,
    CommitTheme,
}

struct Args {
    debug: bool,
    config: Option<PathBuf>,
    base_theme: Option<PathBuf>,
    quality: Option<i8>,
    osc11: Option<bool>,
    overrides: Vec<String>,
    print_config: Option<String>,
    print_base_theme: bool,
    publisher_mode: Option<PublisherMode>,
    file: Option<String>,
    token: Option<String>,
    sequence: Option<u64>,
    theme_output: Option<PathBuf>,
    staged_theme: Option<PathBuf>,
    theme: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let mut parsed = Args {
        debug: false,
        config: None,
        base_theme: None,
        quality: None,
        osc11: None,
        overrides: Vec::new(),
        print_config: None,
        print_base_theme: false,
        publisher_mode: None,
        file: None,
        token: None,
        sequence: None,
        theme_output: None,
        staged_theme: None,
        theme: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--debug" => parsed.debug = true,
            "--config" => parsed.config = Some(required_path(&mut args, "--config")?),
            "--base-theme" => parsed.base_theme = Some(required_path(&mut args, "--base-theme")?),
            "--quality" => {
                let value = required(&mut args, "--quality")?;
                parsed.quality = Some(
                    value
                        .parse()
                        .map_err(|_| format!("invalid --quality value: {value}"))?,
                );
            }
            "--osc11" => parsed.osc11 = Some(true),
            "--no-osc11" => parsed.osc11 = Some(false),
            "--set" => parsed.overrides.push(required(&mut args, "--set")?),
            "--print-config" => parsed.print_config = Some(required(&mut args, "--print-config")?),
            "--print-base-theme" => parsed.print_base_theme = true,
            "--admit-event" => set_mode(&mut parsed.publisher_mode, PublisherMode::Admit)?,
            "--publish-event" => set_mode(&mut parsed.publisher_mode, PublisherMode::PublishEvent)?,
            "--publish-current" => {
                set_mode(&mut parsed.publisher_mode, PublisherMode::PublishCurrent)?
            }
            "--is-current" => set_mode(&mut parsed.publisher_mode, PublisherMode::IsCurrent)?,
            "--commit-theme" => set_mode(&mut parsed.publisher_mode, PublisherMode::CommitTheme)?,
            "--file" => parsed.file = Some(required(&mut args, "--file")?),
            "--token" => parsed.token = Some(required(&mut args, "--token")?),
            "--sequence" => {
                parsed.sequence = Some(
                    required(&mut args, "--sequence")?
                        .parse()
                        .map_err(|_| "--sequence must be a positive integer".to_owned())?,
                );
            }
            "--theme-output" => {
                parsed.theme_output = Some(required_path(&mut args, "--theme-output")?)
            }
            "--staged-theme" => {
                parsed.staged_theme = Some(required_path(&mut args, "--staged-theme")?)
            }
            "--theme" => parsed.theme = Some(required_path(&mut args, "--theme")?),
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }
    if parsed.print_config.is_some() && parsed.print_base_theme {
        return Err("--print-config and --print-base-theme are mutually exclusive".into());
    }
    if parsed.publisher_mode.is_some() && (parsed.print_config.is_some() || parsed.print_base_theme)
    {
        return Err("publisher mode cannot be combined with print modes".into());
    }
    Ok(parsed)
}

fn set_mode(slot: &mut Option<PublisherMode>, mode: PublisherMode) -> Result<(), String> {
    if slot.replace(mode).is_some() {
        return Err("publisher modes are mutually exclusive".to_owned());
    }
    Ok(())
}

fn admission(args: &Args) -> Result<Admission, String> {
    Ok(Admission {
        file: Some(args.file.clone().ok_or("--file is required")?).filter(|file| !file.is_empty()),
        token: args.token.clone().ok_or("--token is required")?,
        sequence: args.sequence.ok_or("--sequence is required")?,
    })
}

fn required(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn required_path(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<PathBuf, String> {
    required(args, flag).map(PathBuf::from)
}

fn print_help() {
    eprintln!(
        "Usage: rmpc-theme-gen [OPTIONS] < ARTWORK\n\
         \nOptions:\n\
         \x20 --config PATH            Read PATH instead of the default config\n\
         \x20 --base-theme PATH        Override the base theme path\n\
         \x20 --quality 0..4           Override extraction quality\n\
         \x20 --osc11 | --no-osc11    Override terminal sync for the hook\n\
         \x20 --set KEY=VALUE          Override any documented scalar key\n\
         \x20 --print-config KEY       Print one effective scalar and exit\n\
         \x20 --print-base-theme       Print the effective base document and exit\n\
         \x20 --admit-event --file FILE  Admit a file event; print TOKEN SEQUENCE\n\
         \x20 --publish-event --file FILE --token TOKEN --sequence N [--theme-output PATH]\n\
         \x20 --publish-current        Publish current MPD song through admission path\n\
         \x20 --commit-theme ...       Commit a staged theme iff event is current\n\
         \x20 --debug                  Print provenance to stderr"
    );
}

fn load_base_theme(path: Option<&Path>) -> Result<String, String> {
    match path {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|error| format!("read base theme {}: {error}", path.display())),
        None => Ok(rmpc_theme_gen::DEFAULT_THEME.to_string()),
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let mut config = EffectiveConfig::load(args.config.as_deref())?;
    if let Some(path) = args.base_theme.clone() {
        config.base_theme = Some(path);
    }
    if let Some(quality) = args.quality {
        config.quality = quality;
    }
    if let Some(osc11) = args.osc11 {
        config.osc11 = osc11;
    }
    for assignment in &args.overrides {
        config.apply_override(assignment)?;
    }
    config.validate()?;

    if let Some(key) = args.print_config.as_deref() {
        println!("{}", config.print_value(key)?);
        return Ok(());
    }
    if let Some(mode) = args.publisher_mode {
        match mode {
            PublisherMode::Admit => {
                let file = args.file.ok_or("--admit-event requires --file")?;
                let accepted = publisher::admit_event(&config, Some(file))?;
                println!("{} {}", accepted.token, accepted.sequence);
            }
            PublisherMode::PublishEvent => {
                let accepted = admission(&args)?;
                let outcome =
                    publisher::publish_event(&config, &accepted, args.theme_output.as_deref())?;
                println!("{}", outcome.line());
            }
            PublisherMode::PublishCurrent => {
                let outcome = publisher::publish_current(&config, args.theme_output.as_deref())?;
                println!("{}", outcome.line());
            }
            PublisherMode::IsCurrent => {
                if !publisher::is_current(&config, &admission(&args)?)? {
                    std::process::exit(1);
                }
            }
            PublisherMode::CommitTheme => {
                let accepted = admission(&args)?;
                let staged = args
                    .staged_theme
                    .as_deref()
                    .ok_or("--staged-theme is required")?;
                let theme = args.theme.as_deref().ok_or("--theme is required")?;
                let committed = publisher::commit_theme(&config, &accepted, staged, theme)?;
                println!("{}", if committed { "committed" } else { "superseded" });
            }
        }
        return Ok(());
    }
    let base_theme = load_base_theme(config.base_theme.as_deref())?;
    if args.print_base_theme {
        print!("{base_theme}");
        return Ok(());
    }
    let mut artwork = Vec::new();
    std::io::stdin()
        .read_to_end(&mut artwork)
        .map_err(|error| format!("read stdin: {error}"))?;

    let generated = rmpc_theme_gen::generate_with_config(&artwork, &base_theme, &config)?;
    std::io::stdout()
        .write_all(generated.theme.as_bytes())
        .map_err(|error| format!("write stdout: {error}"))?;
    if args.debug {
        eprint!("{}", generated.debug);
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("rmpc-theme-gen: {error}");
        std::process::exit(1);
    }
}
