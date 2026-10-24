use std::io::{self, Write};
use std::process::Stdio;

use anyhow::{Context, Result, bail};
use regex_lite::Regex;
use serde_json::{Value, json};
use tokio::process::Command;

pub async fn run() -> Result<()> {
    let kernel = nix::sys::utsname::uname()?
        .release()
        .to_string_lossy()
        .into_owned();
    let (packages, compositor, nvim, node, python, rust) = tokio::join!(
        output("pacman", &["-Q"]),
        output("hyprctl", &["version", "-j"]),
        version("nvim"),
        version("node"),
        version("python3"),
        version("rustc"),
    );
    let compositor: Value = serde_json::from_slice(&compositor?)?;
    let snapshot = json!({
        "kernel": kernel,
        "packageCount": String::from_utf8(packages?)?.lines().count(),
        "wmVersion": compositor["version"],
        "nvimVersion": nvim?,
        "nodeVersion": node?,
        "pythonVersion": python?,
        "rustVersion": rust?,
    });
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &snapshot)?;
    writeln!(stdout)?;
    Ok(())
}

async fn output(program: &str, arguments: &[&str]) -> Result<Vec<u8>> {
    let result = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .output()
        .await
        .with_context(|| format!("run {program}"))?;
    if !result.status.success() {
        bail!("{program} exited with {}", result.status);
    }
    Ok(result.stdout)
}

async fn version(program: &str) -> Result<String> {
    match output(program, &["--version"]).await {
        Ok(bytes) => {
            let text = String::from_utf8(bytes)?;
            let pattern = Regex::new(r"[0-9]+(\.[0-9]+){1,2}")?;
            Ok(pattern
                .find(&text)
                .map_or("", |matched| matched.as_str())
                .to_owned())
        }
        Err(error)
            if error
                .downcast_ref::<io::Error>()
                .is_some_and(|error| error.kind() == io::ErrorKind::NotFound) =>
        {
            Ok(String::new())
        }
        Err(error) => Err(error),
    }
}
