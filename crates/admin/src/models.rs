use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    User,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::User => "user",
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Role {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "admin" => Ok(Role::Admin),
            "user" => Ok(Role::User),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TunnelType {
    Tcp,
    Udp,
    Http,
    Https,
    /// Secret TCP (frp-style STCP, self-built: no public port; visitor
    /// clients with the secret key reach the service client via the
    /// coordinator — P2P direct or node relay fallback).
    Stcp,
    /// Secret UDP (frp-style SUDP, same visitor model as STCP).
    Sudp,
    /// gostc-style P2P tunnel (frp xtcp semantics, self-built): service
    /// client registers node + intranet target + vKey (sk_hash); visitors
    /// connect via NAT traversal with node relay fallback. Direct-connection
    /// first, never occupies a public port.
    P2p,
}

impl TunnelType {
    pub fn as_str(self) -> &'static str {
        match self {
            TunnelType::Tcp => "tcp",
            TunnelType::Udp => "udp",
            TunnelType::Http => "http",
            TunnelType::Https => "https",
            TunnelType::Stcp => "stcp",
            TunnelType::Sudp => "sudp",
            TunnelType::P2p => "p2p",
        }
    }
}

impl FromStr for TunnelType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tcp" => Ok(Self::Tcp),
            "udp" => Ok(Self::Udp),
            "http" => Ok(Self::Http),
            "https" => Ok(Self::Https),
            "stcp" => Ok(Self::Stcp),
            "sudp" => Ok(Self::Sudp),
            "p2p" => Ok(Self::P2p),
            other => Err(format!("unknown tunnel type: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TunnelStatus {
    Active,
    Paused,
}

impl TunnelStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TunnelStatus::Active => "active",
            TunnelStatus::Paused => "paused",
        }
    }
}

impl FromStr for TunnelStatus {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    Online,
    Offline,
    Disabled,
}

impl NodeStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            NodeStatus::Online => "online",
            NodeStatus::Offline => "offline",
            NodeStatus::Disabled => "disabled",
        }
    }
}

impl FromStr for NodeStatus {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "online" => Ok(Self::Online),
            "offline" => Ok(Self::Offline),
            "disabled" => Ok(Self::Disabled),
            _ => Err(()),
        }
    }
}