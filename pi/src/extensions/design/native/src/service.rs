use crate::{
    actor::{self, Actor, Event, Message, Open, Request, Selection},
    documents::{self, Edit},
};
use anyhow::{Context, Result, bail};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, Method, StatusCode},
    response::{
        IntoResponse, Response,
        sse::{Event as SseEvent, KeepAlive, Sse},
    },
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    convert::Infallible,
    fs,
    io::{Read, Write},
    path::PathBuf,
    time::Duration,
};
use tokio::sync::{broadcast, mpsc, oneshot, watch};
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    services::ServeDir,
};

#[derive(Clone)]
struct HttpState {
    actor: actor::Sender,
    events: broadcast::Sender<Event>,
    shutdown: watch::Receiver<bool>,
}

struct ApiError(anyhow::Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("{:#}", self.0)})),
        )
            .into_response()
    }
}

type ApiResult = std::result::Result<Json<Value>, ApiError>;

async fn dispatch(state: HttpState, headers: HeaderMap, request: Request) -> ApiResult {
    let expected = headers
        .get("x-pi-design-project")
        .map(|value| value.to_str().map(str::to_owned))
        .transpose()
        .map_err(|error| ApiError(error.into()))?;
    actor::request(&state.actor, request, expected)
        .await
        .map(Json)
        .map_err(ApiError)
}

async fn projects(State(state): State<HttpState>, headers: HeaderMap) -> ApiResult {
    dispatch(state, headers, Request::Projects).await
}

async fn preferences(State(state): State<HttpState>, headers: HeaderMap) -> ApiResult {
    dispatch(state, headers, Request::Preferences).await
}

#[derive(Deserialize)]
struct Theme {
    theme: String,
}

async fn set_theme(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(body): Json<Theme>,
) -> ApiResult {
    dispatch(state, headers, Request::SetTheme(body.theme)).await
}

async fn open(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(body): Json<Open>,
) -> ApiResult {
    dispatch(state, headers, Request::Open(body)).await
}

async fn current(State(state): State<HttpState>, headers: HeaderMap) -> ApiResult {
    dispatch(state, headers, Request::State).await
}

#[derive(Deserialize)]
struct Since {
    #[serde(default)]
    since: u64,
}

async fn changes(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(query): Query<Since>,
) -> ApiResult {
    dispatch(state, headers, Request::Changes(query.since)).await
}

#[derive(Deserialize)]
struct Edits {
    edits: Vec<Edit>,
    snapshot: Option<bool>,
}

async fn edit(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(body): Json<Edits>,
) -> ApiResult {
    let revision = headers
        .get("x-pi-design-revision")
        .map(|value| -> Result<u64> { Ok(value.to_str()?.parse()?) })
        .transpose()
        .map_err(ApiError)?;
    dispatch(
        state,
        headers,
        Request::Edit(body.edits, revision, body.snapshot.unwrap_or(true)),
    )
    .await
}

async fn undo(State(state): State<HttpState>, headers: HeaderMap) -> ApiResult {
    dispatch(state, headers, Request::Undo).await
}

async fn redo(State(state): State<HttpState>, headers: HeaderMap) -> ApiResult {
    dispatch(state, headers, Request::Redo).await
}

#[derive(Deserialize)]
struct HistorySnapshot {
    id: u64,
}

async fn restore_history(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(body): Json<HistorySnapshot>,
) -> ApiResult {
    dispatch(state, headers, Request::Restore(body.id)).await
}

async fn selection(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(body): Json<Selection>,
) -> ApiResult {
    dispatch(state, headers, Request::Selection(body)).await
}

