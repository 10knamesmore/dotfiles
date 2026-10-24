#![allow(
    clippy::disallowed_types,
    reason = "zbus interface expansion generates std::collections::HashMap"
)]

use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use anyhow::{Result, ensure};
use tokio::sync::{Notify, oneshot};
use zbus::{DBusError, interface, zvariant::OwnedObjectPath};

use super::protocol::{Event, Output, Prompt, PromptKind};

pub const PATH: &str = "/org/dotfiles/desktop_backend/bluetooth_agent";

#[derive(Debug, DBusError)]
#[zbus(prefix = "org.bluez.Error")]
pub enum AgentError {
    Rejected(String),
    Canceled(String),
}

type Answer = std::result::Result<String, AgentError>;

struct PendingPrompt {
    prompt: Prompt,
    answer: Option<oneshot::Sender<Answer>>,
}

#[derive(Default)]
struct Prompts {
    next_id: u64,
    pending: BTreeMap<String, PendingPrompt>,
}

#[derive(Clone)]
pub struct Agent {
    output: Output,
    prompts: Arc<Mutex<Prompts>>,
    pub registered: Arc<AtomicBool>,
    pub released: Arc<Notify>,
}

impl Agent {
    pub fn new(output: Output) -> Self {
        Self {
            output,
            prompts: Arc::default(),
            registered: Arc::default(),
            released: Arc::default(),
        }
    }

    fn publish(
        &self,
        device: OwnedObjectPath,
        kind: PromptKind,
        passkey: Option<String>,
        entered: Option<u16>,
        uuid: Option<String>,
        answer: Option<oneshot::Sender<Answer>>,
    ) -> std::result::Result<(), AgentError> {
        let mut prompts = self.prompts.lock().unwrap();
        let existing_id = if answer.is_none() {
            prompts
                .pending
                .iter()
                .find(|(_, pending)| {
                    pending.answer.is_none()
                        && pending.prompt.device == device
                        && pending.prompt.kind == kind
                })
                .map(|(id, _)| id.clone())
        } else {
            None
        };
        let request_id = existing_id.unwrap_or_else(|| {
            prompts.next_id += 1;
            prompts.next_id.to_string()
        });
        let prompt = Prompt {
            request_id: request_id.clone(),
            device,
            kind,
            passkey,
            entered,
            uuid,
        };
        self.output
            .emit(Event::Prompt {
                prompt: prompt.clone(),
            })
            .map_err(|error| AgentError::Canceled(error.to_string()))?;
        prompts
            .pending
            .insert(request_id, PendingPrompt { prompt, answer });
        Ok(())
    }

    async fn ask(
        &self,
        device: OwnedObjectPath,
        kind: PromptKind,
        passkey: Option<String>,
        uuid: Option<String>,
    ) -> Answer {
        let (answer, response) = oneshot::channel();
        self.publish(device, kind, passkey, None, uuid, Some(answer))?;
        response
            .await
            .map_err(|_| AgentError::Canceled("Bluetooth session closed".into()))?
    }

    pub fn respond(&self, request_id: &str, accept: bool, value: Option<String>) -> Result<()> {
        let mut prompts = self.prompts.lock().unwrap();
        let pending = prompts
            .pending
            .get(request_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown Bluetooth prompt {request_id}"))?;
        ensure!(
            pending.answer.is_some(),
            "Display-only prompt has no response"
        );
        // Invalid input leaves the prompt open, so the user can correct it.
        if accept {
            match pending.prompt.kind {
                PromptKind::Pin => {
                    let pin = value.as_deref().unwrap_or_default();
                    ensure!(
                        (1..=16).contains(&pin.chars().count()) && !pin.contains('\0'),
                        "Bluetooth PIN must contain 1–16 characters without NUL"
                    );
                }
                PromptKind::Passkey => {
                    let passkey = value.as_deref().unwrap_or_default();
                    ensure!(
                        (1..=6).contains(&passkey.len())
                            && passkey.bytes().all(|byte| byte.is_ascii_digit()),
                        "Bluetooth passkey must be 1–6 decimal digits (0–999999)"
                    );
                }
                _ => {}
            }
        }
        let pending = prompts.pending.remove(request_id).unwrap();
        let answer = if accept {
            Ok(value.unwrap_or_default())
        } else {
            Err(AgentError::Rejected("Rejected by user".into()))
        };
        let _ = pending.answer.unwrap().send(answer);
        self.output.emit(Event::PromptClosed {
            request_id: request_id.to_owned(),
        })
    }

    pub fn close_device(&self, device: &OwnedObjectPath) {
        self.close_matching(|prompt| &prompt.device == device);
    }

    pub fn close_all(&self) {
        self.close_matching(|_| true);
    }

    fn close_matching(&self, matches: impl Fn(&Prompt) -> bool) {
        let mut prompts = self.prompts.lock().unwrap();
        let ids: Vec<_> = prompts
            .pending
            .iter()
            .filter(|(_, pending)| matches(&pending.prompt))
            .map(|(id, _)| id.clone())
            .collect();
        for request_id in ids {
            let pending = prompts.pending.remove(&request_id).unwrap();
            if let Some(answer) = pending.answer {
                let _ = answer.send(Err(AgentError::Canceled("Pairing canceled".into())));
            }
            let _ = self.output.emit(Event::PromptClosed { request_id });
        }
    }
}

// Each call takes &self, and zbus spawns calls independently: Cancel and stdin responses
// must remain runnable while a security request awaits its oneshot.
#[interface(name = "org.bluez.Agent1", spawn = true)]
impl Agent {
    fn release(&self) {
        self.registered.store(false, Ordering::SeqCst);
        self.close_all();
        self.released.notify_one();
    }

    async fn request_pin_code(
        &self,
        device: OwnedObjectPath,
    ) -> std::result::Result<String, AgentError> {
        self.ask(device, PromptKind::Pin, None, None).await
    }

    fn display_pin_code(
        &self,
        device: OwnedObjectPath,
        pincode: String,
    ) -> std::result::Result<(), AgentError> {
        self.publish(
            device,
            PromptKind::DisplayPin,
            Some(pincode),
            None,
            None,
            None,
        )
    }

    async fn request_passkey(
        &self,
        device: OwnedObjectPath,
    ) -> std::result::Result<u32, AgentError> {
        let value = self.ask(device, PromptKind::Passkey, None, None).await?;
        Ok(value.parse().unwrap())
    }

    fn display_passkey(
        &self,
        device: OwnedObjectPath,
        passkey: u32,
        entered: u16,
    ) -> std::result::Result<(), AgentError> {
        self.publish(
            device,
            PromptKind::DisplayPasskey,
            Some(format!("{passkey:06}")),
            Some(entered),
            None,
            None,
        )
    }

    async fn request_confirmation(
        &self,
        device: OwnedObjectPath,
        passkey: u32,
    ) -> std::result::Result<(), AgentError> {
        self.ask(
            device,
            PromptKind::Confirmation,
            Some(format!("{passkey:06}")),
            None,
        )
        .await?;
        Ok(())
    }

    async fn request_authorization(
        &self,
        device: OwnedObjectPath,
    ) -> std::result::Result<(), AgentError> {
        self.ask(device, PromptKind::Authorization, None, None)
            .await?;
        Ok(())
    }

    async fn authorize_service(
        &self,
        device: OwnedObjectPath,
        uuid: String,
    ) -> std::result::Result<(), AgentError> {
        self.ask(device, PromptKind::Service, None, Some(uuid))
            .await?;
        Ok(())
    }

    fn cancel(&self) {
        self.close_all();
    }
}
