use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use zbus::zvariant::OwnedObjectPath;

use super::snapshot::{Adapter, Device};

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum Command {
    Respond {
        #[serde(rename = "requestId")]
        request_id: String,
        accept: bool,
        value: Option<String>,
    },
    #[serde(untagged)]
    Operation(Operation),
}

#[derive(Debug, Deserialize)]
pub struct Operation {
    pub id: String,
    pub action: Action,
    pub device: Option<OwnedObjectPath>,
    pub adapter: Option<OwnedObjectPath>,
    pub value: Option<bool>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    Connect,
    Disconnect,
    Cancel,
    Forget,
    Trust,
    Power,
    Scan,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    Pairing,
    Connecting,
    Done,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PromptKind {
    Confirmation,
    Pin,
    Passkey,
    DisplayPin,
    DisplayPasskey,
    Authorization,
    Service,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub request_id: String,
    pub device: OwnedObjectPath,
    pub kind: PromptKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passkey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entered: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event {
    State {
        adapters: Vec<Adapter>,
        devices: Vec<Device>,
    },
    Ready,
    Operation {
        id: String,
        device: String,
        action: Action,
        stage: Stage,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    Prompt {
        #[serde(flatten)]
        prompt: Prompt,
    },
    PromptClosed {
        #[serde(rename = "requestId")]
        request_id: String,
    },
}

#[derive(Clone)]
pub struct Output(pub mpsc::UnboundedSender<Event>);

impl Output {
    pub fn emit(&self, event: Event) -> Result<()> {
        self.0.send(event)?;
        Ok(())
    }

    pub fn operation(
        &self,
        operation: &Operation,
        stage: Stage,
        error: Option<String>,
    ) -> Result<()> {
        self.emit(Event::Operation {
            id: operation.id.clone(),
            device: operation
                .device
                .as_ref()
                .map(|path| path.as_str().to_owned())
                .unwrap_or_default(),
            action: operation.action,
            stage,
            error,
        })
    }
}
