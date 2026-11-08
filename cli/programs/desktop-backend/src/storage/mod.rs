mod operations;
mod protocol;
mod settings;
mod snapshot;

use std::collections::VecDeque;

use anyhow::{Context, Result, bail, ensure};
use futures_util::StreamExt;
use rustc_hash::{FxHashMap, FxHashSet};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::unix::pipe::Receiver,
    signal::unix::{SignalKind, signal},
    task::JoinSet,
};
use zbus::{Connection, MatchRule, Message, MessageStream, fdo, message::Type};

use self::{
    operations::AutomaticMount,
    protocol::{Action, Command, Event, Output},
    settings::{Preference, Settings, Store},
    snapshot::{ROOT, SERVICE, Snapshot},
};

struct PendingCommand {
    command: Command,
    automatic: Option<AutomaticMount>,
}

struct Completion {
    command: Command,
    result: Result<()>,
    removing: Vec<String>,
    name: String,
}

struct Session {
    connection: Connection,
    output: Output,
    store: Store,
    settings: Settings,
    snapshot: Snapshot,
    queue: VecDeque<PendingCommand>,
    // Only blocks first seen after startup may become automatic mount candidates.
    new_blocks: FxHashSet<String>,
    removing: FxHashMap<String, u64>,
    automatic_id: u64,
}

pub async fn run() -> Result<()> {
    let (store, settings) = Store::load().await?;
    let connection = Connection::system().await?;
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender(SERVICE)?
        .path_namespace(ROOT)?
        .build();
    // Subscribe before the first read so insertion events cannot be lost between them.
    let mut changes = MessageStream::for_match_rule(rule, &connection, Some(256)).await?;
    let bus = fdo::DBusProxy::new(&connection).await?;
    let mut owners = bus
        .receive_name_owner_changed_with_args(&[(0, SERVICE)])
        .await?;
    let snapshot = Snapshot::read(&connection).await?;
    let mut session = Session {
        connection,
        output: Output(tokio::io::stdout()),
        store,
        settings,
        snapshot,
        queue: VecDeque::new(),
        new_blocks: FxHashSet::default(),
        removing: FxHashMap::default(),
        automatic_id: 0,
    };
    session.publish().await?;
    session.output.emit(Event::Ready).await?;

    let input = Receiver::from_owned_fd(nix::unistd::dup(std::io::stdin())?)?;
    let mut lines = BufReader::new(input).lines();
    let mut terminate = signal(SignalKind::terminate())?;
    let interrupted = tokio::signal::ctrl_c();
    tokio::pin!(interrupted);
    let mut tasks = JoinSet::new();
    let result = async {
        loop {
            if tasks.is_empty() {
                session.start_next(&mut tasks).await?;
            }
            tokio::select! {
                line = lines.next_line() => {
                    let Some(line) = line? else { return Ok(()); };
                    session.queue.push_back(PendingCommand {
                        command: serde_json::from_str(&line).context("read storage command")?,
                        automatic: None,
                    });
                }
                message = changes.next() => {
                    let message = message.context("UDisks2 event stream closed")??;
                    if changes_state(&message)? {
                        session.refresh().await?;
                    }
                }
                completed = tasks.join_next(), if !tasks.is_empty() => {
                    session.complete(completed.unwrap()?).await?;
                }
                changed = owners.next() => {
                    let changed = changed.context("D-Bus service owner stream closed")?;
                    if changed.args()?.old_owner().as_ref().is_some() {
                        bail!("UDisks2 service disconnected");
                    }
                }
                _ = terminate.recv() => return Ok(()),
                interrupted = &mut interrupted => {
                    interrupted?;
                    return Ok(());
                }
            }
        }
    }
    .await;
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
    result
}

impl Session {
    async fn publish(&mut self) -> Result<()> {
        self.output
            .emit(Event::State {
                drives: &self.snapshot.drives,
                settings: &self.settings,
            })
            .await
    }

