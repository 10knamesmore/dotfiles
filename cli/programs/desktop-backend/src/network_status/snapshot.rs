//! Connection snapshot in the shape the Quickshell network service parses.

use std::fmt;

use serde::Serialize;
use serde::ser::Serializer;

/// What backs the default route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionKind {
    Wifi,
    Ethernet,
    Disconnected,
}

impl ConnectionKind {
    /// Name of the connection type in the JSON snapshot.
    fn as_str(self) -> &'static str {
        match self {
            Self::Wifi => "wifi",
            Self::Ethernet => "ethernet",
            Self::Disconnected => "disconnected",
        }
    }
}

impl fmt::Display for ConnectionKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for ConnectionKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// One published status line; the field names are the JSON keys of the interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub connection_type: ConnectionKind,
    pub interface_name: String,
    pub address: String,
    pub ssid: String,
    pub signal_strength: u8,
}

impl Snapshot {
    /// State published when no interface owns a default IPv4 route.
    pub fn disconnected() -> Self {
        Self {
            connection_type: ConnectionKind::Disconnected,
            interface_name: String::new(),
            address: String::new(),
            ssid: String::new(),
            signal_strength: 0,
        }
    }

    /// Report `interface_name` as ethernet, the kind assumed when its details are unknown.
    pub fn ethernet(interface_name: &str) -> Self {
        Self {
            connection_type: ConnectionKind::Ethernet,
            interface_name: interface_name.to_owned(),
            address: String::new(),
            ssid: String::new(),
            signal_strength: 0,
        }
    }
}