async fn events(State(state): State<HttpState>) -> impl IntoResponse {
    let mut receiver = state.events.subscribe();
    let mut shutdown = state.shutdown;
    let initial = actor::request(&state.actor, Request::State, None).await;
    let stream = async_stream::stream! {
        let initial = match initial {
            Ok(value) => Event { name: "state", value },
            Err(error) => Event { name: "error", value: json!({"error": format!("{error:#}")}) },
        };
        yield Ok::<_, Infallible>(sse_event(initial));
        loop {
            if *shutdown.borrow() { break; }
            tokio::select! {
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() { break; }
                }
                event = receiver.recv() => match event {
                    Ok(event) => yield Ok(sse_event(event)),
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        let event = match actor::request(&state.actor, Request::State, None).await {
                            Ok(value) => Event { name: "state", value },
                            Err(error) => Event { name: "error", value: json!({"error": format!("{error:#}")}) },
                        };
                        yield Ok(sse_event(event));
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

fn sse_event(event: Event) -> SseEvent {
    SseEvent::default()
        .event(event.name)
        .data(event.value.to_string())
}

pub fn serve(agent_dir: PathBuf, web_dir: PathBuf, session_id: String) -> Result<()> {
    if session_id.is_empty()
        || session_id.contains(['/', '\\'])
        || session_id == "."
        || session_id == ".."
    {
        bail!("session_id must be a nonempty filename");
    }
    if !web_dir.is_dir() {
        bail!("Web assets directory does not exist: {}", web_dir.display());
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(run(agent_dir, web_dir, session_id))
}

async fn run(agent_dir: PathBuf, web_dir: PathBuf, session_id: String) -> Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}", listener.local_addr()?);
    let sessions = agent_dir.join("design/sessions");
    fs::create_dir_all(&sessions)?;
    let endpoint_file = sessions.join(format!("{session_id}.json"));
    let endpoint = json!({"url": url});
    documents::atomic_json(&endpoint_file, &endpoint)?;

    let (sender, receiver) = mpsc::unbounded_channel();
    let (events_sender, _) = broadcast::channel(256);
    let (shutdown_sender, shutdown_receiver) = watch::channel(false);
    let actor = tokio::spawn(
        Actor::new(
            agent_dir,
            sender.clone(),
            receiver,
            events_sender.clone(),
            shutdown_receiver.clone(),
        )
        .run(),
    );
    let router = Router::new()
        .route("/api/projects", get(projects))
        .route("/api/preferences", get(preferences).post(set_theme))
        .route("/api/open", post(open))
        .route("/api/state", get(current))
        .route("/api/changes", get(changes))
        .route("/api/edit", post(edit))
        .route("/api/undo", post(undo))
        .route("/api/redo", post(redo))
        .route("/api/history", post(restore_history))
        .route("/api/selection", post(selection))
        .route("/api/events", get(events))
        .fallback_service(ServeDir::new(web_dir).append_index_html_on_directories(true))
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::predicate(|origin, _| {
                    origin
                        .to_str()
                        .ok()
                        .and_then(|origin| reqwest::Url::parse(origin).ok())
                        .is_some_and(|url| {
                            url.scheme() == "http"
                                && matches!(
                                    url.host_str(),
                                    Some("127.0.0.1" | "localhost" | "[::1]")
                                )
                        })
                }))
                .allow_methods([Method::GET, Method::POST])
                .allow_headers(Any),
        )
        .with_state(HttpState {
            actor: sender.clone(),
            events: events_sender,
            shutdown: shutdown_receiver.clone(),
        });
    let mut server_shutdown = shutdown_receiver;
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = server_shutdown.wait_for(|shutdown| *shutdown).await;
            })
            .await
    });

    let (stdin_closed, stdin_eof) = oneshot::channel();
    std::thread::Builder::new()
        .name("pi-design-stdin".to_owned())
        .spawn(move || {
            let mut buffer = [0u8; 1024];
            let mut stdin = std::io::stdin().lock();
            loop {
                match stdin.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => (),
                }
            }
            let _ = stdin_closed.send(());
        })?;
    {
        let mut stdout = std::io::stdout().lock();
        writeln!(stdout, "{endpoint}")?;
        stdout.flush()?;
    }
    let mut server = server;
    let server_result = tokio::select! {
        _ = stdin_eof => None,
        result = &mut server => Some(result),
    };
    let _ = shutdown_sender.send(true);
    let (reply, response) = oneshot::channel();
    let _ = sender.send(Message::Shutdown(reply));
    let stopped = response
        .await
        .context("Design actor stopped during shutdown")?;
    actor.await?;
    let server_result = match server_result {
        Some(result) => result,
        None => server.await,
    };
    // Only this session owns this file; remove it when its service stops.
    fs::remove_file(endpoint_file)?;
    server_result??;
    stopped
}
