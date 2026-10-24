mod agent;
mod operations;
mod protocol;
mod snapshot;

use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Stdout},
    net::unix::pipe::Receiver,
    signal::unix::{SignalKind, signal},
    sync::mpsc,
    task::JoinSet,
};
use zbus::{Connection, MatchRule, Message, MessageStream, fdo, message::Type};

use self::{
    agent::Agent,
    operations::Session,
    protocol::{Command, Event, Output},
    snapshot::Snapshot,
};

pub async fn run() -> Result<()> {
    let connection = Connection::system().await?;
    // Subscribe before GetManagedObjects so changes during the initial read are retained.
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender("org.bluez")?
        .build();
    let mut changes = MessageStream::for_match_rule(rule, &connection, Some(256)).await?;
    let (sender, mut events) = mpsc::unbounded_channel();
    let output = Output(sender);
    let session = Session::new(connection, Agent::new(output.clone()), output);
    let mut tasks = JoinSet::new();
    let mut stdout = tokio::io::stdout();
    let result = serve(&session, &mut changes, &mut events, &mut tasks, &mut stdout).await;
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
    session.cleanup().await;
    while let Ok(event) = events.try_recv() {
        write_event(&mut stdout, event).await?;
    }
    result
}

async fn serve(
    session: &Session,
    changes: &mut MessageStream,
    events: &mut mpsc::UnboundedReceiver<Event>,
    tasks: &mut JoinSet<Result<()>>,
    stdout: &mut Stdout,
) -> Result<()> {
    let mut terminate = signal(SignalKind::terminate())?;
    let interrupted = tokio::signal::ctrl_c();
    tokio::pin!(interrupted);
    // A nonblocking pipe lets signal shutdown finish without Tokio's blocking stdin reader.
    let input = Receiver::from_owned_fd(nix::unistd::dup(std::io::stdin())?)?;
    let mut lines = BufReader::new(input).lines();
    let mut published = None;
    publish_snapshot(session, &mut published).await?;
    session.register_agent().await?;
    session.output.emit(Event::Ready)?;

    loop {
        tokio::select! {
            line = lines.next_line() => {
                let Some(line) = line? else {
                    return Ok(());
                };
                match serde_json::from_str(&line).context("read Bluetooth command")? {
                    Command::Respond { request_id, accept, value } => {
                        if let Err(error) = session.agent.respond(&request_id, accept, value) {
                            eprintln!("[bluetooth] {error:#}");
                        }
                    }
                    Command::Operation(operation) => {
                        let session = session.clone();
                        tasks.spawn_local(async move { session.perform(operation).await });
                    }
                }
            }
            event = events.recv() => {
                write_event(stdout, event.context("Bluetooth output channel closed")?).await?;
            }
            message = changes.next() => {
                let message = message.context("BlueZ event stream closed")??;
                if changes_state(&message)? {
                    publish_snapshot(session, &mut published).await?;
                }
            }
            completed = tasks.join_next(), if !tasks.is_empty() => {
                completed.unwrap()??;
            }
            _ = session.agent.released.notified() => {
                bail!("BlueZ released the Bluetooth agent");
            }
            _ = terminate.recv() => return Ok(()),
            interrupted = &mut interrupted => {
                interrupted?;
                return Ok(());
            }
        }
    }
}

async fn publish_snapshot(session: &Session, published: &mut Option<Snapshot>) -> Result<()> {
    let snapshot = Snapshot::read(&session.connection).await?;
    if published.as_ref() != Some(&snapshot) {
        session.output.emit(snapshot.clone().into_event())?;
        *published = Some(snapshot);
    }
    Ok(())
}

fn changes_state(message: &Message) -> Result<bool> {
    if let Some(signal) = fdo::PropertiesChanged::from_message(message.clone()) {
        return Ok(signal
            .args()?
            .interface_name()
            .as_str()
            .starts_with("org.bluez."));
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

async fn write_event(stdout: &mut Stdout, event: Event) -> Result<()> {
    let mut line = serde_json::to_vec(&event)?;
    line.push(b'\n');
    stdout.write_all(&line).await?;
    stdout.flush().await?;
    Ok(())
}
