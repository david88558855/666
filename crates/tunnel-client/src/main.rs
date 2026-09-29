//! gostc-rs tunnel-client: runs on an intranet machine.
//!
//! gostc-style connection model: the client only knows the CENTRAL CONSOLE
//! address and its client token -- nodes may be created before or after the
//! client, in any order. Periodically it asks the console which node tunnel
//! endpoints to connect to (derived from its active tunnels), and keeps a
//! pool of reverse control channels to each node. When a node sends
//! `DIAL <local_addr>` over a channel, the client dials that local service
//! and relays it to the public port on the node.

mod api_client;

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio::sync::watch;
use tracing::{error, info, warn};

const PROTO_PREFIX: &str = "GOSTC1";

#[derive(Clone)]
struct Args {
    api: String,
    token: String,
    pool: usize,
    interval: u64,
}

#[derive(Clone)]
struct WorkerArgs {
    server: String,
    token: String,
}

#[derive(Debug, Deserialize)]
struct ConnectNode {
    #[allow(dead_code)]
    id: i64,
    tunnel_endpoint: String,
}

#[derive(Debug, Deserialize)]
struct ConnectResponse {
    #[allow(dead_code)]
    client_id: i64,
    #[allow(dead_code)]
    name: String,
    nodes: Vec<ConnectNode>,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let args = parse_args()?;
    info!(
        api = %args.api,
        pool = args.pool,
        "gostc-rs tunnel-client started; tunnels are managed in the web panel"
    );

    // The console pushes node endpoint updates through this channel;
    // the spawner restarts the control channel pool accordingly.
    let (endpoint_tx, endpoint_rx) = watch::channel(Vec::<String>::new());
    let spawner = tokio::spawn(spawn_workers(endpoint_rx, args.token.clone(), args.pool));

    let tick = Duration::from_secs(args.interval);
    let mut last = String::new();
    loop {
        match resolve_endpoints(&args).await {
            Ok(endpoints) => {
                let joined = endpoints.join("|");
                if joined != last {
                    info!(endpoints = %if joined.is_empty() { "(none yet)" } else { &joined },
                          "node endpoints updated");
                    let _ = endpoint_tx.send(endpoints);
                    last = joined;
                }
            }
            Err(e) => warn!("connect to console failed: {e:#}"),
        }
        tokio::time::sleep(tick).await;
    }
    #[allow(unreachable_code)]
    {
        let _ = spawner.abort();
    }
    Ok(())
}

async fn resolve_endpoints(args: &Args) -> Result<Vec<String>> {
    let value = api_client::post_json(
        &args.api,
        "/clients/connect",
        &json!({ "token": args.token }),
    )
    .await
    .context("connect to console")?;
    let resp: ConnectResponse = serde_json::from_value(value).context("parse connect response")?;
    Ok(resp
        .nodes
        .into_iter()
        .map(|n| n.tunnel_endpoint)
        .collect())
}

/// Maintain one pool of control channels per node endpoint. Any endpoint
/// change (tunnel added on a new node, endpoint edited, ...) restarts the
/// pools; the old channels are simply re-established.
async fn spawn_workers(mut rx: watch::Receiver<Vec<String>>, token: String, pool: usize) {
    let mut handles: Vec<JoinHandle<()>> = Vec::new();
    loop {
        if rx.changed().await.is_err() {
            break;
        }
        let servers = rx.borrow_and_update().clone();
        for h in handles.drain(..) {
            h.abort();
        }
        if servers.is_empty() {
            info!("no active tunnels yet; waiting for panel configuration");
            continue;
        }
        info!(servers = ?servers, pool, "opening control channel pools");
        for server in &servers {
            for i in 0..pool {
                let a = WorkerArgs {
                    server: server.clone(),
                    token: token.clone(),
                };
                handles.push(tokio::spawn(worker(i, a)));
            }
        }
    }
}

/// Each worker maintains exactly one control channel at a time. The channel
/// stays idle (in the node's pool) until the node borrows it for a public
/// connection, then it serves exactly one relay and is replaced.
async fn worker(id: usize, args: WorkerArgs) {
    let mut backoff = 2u64;
    loop {
        match run_once(&args).await {
            Ok(()) => {
                backoff = 2;
            }
            Err(e) => {
                warn!(worker = id, server = %args.server, "channel error: {e:#}; retrying in {backoff}s");
                tokio::time::sleep(Duration::from_secs(backoff)).await;
                backoff = (backoff * 2).min(60);
            }
        }
    }
}

async fn run_once(args: &WorkerArgs) -> Result<()> {
    let mut stream = TcpStream::connect(&args.server)
        .await
        .with_context(|| format!("connect {}", args.server))?;
    stream
        .write_all(format!("{PROTO_PREFIX} {}\n", args.token).as_bytes())
        .await?;
    let reply = read_line(&mut stream).await?;
    if reply.trim() != "OK" {
        bail!("server rejected auth: {}", reply.trim());
    }
    info!(server = %args.server, "control channel ready, waiting for dial requests");

    // Wait until the node borrows this channel for a public connection.
    let line = read_line(&mut stream).await?;
    let mut parts = line.trim().splitn(2, ' ');
    let cmd = parts.next().unwrap_or("");
    let local_addr = parts.next().unwrap_or("");
    if cmd != "DIAL" || local_addr.is_empty() {
        bail!("unexpected server message: {:?}", line.trim());
    }

    info!(local = %local_addr, "dial request, connecting to local service");
    let mut local = TcpStream::connect(local_addr)
        .await
        .with_context(|| format!("connect local service {local_addr}"))?;
    match tokio::io::copy_bidirectional(&mut stream, &mut local).await {
        Ok((up, down)) => info!(local = %local_addr, up, down, "relay finished"),
        Err(e) => error!(local = %local_addr, "relay error: {e}"),
    }
    Ok(())
}

/// Read one `\n`-terminated line reading a single byte at a time so no
/// extra bytes are consumed beyond the line (the socket switches to raw
/// relaying right after).
async fn read_line(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut buf = Vec::with_capacity(128);
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

fn parse_args() -> Result<Args> {
    let mut api = String::new();
    let mut token = String::new();
    let mut pool = 4usize;
    let mut interval = 30u64;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--api" => api = it.next().unwrap_or_default(),
            "--token" => token = it.next().unwrap_or_default(),
            "--pool" => pool = it.next().and_then(|v| v.parse().ok()).unwrap_or(4),
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
    if api.is_empty() || token.is_empty() {
        print_help();
        bail!("--api and --token are required");
    }
    Ok(Args {
        api: api.trim_end_matches('/').to_string(),
        token,
        pool,
        interval,
    })
}

fn print_help() {
    eprintln!(
        "gostc-rs tunnel-client\n\n\
         USAGE:\n    \
         gostc-rs-tunnel-client --api <http://console-host:port> --token <client-token> [options]\n\n\
         OPTIONS:\n    \
         --api <url>        Central console address (http only)\n    \
         --token <token>    Client token shown when creating the client in the panel\n    \
         --pool <n>         Idle control channels per node (default: 4)\n    \
         --interval <secs>  Console polling interval (default: 30)\n\n\
         NOTE: nodes and clients can be started in ANY order; tunnels are\n    \
         created in the web panel and applied automatically."
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
