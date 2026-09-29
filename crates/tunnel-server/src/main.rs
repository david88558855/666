//! gostc-rs tunnel-server: runs on a node with a public IP.
//!
//! Flow (gostc-style UX):
//! 1. Registers with the admin API using the node secret (and re-registers
//!    periodically as a heartbeat, which also picks up config changes).
//! 2. Opens a control listener on the node's `tunnel_endpoint` port.
//! 3. For every active TCP tunnel of its clients, opens a public listener on
//!    `remote_port`. Tunnels are configured entirely in the web panel.
//! 4. tunnel-clients authenticate with their client token and keep a pool of
//!    idle control channels alive. Each incoming public connection borrows
//!    one idle channel and sends `DIAL <local_addr>`; the client dials the
//!    local service and the connection is relayed 1:1.

mod api_client;

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{error, info, warn};

const PROTO_PREFIX: &str = "GOSTC1";
const CHANNEL_CAPACITY: usize = 16;
const IDLE_WAIT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Deserialize)]
struct TunnelInfo {
    id: i64,
    name: String,
    #[serde(rename = "type")]
    kind: String,
    local_addr: String,
    remote_port: Option<i64>,
    status: String,
}

#[derive(Debug, Deserialize)]
struct ClientInfo {
    #[allow(dead_code)]
    name: String,
    token: String,
    status: String,
    tunnels: Vec<TunnelInfo>,
}

#[derive(Debug, Deserialize)]
struct RegisterResponse {
    #[allow(dead_code)]
    node_id: i64,
    #[allow(dead_code)]
    name: String,
    tunnel_endpoint: String,
    clients: Vec<ClientInfo>,
}

/// Per-tunnel runtime entry: which client pool serves it, the local address
/// to dial, and the listener task handle (aborted when the tunnel goes away).
struct TunnelEntry {
    name: String,
    #[allow(dead_code)]
    client_token: String,
    #[allow(dead_code)]
    local_addr: String,
    listener: tokio::task::JoinHandle<()>,
}

type PoolRx = Arc<Mutex<mpsc::Receiver<TcpStream>>>;

#[derive(Clone)]
struct Shared {
    /// client token -> sender used by handle_control to hand over channels
    senders: Arc<RwLock<HashMap<String, mpsc::Sender<TcpStream>>>>,
    /// client token -> shared receiver used by tunnel listeners to borrow
    /// idle channels (tokio Mutex so it can be held across recv().await)
    receivers: Arc<RwLock<HashMap<String, PoolRx>>>,
    /// tunnel id -> runtime entry
    tunnels: Arc<RwLock<HashMap<i64, TunnelEntry>>>,
}

