//! gostc-rs tunnel-client: runs on an intranet machine.
//!
//! One process serves ALL tunnels configured in the web panel for its client
//! token. It keeps a pool of control channels connected to the tunnel-server
//! (reverse connections, so no inbound port is needed). When the server sends
//! `DIAL <local_addr>` over a channel, the client dials that local service
//! and relays it to the public port on the node.

use anyhow::{bail, Context, Result};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{error, info, warn};

const PROTO_PREFIX: &str = "GOSTC1";

#[derive(Clone)]
struct Args {
    server: String,
    token: String,
    pool: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let args = parse_args()?;
    info!(
        server = %args.server,
        pool = args.pool,
        "gostc-rs tunnel-client started; tunnels are managed in the web panel"
    );

    let mut handles = Vec::new();
    for i in 0..args.pool {
        let a = args.clone();
        handles.push(tokio::spawn(async move { worker(i, a).await }));
    }
    for h in handles {
        let _ = h.await;
    }
    Ok(())
}

/// Each worker maintains exactly one control channel at a time. The channel
/// stays idle (in the server's pool) until the server borrows it for a
/// public connection, then it serves exactly one relay and is replaced.
async fn worker(id: usize, args: Args) {
    let mut backoff = 2u64;
    loop {
        match run_once(&args).await {
            Ok(()) => {
                backoff = 2;
            }
            Err(e) => {
                warn!(worker = id, "channel error: {e:#}; retrying in {backoff}s");
                tokio::time::sleep(Duration::from_secs(backoff)).await;
                backoff = (backoff * 2).min(60);
            }
        }
    }
}

async fn run_once(args: &Args) -> Result<()> {
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
    info!("control channel ready, waiting for dial requests");

    // Wait until the server borrows this channel for a public connection.
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
    let mut server = String::new();
    let mut token = String::new();
    let mut pool = 4usize;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--server" => server = it.next().unwrap_or_default(),
            "--token" => token = it.next().unwrap_or_default(),
            "--pool" => pool = it.next().and_then(|v| v.parse().ok()).unwrap_or(4),
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
    if server.is_empty() || token.is_empty() {
        print_help();
        bail!("--server and --token are required");
    }
    Ok(Args {
        server,
        token,
        pool,
    })
}

fn print_help() {
    eprintln!(
        "gostc-rs tunnel-client\n\n\
         USAGE:\n    \
         gostc-rs-tunnel-client --server <node-host:port> --token <client-token> [options]\n\n\
         OPTIONS:\n    \
         --server <addr>    Node tunnel endpoint (host:port shown in the panel)\n    \
         --token <token>    Client token shown when creating the client in the panel\n    \
         --pool <n>         Idle control channels to keep (default: 4)\n\n\
         NOTE: tunnels are created/managed in the web panel; local addresses\n    \
         are delivered by the server, no per-tunnel flags needed."
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
