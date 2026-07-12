//! Connects to a running gpservice, performs the ping/pong handshake, and
//! exchanges encrypted frames. The read loop forwards decoded `WsEvent`s on a
//! channel; `Handle` lets callers send `WsRequest`s.

use anyhow::{Context, Result, ensure};
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::crypto::Crypto;
use gp_protocol::{WsEvent, WsRequest};

/// Written by gpservice at startup; format is `pid:port`.
const LOCK_FILE: &str = "/var/run/gpservice.lock";

/// Read the loopback port gpservice is listening on.
pub async fn read_port() -> Result<u16> {
  let content = tokio::fs::read_to_string(LOCK_FILE)
    .await
    .with_context(|| format!("reading {LOCK_FILE} — is gpservice running?"))?;
  let port = content
    .trim()
    .split(':')
    .nth(1)
    .context("lock file not in `pid:port` format")?
    .parse()
    .context("parsing port")?;
  Ok(port)
}

/// Obtain the 32-byte shared key.
///
/// - `--api-key-on-stdin`: read base64 from stdin, exactly as gpservice feeds
///   the GUI it launches.
/// - otherwise: dev fallback of all-zeros, matching `gpservice --no-gui`
///   (debug build), so the GUI can be run standalone during development.
///
/// Used on the `gpservice`-launched handshake path; kept even when unused in
/// standalone dev runs.
#[allow(dead_code)]
pub async fn load_api_key(from_stdin: bool) -> Result<Vec<u8>> {
  if from_stdin {
    let mut buf = String::new();
    tokio::io::stdin().read_to_string(&mut buf).await?;
    let key = base64::engine::general_purpose::STANDARD
      .decode(buf.trim())
      .context("decoding base64 api key")?;
    ensure!(key.len() == 32, "api key must be 32 bytes, got {}", key.len());
    Ok(key)
  } else {
    tracing::warn!("no --api-key-on-stdin: using dev zero-key (works with `gpservice --no-gui`)");
    Ok(vec![0u8; 32])
  }
}

/// Handle for sending requests to the service.
#[derive(Clone)]
pub struct Handle {
  tx: mpsc::Sender<WsRequest>,
}

impl Handle {
  pub async fn send(&self, req: WsRequest) -> Result<()> {
    self.tx.send(req).await.context("service channel closed")?;
    Ok(())
  }
}

/// Connect and run the read/write loops until the socket closes.
///
/// Returns a `Handle` for outbound requests and a receiver of inbound events.
/// The driving task is spawned onto the current tokio runtime.
pub async fn connect(port: u16, key: Vec<u8>) -> Result<(Handle, mpsc::Receiver<WsEvent>)> {
  let url = format!("ws://127.0.0.1:{port}/ws");
  let (ws, _) = tokio_tungstenite::connect_async(&url)
    .await
    .with_context(|| format!("connecting to {url}"))?;
  tracing::info!("connected to {url}");

  let crypto = Crypto::new(key);
  let (mut sink, mut stream) = ws.split();

  let (event_tx, event_rx) = mpsc::channel::<WsEvent>(32);
  let (req_tx, mut req_rx) = mpsc::channel::<WsRequest>(32);

  tokio::spawn(async move {
    loop {
      tokio::select! {
        // Outbound: encrypt requests and send as binary frames.
        Some(req) = req_rx.recv() => {
          match crypto.encrypt(&req) {
            Ok(frame) => {
              if let Err(e) = sink.send(Message::Binary(frame.into())).await {
                tracing::error!("send failed: {e}");
                break;
              }
            }
            Err(e) => tracing::error!("encrypt failed: {e}"),
          }
        }
        // Inbound: handle handshake ping, decrypt events.
        msg = stream.next() => {
          let Some(msg) = msg else { break };
          let msg = match msg {
            Ok(m) => m,
            Err(e) => { tracing::error!("recv failed: {e}"); break; }
          };
          match msg {
            Message::Ping(_) => {
              // Handshake: server pings "Hi" and waits for exactly one client
              // frame. tungstenite auto-queues the Pong reply — just flush it.
              // Do NOT send our own Pong: gpservice's recv loop treats any
              // non-Binary frame as fatal, so a duplicate pong drops the
              // connection (the bug that closed us mid-connect).
              if let Err(e) = sink.flush().await {
                tracing::error!("pong flush failed: {e}");
                break;
              }
            }
            Message::Binary(data) => match crypto.decrypt::<WsEvent>(&data) {
              Ok(ev) => {
                if event_tx.send(ev).await.is_err() {
                  break; // receiver dropped
                }
              }
              Err(e) => tracing::warn!("bad frame: {e}"),
            },
            Message::Close(_) => {
              tracing::info!("service closed connection");
              break;
            }
            _ => {}
          }
        }
        else => break,
      }
    }
    tracing::info!("client loop ended");
  });

  Ok((Handle { tx: req_tx }, event_rx))
}
