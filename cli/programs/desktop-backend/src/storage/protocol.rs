use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncWriteExt, Stdout};

use super::{
    settings::{AutoMount, Settings},
    snapshot::Drive,
};

#[derive(Clone, Debug, Deserialize)]
pub struct Command {
    pub id: String,
    #[serde(flatten)]
    pub action: Action,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum Action {
    Mount {
        target: String,
        #[serde(rename = "readOnly")]
        read_only: Option<bool>,
    },
    Unmount {
        target: String,
    },
    Remove {
        target: String,
    },
    Settings {
        #[serde(rename = "autoMount")]
        auto_mount: bool,
        notifications: bool,
    },
    Preference {
        target: String,
        #[serde(rename = "autoMount")]
        auto_mount: AutoMount,
        #[serde(rename = "readOnly")]
        read_only: bool,
    },
}

impl Command {
    pub fn name(&self) -> &'static str {
        match self.action {
            Action::Mount { .. } => "mount",
            Action::Unmount { .. } => "unmount",
            Action::Remove { .. } => "remove",
            Action::Settings { .. } => "settings",
            Action::Preference { .. } => "preference",
        }
    }

    pub fn target(&self) -> &str {
        match &self.action {
            Action::Mount { target, .. }
            | Action::Unmount { target }
            | Action::Remove { target }
            | Action::Preference { target, .. } => target,
            Action::Settings { .. } => "",
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event<'a> {
    State {
        drives: &'a [Drive],
        settings: &'a Settings,
    },
    Ready,
    Operation {
        id: &'a str,
        action: &'a str,
        target: &'a str,
        stage: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<&'a str>,
    },
    Notice {
        kind: &'a str,
        title: &'a str,
        body: &'a str,
        target: &'a str,
    },
}

pub struct Output(pub Stdout);

impl Output {
    pub async fn emit(&mut self, event: Event<'_>) -> Result<()> {
        let mut line = serde_json::to_vec(&event)?;
        line.push(b'\n');
        self.0.write_all(&line).await?;
        self.0.flush().await?;
        Ok(())
    }

    pub async fn operation(
        &mut self,
        command: &Command,
        stage: &str,
        error: Option<&str>,
    ) -> Result<()> {
        self.emit(Event::Operation {
            id: &command.id,
            action: command.name(),
            target: command.target(),
            stage,
            error,
        })
        .await
    }

    pub async fn notice(
        &mut self,
        settings: &Settings,
        kind: &str,
        title: &str,
        body: &str,
        target: &str,
    ) -> Result<()> {
        if settings.notifications {
            self.emit(Event::Notice {
                kind,
                title,
                body,
                target,
            })
            .await?;
        }
        Ok(())
    }
}
