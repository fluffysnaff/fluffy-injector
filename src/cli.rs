use crate::core::{injector, process_scanner};
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

const USAGE: &str = "usage: fluffy_injector [<pid|process> <dll>...]\n";

pub(crate) fn run() -> Option<i32> {
    let mut args = std::env::args().skip(1);
    Some(execute(args.next()?, &mut args))
}

fn execute(first: String, rest: &mut impl Iterator<Item = String>) -> i32 {
    if matches!(first.as_str(), "-h" | "--help") {
        print!("{USAGE}");
        return 0;
    }
    let target = match parse_target(&first, rest) {
        Ok(target) => target,
        Err(error) => return usage_error(&error),
    };
    let mut dlls = rest.map(PathBuf::from).peekable();
    if dlls.peek().is_none() {
        return usage_error("missing DLL path");
    }
    let pid = match resolve_pid(&target) {
        Ok(pid) => pid,
        Err(error) => return usage_error(&format!("{error:#}")),
    };
    match dlls.try_for_each(|dll| inject_one(pid, &target, &dll)) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("error: {error:#}");
            1
        }
    }
}

fn usage_error(error: &str) -> i32 {
    eprint!("error: {error}\n{USAGE}");
    2
}

fn parse_target(first: &str, rest: &mut impl Iterator<Item = String>) -> Result<String, String> {
    match first {
        "-n" | "--name" | "-p" | "--pid" => rest
            .next()
            .filter(|value| !value.starts_with('-'))
            .ok_or_else(|| format!("{first} requires a value")),
        flag if flag.starts_with('-') => Err(format!("unknown option: {flag}")),
        other => Ok(other.to_owned()),
    }
}

fn resolve_pid(target: &str) -> Result<u32> {
    if let Ok(pid) = target.parse() {
        return Ok(pid);
    }
    let pids = process_scanner::pids_named(target).context("Failed to scan processes")?;
    match pids.as_slice() {
        [] => bail!("process not found: {target}"),
        [pid] => Ok(*pid),
        many => bail!(
            "multiple processes named {target}: {}\nuse a pid to choose one",
            many.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn inject_one(pid: u32, label: &str, dll: &Path) -> Result<()> {
    if !dll.is_file() {
        bail!("DLL not found: {}", dll.display());
    }
    let path = dll
        .canonicalize()
        .with_context(|| format!("invalid DLL path: {}", dll.display()))?;
    injector::inject_dll(pid, &path, false, false)?;
    println!("injected {} into {label} ({pid})", path.display());
    Ok(())
}
