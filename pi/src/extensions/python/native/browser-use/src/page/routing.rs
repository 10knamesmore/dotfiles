//! Execute declarative request rules in Rust while Python is blocked or between cells.

use std::collections::BTreeMap;
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicU64, Ordering},
};

use anyhow::{Context, bail};
use base64::{Engine, prelude::BASE64_STANDARD};
use chromiumoxide::cdp::browser_protocol::{
    fetch::{
        ContinueRequestParams, DisableParams, EnableParams, EventRequestPaused, FailRequestParams,
        FulfillRequestParams, HeaderEntry, RequestPattern,
    },
    network::{ErrorReason, SetCacheDisabledParams},
};
use futures::StreamExt;
use globset::{GlobBuilder, GlobMatcher};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use serde::Deserialize;

use crate::diagnostics;

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Response {
    status: Option<i64>,
    headers: Option<BTreeMap<String, String>>,
    content_type: Option<String>,
    #[serde(skip)]
    body: Vec<u8>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Request {
    url: Option<String>,
    method: Option<String>,
    headers: Option<BTreeMap<String, String>>,
    #[serde(skip)]
    post_data: Option<Vec<u8>>,
}

enum Action {
    Abort,
    Fulfill(Response),
    Continue(Request),
}

pub(super) struct Rule {
    pub id: String,
    pattern: GlobMatcher,
    action: Action,
}

fn record_without_payload<'py>(
    value: &'py Bound<'py, PyAny>,
    key: &str,
) -> PyResult<(Bound<'py, PyDict>, Option<Vec<u8>>)> {
    let record = value.cast::<PyDict>()?.copy()?;
    let payload = match record.get_item(key)? {
        Some(value) => {
            let bytes = if let Ok(text) = value.extract::<String>() {
                text.into_bytes()
            } else {
                value.cast::<PyBytes>()?.as_bytes().to_vec()
            };
            record.del_item(key)?;
            Some(bytes)
        }
        None => None,
    };
    Ok((record, payload))
}

pub(super) fn rule(
    pattern: String,
    action: &str,
    response: Option<&Bound<'_, PyAny>>,
    request: Option<&Bound<'_, PyAny>>,
) -> PyResult<Rule> {
    let parsed = (|| -> anyhow::Result<Action> {
        match action {
            "abort" if response.is_none() && request.is_none() => Ok(Action::Abort),
            "fulfill" if request.is_none() => {
                let response = response.context("fulfill requires a response record")?;
                let (record, body) = record_without_payload(response, "body")?;
                let mut response: Response = pythonize::depythonize(record.as_any())?;
                response.body = body.unwrap_or_default();
                if !(100..=599).contains(&response.status.unwrap_or(200)) { bail!("Response status must be between 100 and 599"); }
                Ok(Action::Fulfill(response))
            }
            "continue" if response.is_none() => {
                let request = if let Some(request) = request {
                    let (record, post_data) = record_without_payload(request, "post_data")?;
                    let mut request: Request = pythonize::depythonize(record.as_any())?;
                    request.post_data = post_data;
                    request
                } else { Request::default() };
                Ok(Action::Continue(request))
            }
            _ => bail!("Use action='abort', action='fulfill' with response, or action='continue' with an optional request record"),
        }
    })().map_err(|error| pyo3::exceptions::PyValueError::new_err(error.to_string()))?;
    let pattern = GlobBuilder::new(&pattern)
        .literal_separator(false)
        .build()
        .map_err(|error| pyo3::exceptions::PyValueError::new_err(error.to_string()))?
        .compile_matcher();
    static NEXT: AtomicU64 = AtomicU64::new(1);
    Ok(Rule {
        id: format!("route-{}", NEXT.fetch_add(1, Ordering::Relaxed)),
        pattern,
        action: parsed,
    })
}

