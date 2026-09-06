//! rmpc-theme-gen: album art on stdin, complete rmpc theme on stdout.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use rmpc_theme_gen::config::EffectiveConfig;

struct Args {
    debug: bool,
    config: Option<PathBuf>,
    base_theme: Option<PathBuf>,
    quality: Option<i8>,
    osc11: Option<bool>,
    overrides: Vec<String>,
    print_config: Option<String>,
    print_base_theme: bool,
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
    Ok(parsed)
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
    if let Some(path) = args.base_theme {
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

    if let Some(key) = args.print_config {
        println!("{}", config.print_value(&key)?);
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
