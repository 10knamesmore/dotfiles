mod common;
mod picker;
mod portal;
mod recording;
mod screenshots;
mod server;

pub use picker::pick;
pub use server::serve;

/// The server survives the launcher's destruction so stdin EOF can finalize its video.
pub async fn launch() -> anyhow::Result<()> {
    let status = tokio::process::Command::new(std::env::current_exe()?)
        .arg("capture-server")
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?
        .wait()
        .await?;
    anyhow::ensure!(status.success(), "capture server exited {status}");
    Ok(())
}
