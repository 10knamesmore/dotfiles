//! Convert Python cookie records to Chrome's default-context storage API.

use anyhow::{Context, bail};
use chromiumoxide::cdp::browser_protocol::network::{
    Cookie, CookieParam, CookiePartitionKey, CookieSameSite, TimeSinceEpoch,
};
use pyo3::prelude::*;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CookieInput {
    name: String,
    value: String,
    url: Option<String>,
    domain: Option<String>,
    path: Option<String>,
    expires: Option<f64>,
    http_only: Option<bool>,
    secure: Option<bool>,
    same_site: Option<String>,
    partition_key: Option<PartitionKey>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PartitionKey {
    top_level_site: String,
    has_cross_site_ancestor: bool,
}

pub(super) fn parse(cookies: &Bound<'_, PyAny>) -> PyResult<Vec<CookieParam>> {
    let inputs: Vec<CookieInput> = pythonize::depythonize(cookies)?;
    inputs
        .into_iter()
        .map(convert)
        .collect::<anyhow::Result<_>>()
        .map_err(|error| pyo3::exceptions::PyValueError::new_err(error.to_string()))
}

fn convert(cookie: CookieInput) -> anyhow::Result<CookieParam> {
    if cookie.url.is_none() && (cookie.domain.is_none() || cookie.path.is_none()) {
        bail!("Each cookie requires url or both domain and path");
    }
    let mut builder = CookieParam::builder().name(cookie.name).value(cookie.value);
    if let Some(url) = cookie.url {
        builder = builder.url(url);
    }
    if let Some(domain) = cookie.domain {
        builder = builder.domain(domain);
    }
    if let Some(path) = cookie.path {
        builder = builder.path(path);
    }
    if let Some(expires) = cookie.expires {
        if !expires.is_finite() {
            bail!("Cookie expires must be finite Unix seconds");
        }
        builder = builder.expires(TimeSinceEpoch::new(expires));
    }
    if let Some(value) = cookie.http_only {
        builder = builder.http_only(value);
    }
    if let Some(value) = cookie.secure {
        builder = builder.secure(value);
    }
    if let Some(value) = cookie.same_site {
        builder = builder.same_site(
            value
                .parse::<CookieSameSite>()
                .ok()
                .context("same_site must be Strict, Lax or None")?,
        );
    }
    if let Some(key) = cookie.partition_key {
        builder = builder.partition_key(CookiePartitionKey::new(
            key.top_level_site,
            key.has_cross_site_ancestor,
        ));
    }
    builder.build().map_err(anyhow::Error::msg)
}

pub(super) fn records(cookies: Vec<Cookie>) -> Vec<Value> {
    cookies.into_iter().map(|cookie| json!({
        "name": cookie.name, "value": cookie.value, "domain": cookie.domain, "path": cookie.path,
        "expires": if cookie.session { Value::Null } else { json!(cookie.expires) }, "http_only": cookie.http_only, "secure": cookie.secure,
        "same_site": cookie.same_site,
        "partition_key": cookie.partition_key.map(|key| json!({"top_level_site":key.top_level_site, "has_cross_site_ancestor": key.has_cross_site_ancestor})),
    })).collect()
}