/// Keep rules active until explicitly removed or their page/session closes.
pub(super) struct Routing {
    page: chromiumoxide::Page,
    rules: Arc<RwLock<Vec<Arc<Rule>>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Routing {
    pub async fn start(page: chromiumoxide::Page) -> anyhow::Result<Self> {
        let mut events = page.event_listener::<EventRequestPaused>().await?;
        let rules: Arc<RwLock<Vec<Arc<Rule>>>> = Arc::new(RwLock::new(Vec::new()));
        let active = rules.clone();
        let target = page.clone();
        let task = tokio::spawn(async move {
            while let Some(event) = events.next().await {
                let rule = active
                    .read()
                    .unwrap()
                    .iter()
                    .rev()
                    .find(|rule| rule.pattern.is_match(&event.request.url))
                    .cloned();
                if let Err(_error) = respond(&target, &event, rule.as_deref()).await {
                    diagnostics::event("route_response", "failed");
                    // A failed rule must not leave the request suspended indefinitely.
                    let _ = target
                        .execute(FailRequestParams::new(
                            event.request_id.clone(),
                            ErrorReason::Failed,
                        ))
                        .await;
                }
            }
        });
        Ok(Self { page, rules, task })
    }

    pub async fn add(&self, rule: Rule) -> anyhow::Result<String> {
        let id = rule.id.clone();
        self.rules.write().unwrap().push(Arc::new(rule));
        self.page.execute(SetCacheDisabledParams::new(true)).await?;
        self.page
            .execute(
                EnableParams::builder()
                    .pattern(RequestPattern::builder().url_pattern("*").build())
                    .build(),
            )
            .await?;
        diagnostics::event("route_registered", &id);
        Ok(id)
    }

    pub async fn remove(&self, id: Option<&str>) -> anyhow::Result<()> {
        self.rules
            .write()
            .unwrap()
            .retain(|rule| id.is_some_and(|id| rule.id != id));
        let empty = self.rules.read().unwrap().is_empty();
        if empty {
            self.disable().await?;
        }
        Ok(())
    }

    async fn disable(&self) -> anyhow::Result<()> {
        self.page.execute(DisableParams::default()).await?;
        self.page
            .execute(SetCacheDisabledParams::new(false))
            .await?;
        Ok(())
    }

    pub async fn close(&self) {
        self.rules.write().unwrap().clear();
        if self.disable().await.is_err() {
            diagnostics::event("route_cleanup", "connection_closed_or_failed");
        }
        self.task.abort();
    }
}

impl Drop for Routing {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn respond(
    page: &chromiumoxide::Page,
    event: &EventRequestPaused,
    rule: Option<&Rule>,
) -> anyhow::Result<()> {
    let Some(rule) = rule else {
        page.execute(ContinueRequestParams::new(event.request_id.clone()))
            .await?;
        return Ok(());
    };
    match &rule.action {
        Action::Abort => {
            page.execute(FailRequestParams::new(
                event.request_id.clone(),
                ErrorReason::BlockedByClient,
            ))
            .await?;
        }
        Action::Fulfill(response) => {
            let mut headers = response.headers.clone().unwrap_or_default();
            if let Some(content_type) = &response.content_type {
                headers.retain(|name, _| !name.eq_ignore_ascii_case("content-type"));
                headers.insert("Content-Type".into(), content_type.clone());
            }
            let headers: Vec<_> = headers
                .into_iter()
                .map(|(name, value)| HeaderEntry::new(name, value))
                .collect();
            page.execute(
                FulfillRequestParams::builder()
                    .request_id(event.request_id.clone())
                    .response_code(response.status.unwrap_or(200))
                    .response_headers(headers)
                    .body(BASE64_STANDARD.encode(&response.body))
                    .build()
                    .map_err(anyhow::Error::msg)?,
            )
            .await?;
        }
        Action::Continue(request) => {
            let mut command = ContinueRequestParams::builder().request_id(event.request_id.clone());
            if let Some(url) = &request.url {
                command = command.url(url.clone());
            }
            if let Some(method) = &request.method {
                command = command.method(method.clone());
            }
            if let Some(data) = &request.post_data {
                command = command.post_data(BASE64_STANDARD.encode(data));
            }
            if let Some(headers) = &request.headers {
                command = command.headers(
                    headers
                        .iter()
                        .map(|(name, value)| HeaderEntry::new(name.clone(), value.clone()))
                        .collect::<Vec<_>>(),
                );
            }
            page.execute(command.build().map_err(anyhow::Error::msg)?)
                .await?;
        }
    }
    diagnostics::event("route_response", &rule.id);
    Ok(())
}
