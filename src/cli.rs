use crate::core::{injector, process_scanner};
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

const USAGE: &str = "\
usage: fluffy_injector [options] <pid|process> <dll>...
  -c, --copy     inject a temp copy so the original can be rebuilt
  -r, --random   randomize the copy name (implies --copy)
";

pub(crate) fn run() -> Option<i32> {
    let mut args = std::env::args().skip(1);
    Some(execute(args.next()?, args))
}

fn execute(first: String, rest: impl Iterator<Item = String>) -> i32 {
    if matches!(first.as_str(), "-h" | "--help") {
        print!("{USAGE}");
        return 0;
    }
    let (copy, random, target, dlls) = match parse_args(std::iter::once(first).chain(rest)) {
        Ok(req) => req,
        Err(error) => return usage_error(&error),
    };
    let pid = match resolve_pid(&target) {
        Ok(pid) => pid,
        Err(error) => return usage_error(&format!("{error:#}")),
    };
    if let Err(error) = dlls
        .iter()
        .try_for_each(|dll| inject_one(pid, &target, dll, copy, random))
    {
        eprintln!("error: {error:#}");
        return 1;
    }
    0
}

fn parse_args(
    mut args: impl Iterator<Item = String>,
) -> Result<(bool, bool, String, Vec<PathBuf>), String> {
    let mut copy = false;
    let mut random = false;
    let mut pos = Vec::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-c" | "--copy" => copy = true,
            "-r" | "--random" => random = true,
            "-n" | "--name" | "-p" | "--pid" => pos.push(flag_value(&arg, &mut args)?),
            flag if flag.starts_with('-') => return Err(format!("unknown option: {flag}")),
            _ => pos.push(arg),
        }
    }
    let mut pos = pos.into_iter();
    let target = pos.next().ok_or_else(|| "missing process".to_string())?;
    let dlls: Vec<PathBuf> = pos.map(PathBuf::from).collect();
    if dlls.is_empty() {
        return Err("missing DLL path".into());
    }
    Ok((copy || random, random, target, dlls))
}

fn flag_value(flag: &str, args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    args.next()
        .filter(|value| !value.starts_with('-'))
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn usage_error(error: &str) -> i32 {
    eprint!("error: {error}\n{USAGE}");
    2
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

fn inject_one(pid: u32, label: &str, dll: &Path, copy: bool, random: bool) -> Result<()> {
    if !dll.is_file() {
        bail!("DLL not found: {}", dll.display());
    }
    let path = dll
        .canonicalize()
        .with_context(|| format!("invalid DLL path: {}", dll.display()))?;
    injector::inject_dll(pid, &path, copy, random)?;
    println!("injected {} into {label} ({pid})", path.display());
    Ok(())
}