    async fn refresh(&mut self) -> Result<()> {
        let next = Snapshot::read(&self.connection).await?;
        for drive in &self.snapshot.drives {
            let still_present = next.drives.iter().any(|other| {
                other.path == drive.path && other.media_detected == drive.media_detected
            });
            if !still_present {
                let intentional = self.removing.remove(&drive.path) == Some(drive.media_detected);
                if !intentional {
                    self.output
                        .notice(
                            &self.settings,
                            "removed",
                            "Storage removed",
                            &drive.name,
                            &drive.path,
                        )
                        .await?;
                }
            }
        }
        for drive in &next.drives {
            let previous =
                self.snapshot.drives.iter().find(|old| {
                    old.path == drive.path && old.media_detected == drive.media_detected
                });
            if previous.is_none() {
                self.output
                    .notice(
                        &self.settings,
                        "inserted",
                        "Storage inserted",
                        &drive.name,
                        &drive.path,
                    )
                    .await?;
            }
        }

        let previous_blocks = self.snapshot.block_paths.iter().collect::<FxHashSet<_>>();
        self.new_blocks
            .retain(|path| next.block_paths.contains(path));
        for path in &next.block_paths {
            if !previous_blocks.contains(path) {
                self.new_blocks.insert(path.clone());
            }
        }
        for drive in &next.drives {
            // Readers can retain block objects even while no card is present.
            let new_media =
                !self.snapshot.drives.iter().any(|old| {
                    old.path == drive.path && old.media_detected == drive.media_detected
                });
            for volume in &drive.volumes {
                if new_media {
                    self.new_blocks.insert(volume.path.clone());
                }
                if volume.mountable
                    && self.new_blocks.remove(&volume.path)
                    && volume.mount_points.is_empty()
                    && self.settings.should_mount(&volume.key)
                    && !self.removing.contains_key(&drive.path)
                {
                    self.automatic_id += 1;
                    self.queue.push_back(PendingCommand {
                        command: Command {
                            id: format!("auto-{}", self.automatic_id),
                            action: Action::Mount {
                                target: volume.path.clone(),
                                read_only: None,
                            },
                        },
                        automatic: Some(AutomaticMount {
                            drive: drive.path.clone(),
                            media_detected: drive.media_detected,
                            uuid: volume.uuid.clone(),
                        }),
                    });
                }
            }
        }
        let changed = next.drives != self.snapshot.drives;
        self.snapshot = next;
        if changed {
            self.publish().await?;
        }
        Ok(())
    }

    async fn start_next(&mut self, tasks: &mut JoinSet<Completion>) -> Result<()> {
        while let Some(PendingCommand { command, automatic }) = self.queue.pop_front() {
            self.output.operation(&command, "running", None).await?;
            if matches!(
                command.action,
                Action::Settings { .. } | Action::Preference { .. }
            ) {
                let result = self.update_settings(&command).await;
                self.complete(Completion {
                    command,
                    result,
                    removing: Vec::new(),
                    name: String::new(),
                })
                .await?;
                continue;
            }
            let mut removing = Vec::new();
            let mut name = String::new();
            if let Action::Mount { target, .. } = &command.action
                && let Some((drive, _)) = self.snapshot.volume(target)
            {
                self.removing.remove(&drive.path);
            }
            if let Action::Remove { target } = &command.action
                && let Ok(drives) = operations::removal_drives(&self.snapshot, target)
            {
                name = drives[0].name.clone();
                for drive in drives {
                    removing.push(drive.path.clone());
                    self.removing
                        .insert(drive.path.clone(), drive.media_detected);
                }
            }
            let connection = self.connection.clone();
            let settings = self.settings.clone();
            // Serialize filesystem operations; keep processing D-Bus events while UDisks authorizes them.
            tasks.spawn_local(async move {
                let result =
                    operations::execute(&connection, &command, &settings, automatic.as_ref()).await;
                Completion {
                    command,
                    result,
                    removing,
                    name,
                }
            });
            break;
        }
        Ok(())
    }

    async fn update_settings(&mut self, command: &Command) -> Result<()> {
        let mut settings = self.settings.clone();
        match &command.action {
            Action::Settings {
                auto_mount,
                notifications,
            } => {
                settings.auto_mount = *auto_mount;
                settings.notifications = *notifications;
            }
            Action::Preference {
                target,
                auto_mount,
                read_only,
            } => {
                ensure!(
                    !target.is_empty(),
                    "remembered preferences require a filesystem UUID"
                );
                settings.volumes.insert(
                    target.clone(),
                    Preference {
                        auto_mount: *auto_mount,
                        read_only: *read_only,
                    },
                );
            }
            _ => unreachable!(),
        }
        self.store.save(&settings).await?;
        self.settings = settings;
        self.publish().await?;
        Ok(())
    }

    async fn complete(&mut self, completion: Completion) -> Result<()> {
        let Completion {
            command,
            result,
            removing,
            name,
        } = completion;
        match result {
            Ok(()) => {
                self.output.operation(&command, "done", None).await?;
                if matches!(command.action, Action::Remove { .. }) {
                    self.output
                        .notice(
                            &self.settings,
                            "safe",
                            "Safe to remove",
                            &name,
                            command.target(),
                        )
                        .await?;
                }
            }
            Err(error) => {
                for path in removing {
                    self.removing.remove(&path);
                }
                let error = format!("{error:#}");
                self.output
                    .operation(&command, "error", Some(&error))
                    .await?;
                self.output
                    .notice(
                        &self.settings,
                        "error",
                        "Storage operation failed",
                        &error,
                        command.target(),
                    )
                    .await?;
            }
        }
        // Methods can complete before their last PropertiesChanged reaches this connection.
        self.refresh().await
    }
}

fn changes_state(message: &Message) -> Result<bool> {
    if let Some(signal) = fdo::PropertiesChanged::from_message(message.clone()) {
        return Ok(signal
            .args()?
            .interface_name()
            .as_str()
            .starts_with(SERVICE));
    }
    let header = message.header();
    Ok(
        header.interface().map(|name| name.as_str()) == Some("org.freedesktop.DBus.ObjectManager")
            && matches!(
                header.member().map(|name| name.as_str()),
                Some("InterfacesAdded" | "InterfacesRemoved")
            ),
    )
}
