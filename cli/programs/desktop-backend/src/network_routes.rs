//! IPv4 interface metadata shared by the network status service and traffic panel.

use std::net::{IpAddr, Ipv4Addr};

use anyhow::{Context, Result};
use futures_util::{StreamExt, TryStreamExt};
use rtnetlink::{
    Handle, MulticastGroup, RouteMessageBuilder,
    packet_route::{
        AddressFamily,
        address::AddressAttribute,
        link::LinkAttribute,
        route::{RouteAddress, RouteAttribute, RouteHeader, RouteMessage, RouteType},
    },
};
use rustc_hash::FxHashMap;
use tokio::{sync::watch, task::JoinHandle};

pub struct InterfaceRouting {
    pub name: String,
    pub ipv4: String,
    pub gateway: String,
}

pub struct RoutingSnapshot {
    pub default_interface: Option<String>,
    pub interfaces: Vec<InterfaceRouting>,
}

/// One owned netlink connection, subscribed before queries begin. Notifications are coalesced
/// independently of requests so querying a snapshot cannot block event consumption.
pub struct NetworkRoutes {
    handle: Handle,
    changes: watch::Receiver<()>,
    connection: JoinHandle<()>,
    notifications: JoinHandle<()>,
}

impl NetworkRoutes {
    pub fn connect() -> Result<Self> {
        let (connection, handle, mut messages) = rtnetlink::new_multicast_connection(&[
            MulticastGroup::Link,
            MulticastGroup::Ipv4Ifaddr,
            MulticastGroup::Ipv4Route,
        ])?;
        let connection = tokio::spawn(connection);
        let (changed, changes) = watch::channel(());
        let notifications = tokio::spawn(async move {
            while messages.next().await.is_some() {
                changed.send_replace(());
            }
        });
        Ok(Self {
            handle,
            changes,
            connection,
            notifications,
        })
    }

    pub fn has_changed(&self) -> Result<bool> {
        self.changes
            .has_changed()
            .context("netlink event stream closed")
    }

    pub async fn changed(&mut self) -> Result<()> {
        self.changes
            .changed()
            .await
            .context("netlink event stream closed")
    }

    pub async fn snapshot(&mut self) -> Result<RoutingSnapshot> {
        // A notification arriving during these queries remains pending for the next refresh.
        self.changes.borrow_and_update();
        let mut interfaces = FxHashMap::default();
        let mut links = self.handle.link().get().execute();
        while let Some(link) = links.try_next().await? {
            let name = link
                .attributes
                .into_iter()
                .find_map(|attribute| match attribute {
                    LinkAttribute::IfName(name) => Some(name),
                    _ => None,
                })
                .context("netlink interface has no name")?;
            interfaces.insert(
                link.header.index,
                InterfaceRouting {
                    name,
                    ipv4: String::new(),
                    gateway: String::new(),
                },
            );
        }

        let mut addresses = self.handle.address().get().execute();
        while let Some(address) = addresses.try_next().await? {
            if address.header.family != AddressFamily::Inet {
                continue;
            }
            let Some(interface) = interfaces.get_mut(&address.header.index) else {
                continue;
            };
            // IFA_LOCAL is the local address; IFA_ADDRESS may name the peer on point-to-point links.
            if interface.ipv4.is_empty()
                && let Some(local) = address
                    .attributes
                    .iter()
                    .find_map(|attribute| match attribute {
                        AddressAttribute::Local(IpAddr::V4(address)) => Some(address),
                        _ => None,
                    })
            {
                interface.ipv4 = local.to_string();
            }
        }

        let mut best_routes: FxHashMap<u32, DefaultRoute> = FxHashMap::default();
        let mut default: Option<(u32, u32)> = None;
        let request = RouteMessageBuilder::<Ipv4Addr>::new().build();
        let mut routes = self.handle.route().get(request).execute();
        while let Some(route) = routes.try_next().await? {
            let Some(route) = DefaultRoute::from_message(&route) else {
                continue;
            };
            if !interfaces.contains_key(&route.interface_index) {
                continue;
            }
            if default.is_none_or(|(_, metric)| route.metric < metric) {
                default = Some((route.interface_index, route.metric));
            }
            let existing = best_routes.entry(route.interface_index).or_insert(route);
            if route.metric < existing.metric {
                *existing = route;
            }
        }
        for (index, route) in best_routes {
            if let Some(interface) = interfaces.get_mut(&index) {
                interface.gateway = route.gateway.to_string();
            }
        }
        let default_interface = default
            .and_then(|(index, _)| interfaces.get(&index))
            .map(|interface| interface.name.clone());
        Ok(RoutingSnapshot {
            default_interface,
            interfaces: interfaces.into_values().collect(),
        })
    }
}

impl Drop for NetworkRoutes {
    fn drop(&mut self) {
        self.notifications.abort();
        self.connection.abort();
    }
}

#[derive(Clone, Copy)]
struct DefaultRoute {
    interface_index: u32,
    metric: u32,
    gateway: Ipv4Addr,
}

impl DefaultRoute {
    fn from_message(message: &RouteMessage) -> Option<Self> {
        if message.header.address_family != AddressFamily::Inet
            || message.header.destination_prefix_length != 0
            || message.header.kind != RouteType::Unicast
        {
            return None;
        }
        let mut table = u32::from(message.header.table);
        let mut interface_index = None;
        let mut metric = 0;
        let mut gateway = Ipv4Addr::UNSPECIFIED;
        for attribute in &message.attributes {
            match attribute {
                RouteAttribute::Table(value) => table = *value,
                RouteAttribute::Oif(value) => interface_index = Some(*value),
                RouteAttribute::Priority(value) => metric = *value,
                RouteAttribute::Gateway(RouteAddress::Inet(value)) => gateway = *value,
                _ => {}
            }
        }
        // The desktop displays the main-table default route, not policy-routing decisions.
        if table != u32::from(RouteHeader::RT_TABLE_MAIN) {
            return None;
        }
        Some(Self {
            interface_index: interface_index?,
            metric,
            gateway,
        })
    }
}
