//! rmpc-theme-gen: album art on stdin, complete rmpc theme on stdout.

use std::io::{Read, Write};
use std::path::PathBuf;

struct Args {
    debug: bool,
    base_theme: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let mut debug = false;
    let mut base_theme = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(argument) = args.next() {
        if argument == "--debug" {
            debug = true;
        } else if argument == "--base-theme" {
            let path = args
                .next()
                .ok_or_else(|| "--base-theme requires a path".to_string())?;
            base_theme = Some(PathBuf::from(path));
        } else if argument == "--help" || argument == "-h" {
            eprintln!("Usage: rmpc-theme-gen [--debug] [--base-theme PATH] < ARTWORK");
            std::process::exit(0);
        } else {
            return Err(format!("unknown argument: {}", argument.to_string_lossy()));
        }
    }
    Ok(Args { debug, base_theme })
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let base_theme = match args.base_theme {
        Some(path) => std::fs::read_to_string(&path)
            .map_err(|error| format!("read base theme {}: {error}", path.display()))?,
        None => rmpc_theme_gen::DEFAULT_THEME.to_string(),
    };
    let mut artwork = Vec::new();
    std::io::stdin()
        .read_to_end(&mut artwork)
        .map_err(|error| format!("read stdin: {error}"))?;

    let generated = rmpc_theme_gen::generate(&artwork, &base_theme)?;
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