impl Shared {
    fn new() -> Self {
        Self {
            senders: Arc::new(RwLock::new(HashMap::new())),
            receivers: Arc::new(RwLock::new(HashMap::new())),
            tunnels: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

struct Args {
    api: String,
    secret: String,
    interval: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let args = parse_args()?;
    let shared = Shared::new();

    // First registration: brings the node online and returns its client and
    // tunnel assignments plus the tunnel_endpoint this node must listen on.
    let reg = register(&args).await?;
    info!(
        node_id = reg.node_id,
        name = %reg.name,
        clients = reg.clients.len(),
        "registered with admin api"
    );
    apply_tunnels(&shared, &reg).await;

    // Control listener for tunnel-clients (reverse connections).
    let port = port_of(&reg.tunnel_endpoint);
    let ctrl = TcpListener::bind(("0.0.0.0", port))
        .await
        .with_context(|| format!("bind control listener 0.0.0.0:{port}"))?;
    info!(port, "control listener ready (tunnel-clients connect here)");

    let shared_ctrl = shared.clone();
    tokio::spawn(async move {
        loop {
            match ctrl.accept().await {
                Ok((stream, peer)) => {
                    tokio::spawn(handle_control(shared_ctrl.clone(), stream, peer));
                }
                Err(e) => {
                    error!("control accept failed: {e}");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });

    // Heartbeat: re-register periodically to keep the node online and to pick
    // up client/tunnel create/delete/enable/pause changes from the panel.
    info!(
        interval = args.interval,
        "gostc-rs tunnel-server started; press Ctrl+C to stop"
    );
    let tick = Duration::from_secs(args.interval);
    loop {
        tokio::time::sleep(tick).await;
        if let Err(e) = register_and_apply(&args, &shared).await {
            warn!("heartbeat failed: {e:#}");
        }
    }
}

async fn register(args: &Args) -> Result<RegisterResponse> {
    let value = api_client::post_json(
        &args.api,
        "/nodes/register",
        &json!({ "secret": args.secret }),
    )
    .await
    .context("register to admin api")?;
    serde_json::from_value(value).context("parse register response")
}

async fn register_and_apply(args: &Args, shared: &Shared) -> Result<()> {
    let reg = register(args).await?;
    apply_tunnels(shared, &reg).await;
    Ok(())
}

async fn apply_tunnels(shared: &Shared, reg: &RegisterResponse) {
    // Active client tokens.
    let active: HashSet<String> = reg
        .clients
        .iter()
        .filter(|c| c.status == "active")
        .map(|c| c.token.clone())
        .collect();

    // Desired tunnels: active client + active tcp tunnel with a remote port.
    let mut desired: HashMap<i64, (&ClientInfo, &TunnelInfo)> = HashMap::new();
    let mut used_ports: HashMap<i64, &TunnelInfo> = HashMap::new();
    for c in &reg.clients {
        if c.status != "active" {
            continue;
        }
        for t in &c.tunnels {
            if t.status != "active" || t.kind != "tcp" {
                continue;
            }
            let Some(port) = t.remote_port else {
                continue;
            };
            if let Some(prev) = used_ports.get(&port) {
                warn!(
                    tunnel = %t.name,
                    port,
                    tunnel_other = %prev.name,
                    "remote port conflict; keeping the first tunnel"
                );
                continue;
            }
            used_ports.insert(port, t);
            desired.insert(t.id, (c, t));
        }
    }

    // Sync client channel pools.
    {
        let mut senders = shared.senders.write().await;
        let mut receivers = shared.receivers.write().await;
        let stale: Vec<String> = senders
            .keys()
            .filter(|k| !active.contains(*k))
            .cloned()
            .collect();
        for k in stale {
            senders.remove(&k);
            receivers.remove(&k);
            info!(client = %k, "client channel pool removed");
        }
        for token in &active {
            if senders.contains_key(token) {
                continue;
            }
            let (tx, rx) = mpsc::channel::<TcpStream>(CHANNEL_CAPACITY);
            senders.insert(token.clone(), tx);
            receivers.insert(token.clone(), Arc::new(Mutex::new(rx)));
            info!(client = %token, "client channel pool created");
        }
    }

    // Sync public listeners per tunnel.
    let mut map = shared.tunnels.write().await;

    let stale: Vec<i64> = map
        .keys()
        .filter(|id| !desired.contains_key(id))
        .copied()
        .collect();
    for id in stale {
        if let Some(entry) = map.remove(&id) {
            entry.listener.abort();
            info!(tunnel = %entry.name, "public listener stopped");
        }
    }

    for (id, (c, t)) in &desired {
        if map.contains_key(id) {
            continue;
        }
        let id = *id;
        let port = t.remote_port.unwrap_or_default() as u16;
        let name = t.name.clone();
        let client_token = c.token.clone();
        let local_addr = t.local_addr.clone();
        let pool = {
            let receivers = shared.receivers.read().await;
            receivers.get(&client_token).cloned()
        };
        let listener = tokio::spawn(async move {
            let ln = match TcpListener::bind(("0.0.0.0", port)).await {
                Ok(l) => l,
                Err(e) => {
                    error!(tunnel = %name, port, "bind public port failed: {e}");
                    return;
                }
            };
            info!(tunnel = %name, port, "public listener started");
            loop {
                let (mut public, peer) = match ln.accept().await {
                    Ok(x) => x,
                    Err(e) => {
                        warn!(tunnel = %name, "accept failed: {e}");
                        continue;
                    }
                };
                // Borrow an idle client channel (clients replenish the pool).
                let idle = match &pool {
                    Some(p) => {
                        let mut guard = p.lock().await;
                        match tokio::time::timeout(IDLE_WAIT, guard.recv()).await {
                            Ok(Some(s)) => s,
                            _ => {
                                warn!(tunnel = %name, %peer,
                                      "no idle client channel; dropping connection");
                                continue;
                            }
                        }
                    }
                    None => {
                        warn!(tunnel = %name, "client pool unavailable");
                        continue;
                    }
                };
                let mut idle = idle;
                if idle
                    .write_all(format!("DIAL {local_addr}\n").as_bytes())
                    .await
                    .is_err()
                {
                    continue;
                }
                tokio::spawn(async move {
                    match tokio::io::copy_bidirectional(&mut public, &mut idle).await {
                        Ok((up, down)) => {
                            tracing::debug!(%peer, up, down, "relay finished");
                        }
                        Err(e) => tracing::debug!(%peer, "relay closed: {e}"),
                    }
                });
            }
        });
        map.insert(
            id,
            TunnelEntry {
                name: t.name.clone(),
                client_token: client_token.clone(),
                local_addr: local_addr.clone(),
                listener,
            },
        );
        info!(tunnel = %t.name, port, "public listener queued");
    }
}

async fn handle_control(shared: Shared, mut stream: TcpStream, peer: SocketAddr) {
    // Read the auth line byte-by-byte (never over-read into a buffer:
    // the connection switches to raw relaying right after).
    let line = match tokio::time::timeout(Duration::from_secs(10), read_line(&mut stream)).await {
        Ok(Ok(l)) => l,
        _ => return,
    };
    let mut parts = line.trim().splitn(2, ' ');
    let proto = parts.next().unwrap_or("");
    let token = parts.next().unwrap_or("").to_string();
    if proto != PROTO_PREFIX || token.is_empty() {
        let _ = stream.write_all(b"ERR bad protocol\n").await;
        return;
    }

    let tx = shared.senders.read().await.get(&token).cloned();
    let Some(tx) = tx else {
        let _ = stream.write_all(b"ERR unknown or paused client\n").await;
        warn!(peer = %peer, "client auth rejected (unknown/paused client)");
        return;
    };
    if stream.write_all(b"OK\n").await.is_err() {
        return;
    }
    info!(peer = %peer, "client control channel ready");
    // Hand the connection over to the client's pool; whichever tunnel gets a
    // public connection will borrow it and send `DIAL <local_addr>`.
    let _ = tx.send(stream).await;
}

/// Read one `\n`-terminated line reading a single byte at a time so no
/// extra bytes are consumed beyond the line.
async fn read_line(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut buf = Vec::with_capacity(64);
    let mut byte = [0u8; 1];
    loop {
        let n = stream.read(&mut byte).await?;
        if n == 0 || byte[0] == b'\n' {
            break;
        }
        buf.push(byte[0]);
        if buf.len() > 4096 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "line too long",
            ));
        }
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn port_of(endpoint: &str) -> u16 {
    endpoint
        .rsplit(':')
        .next()
        .and_then(|p| p.parse().ok())
        .unwrap_or(7502)
}

fn parse_args() -> Result<Args> {
    let mut api = String::new();
    let mut secret = String::new();
    let mut interval = 30u64;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--api" => api = it.next().unwrap_or_default(),
            "--secret" => secret = it.next().unwrap_or_default(),
            "--interval" => interval = it.next().and_then(|v| v.parse().ok()).unwrap_or(30),
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            other => {
                eprintln!("unknown argument: {other}");
                print_help();
                std::process::exit(2);
            }
        }
    }
    if api.is_empty() || secret.is_empty() {
        print_help();
        bail!("--api and --secret are required");
    }
    Ok(Args {
        api: api.trim_end_matches('/').to_string(),
        secret,
        interval,
    })
}

fn print_help() {
    eprintln!(
        "gostc-rs tunnel-server (node)\n\n\
         USAGE:\n    \
         gostc-rs-tunnel-server --api <http://admin-host:port> --secret <node-secret> [options]\n\n\
         OPTIONS:\n    \
         --api <url>        Admin API base URL (http only)\n    \
         --secret <secret>  Node secret shown when creating the node in the panel\n    \
         --interval <secs>  Heartbeat interval (default: 30)"
    );
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}
