use anyhow::{Context, Result, bail};
use nix::{
    errno::Errno,
    sys::signal::{Signal, killpg},
    unistd::Pid,
};
use std::{net::TcpListener, path::Path, process::Stdio, time::Duration};
use tokio::{net::TcpStream, process::Child, sync::watch, time};

pub struct Preview {
    child: Child,
    process_group: Option<Pid>,
    pub url: String,
}

impl Preview {
    pub async fn start(root: &Path, shutdown: watch::Receiver<bool>) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        drop(listener);
        let mut command = tokio::process::Command::new("pnpm");
        command
            .args([
                "exec",
                "vite",
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--strictPort",
            ])
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(std::io::stderr())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);
        command.as_std_mut().process_group(0);
        let child = command.spawn().context("Launch pnpm exec vite")?;
        let process_group = Pid::from_raw(child.id().unwrap() as i32);
        let mut preview = Self {
            child,
            process_group: Some(process_group),
            url: format!("http://127.0.0.1:{port}"),
        };
        let deadline = time::Instant::now() + Duration::from_secs(20);
        loop {
            if *shutdown.borrow() {
                preview.stop().await?;
                bail!("Design service is shutting down");
            }
            if let Some(status) = preview.child.try_wait()? {
                preview.stop().await?;
                bail!("Vite exited before becoming ready: {status}");
            }
            if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
                return Ok(preview);
            }
            if time::Instant::now() >= deadline {
                preview.stop().await?;
                bail!("Vite did not become ready within 20 seconds");
            }
            time::sleep(Duration::from_millis(50)).await;
        }
    }

    pub fn exited(&mut self) -> Result<Option<std::process::ExitStatus>> {
        Ok(self.child.try_wait()?)
    }

    pub async fn stop(&mut self) -> Result<()> {
        // pnpm starts Vite as a descendant; terminating only pnpm leaves Vite running.
        if let Some(process_group) = self.process_group.take() {
            match killpg(process_group, Signal::SIGKILL) {
                Ok(()) | Err(Errno::ESRCH) => (),
                Err(error) => return Err(error.into()),
            }
        }
        self.child.wait().await?;
        Ok(())
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        if let Some(process_group) = self.process_group.take() {
            let _ = killpg(process_group, Signal::SIGKILL);
        }
    }
}

use std::os::unix::process::CommandExt;
