//! Resolve CSS, DOM text and Chrome-computed accessibility roles to short-lived DOM objects.

use anyhow::{Context, bail};
use chromiumoxide::cdp::{
    browser_protocol::{accessibility::QueryAxTreeParams, dom::ResolveNodeParams},
    js_protocol::runtime::{
        CallFunctionOnParams, EvaluateParams, ReleaseObjectParams, RemoteObjectId,
    },
};
use serde_json::Value;

use crate::runtime;

#[derive(Clone, Debug)]
pub(super) enum Query {
    Css(String),
    Text {
        text: String,
        exact: bool,
    },
    Role {
        role: String,
        name: Option<String>,
        exact: bool,
    },
}

pub(super) struct Matches {
    pub count: usize,
    pub element: Option<Element>,
}

/// Release the remote object after each operation, including canceled waits.
pub(super) struct Element {
    page: chromiumoxide::Page,
    pub id: RemoteObjectId,
}

impl Element {
    pub async fn call(&self, function: String) -> anyhow::Result<Value> {
        let response = self
            .page
            .execute(
                CallFunctionOnParams::builder()
                    .object_id(self.id.clone())
                    .function_declaration(function)
                    .return_by_value(true)
                    .await_promise(true)
                    .build()
                    .map_err(anyhow::Error::msg)?,
            )
            .await?
            .result;
        if let Some(exception) = response.exception_details {
            bail!("Element JavaScript failed: {exception:?}");
        }
        Ok(response.result.value.unwrap_or(Value::Null))
    }
}

impl Drop for Element {
    fn drop(&mut self) {
        let page = self.page.clone();
        let id = self.id.clone();
        runtime::runtime().spawn(async move {
            let _ = page.execute(ReleaseObjectParams::new(id)).await;
        });
    }
}

fn normalized(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl Query {
    pub async fn resolve(&self, page: &chromiumoxide::Page) -> anyhow::Result<Matches> {
        match self {
            Self::Role { role, name, exact } => {
                let root = page.get_document().await?;
                let nodes = page
                    .execute(
                        QueryAxTreeParams::builder()
                            .node_id(root.node_id)
                            .role(role.clone())
                            .build(),
                    )
                    .await?
                    .result
                    .nodes;
                let name = name.as_deref().map(normalized);
                let nodes: Vec<_> = nodes
                    .into_iter()
                    .filter(|node| {
                        if node.ignored {
                            return false;
                        }
                        match &name {
                            None => true,
                            Some(name) => {
                                let actual = node
                                    .name
                                    .as_ref()
                                    .and_then(|name| name.value.as_ref())
                                    .and_then(Value::as_str)
                                    .map(normalized)
                                    .unwrap_or_default();
                                if *exact {
                                    actual == *name
                                } else {
                                    actual.to_lowercase().contains(&name.to_lowercase())
                                }
                            }
                        }
                    })
                    .collect();
                if nodes.len() != 1 {
                    return Ok(Matches {
                        count: nodes.len(),
                        element: None,
                    });
                }
                let backend = nodes[0]
                    .backend_dom_node_id
                    .context("Accessibility match has no DOM node")?;
                let object = page
                    .execute(
                        ResolveNodeParams::builder()
                            .backend_node_id(backend)
                            .build(),
                    )
                    .await?
                    .result
                    .object;
                Ok(Matches {
                    count: 1,
                    element: Some(Element {
                        page: page.clone(),
                        id: object
                            .object_id
                            .context("Accessibility node is no longer attached")?,
                    }),
                })
            }
            Self::Css(selector) => {
                self.evaluate(
                    page,
                    format!(
                        "Array.from(document.querySelectorAll({}))",
                        serde_json::to_string(selector)?
                    ),
                )
                .await
            }
            Self::Text { text, exact } => {
                let expression = format!(
                    "({})({}, {})",
                    include_str!("text.js"),
                    serde_json::to_string(text)?,
                    exact
                );
                self.evaluate(page, expression).await
            }
        }
    }

    async fn evaluate(
        &self,
        page: &chromiumoxide::Page,
        elements: String,
    ) -> anyhow::Result<Matches> {
        // Return a DOM object only for a unique match; otherwise the count needs no remote handle.
        let expression = format!(
            "(() => {{ const matches = {elements}; return matches.length === 1 ? matches[0] : matches.length; }})()"
        );
        let result = page
            .evaluate_expression(
                EvaluateParams::builder()
                    .expression(expression)
                    .return_by_value(false)
                    .build()
                    .map_err(anyhow::Error::msg)?,
            )
            .await?;
        let object = result.object();
        if let Some(id) = &object.object_id {
            Ok(Matches {
                count: 1,
                element: Some(Element {
                    page: page.clone(),
                    id: id.clone(),
                }),
            })
        } else {
            Ok(Matches {
                count: object
                    .value
                    .as_ref()
                    .and_then(Value::as_u64)
                    .context("Locator did not return a match count")?
                    as usize,
                element: None,
            })
        }
    }
}
