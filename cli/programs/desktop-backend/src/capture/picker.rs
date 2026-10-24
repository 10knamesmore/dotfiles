use std::{env, io::Write, time::Duration};

use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

use super::common::{log, socket_path};

fn parse_windows(value: &str) -> Result<Vec<Value>> {
    value
        .split("[HA>]")
        .filter(|item| !item.is_empty())
        .map(|item| {
            let (token, rest) = item
                .split_once("[HC>]")
                .ok_or_else(|| anyhow!("invalid window token"))?;
            let (app_id, rest) = rest
                .split_once("[HT>]")
                .ok_or_else(|| anyhow!("invalid window class"))?;
            let (title, address) = rest
                .split_once("[HE>]")
                .ok_or_else(|| anyhow!("invalid window address"))?;
            Ok(json!({
                "id": token.parse::<u64>()?.to_string(),
                "address": format!("0x{:x}", address.parse::<u64>()?),
                "title": title,
                "appId": app_id,
            }))
        })
        .collect()
}

/// XDPH's allow-token flag never bypasses this request's explicit confirmation.
pub async fn pick(_allow_token: bool) -> Result<bool> {
    let result = exchange().await;
    match result {
        Ok(Some(selection)) => {
            let mut stdout = std::io::stdout().lock();
            if writeln!(stdout, "{selection}")
                .and_then(|()| stdout.flush())
                .is_err()
            {
                return Ok(false);
            }
            Ok(true)
        }
        Ok(None) => Ok(false),
        Err(error) => {
            log(format_args!("picker could not complete: {error:#}"));
            Ok(false)
        }
    }
}

async fn exchange() -> Result<Option<String>> {
    let windows = parse_windows(&env::var("XDPH_WINDOW_SHARING_LIST").unwrap_or_default())?;
    let mut connection = tokio::time::timeout(
        Duration::from_millis(500),
        UnixStream::connect(socket_path()?),
    )
    .await??;
    let mut request = serde_json::to_vec(&json!({"type":"picker", "windows":windows}))?;
    request.push(b'\n');
    tokio::time::timeout(Duration::from_millis(500), connection.write_all(&request)).await??;
    let mut reader = BufReader::new(connection);
    let mut response = String::new();
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    tokio::select! {
        result = reader.read_line(&mut response) => {
            if result? == 0 { return Ok(None); }
        }
        _ = interrupt.recv() => return Ok(None),
    }
    let response: Value = serde_json::from_str(&response)?;
    match response["cancelled"].as_bool() {
        Some(true) => Ok(None),
        Some(false) => {
            let selection = response["selection"]
                .as_str()
                .ok_or_else(|| anyhow!("missing selection"))?;
            anyhow::ensure!(
                selection.starts_with("[SELECTION]/") && !selection.contains('\n'),
                "invalid selection"
            );
            Ok(Some(selection.to_owned()))
        }
        None => Err(anyhow!("missing cancellation status")),
    }
}
