//! The relay over real WebSockets: tokens, channels, publishing, origins and limits.

use std::{
    net::SocketAddr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures_util::{SinkExt, StreamExt};
use jsonwebtoken::{encode, EncodingKey, Header};
use ouril_protocol::{RelayClaims, RELAY_AUDIENCE, RELAY_ISSUER};
use ouril_relay::{app, Config};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        client::IntoClientRequest, http::HeaderValue, protocol::frame::coding::CloseCode, Message,
    },
    MaybeTlsStream, WebSocketStream,
};

const KEY: &str = "publish-key-for-relay-tests-0123456789abcdef";
const SECRET: &str = "token-secret-for-relay-tests-0123456789abcd";
const ORIGIN: &str = "http://localhost:5173";
const GAME: &str = "game:01a137aa-33cf-7667-a7bb-d673a70b3b19";
const OTHER_GAME: &str = "game:01a137aa-33cf-7667-a7bb-000000000000";
const USER: &str = "user:01a137aa-33cb-764c-8eb5-409f541e78da";

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

fn config() -> Config {
    let mut c = Config::from_lookup(|k| match k {
        "RELAY_API_KEY" => Some(KEY.into()),
        "RELAY_TOKEN_SECRET" => Some(SECRET.into()),
        "ALLOWED_ORIGINS" => Some(ORIGIN.into()),
        _ => None,
    })
    .unwrap();
    c.auth_timeout = Duration::from_millis(300);
    c
}

async fn start(config: Config) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = app(config);
    tokio::spawn(async move {
        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap()
    });
    addr
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn token_with(secret: &str, aud: &str, exp: u64, ch: &[&str]) -> String {
    let claims = RelayClaims {
        iss: RELAY_ISSUER.into(),
        aud: aud.into(),
        sub: "01a137aa-33cb-764c-8eb5-409f541e78da".into(),
        iat: now(),
        exp,
        ch: ch.iter().map(|c| c.to_string()).collect(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap()
}

fn token(ch: &[&str]) -> String {
    token_with(SECRET, RELAY_AUDIENCE, now() + 600, ch)
}

async fn open(addr: SocketAddr, origin: Option<&str>) -> Result<Ws, u16> {
    let mut req = format!("ws://{addr}/ws").into_client_request().unwrap();
    if let Some(o) = origin {
        req.headers_mut()
            .insert("origin", HeaderValue::from_str(o).unwrap());
    }
    match connect_async(req).await {
        Ok((ws, _)) => Ok(ws),
        Err(tokio_tungstenite::tungstenite::Error::Http(res)) => Err(res.status().as_u16()),
        Err(e) => panic!("{e}"),
    }
}

/// Open, authenticate, and wait for `ready`.
async fn subscribe(addr: SocketAddr, ch: &[&str]) -> Ws {
    let mut ws = open(addr, Some(ORIGIN)).await.unwrap();
    ws.send(Message::text(json!({ "token": token(ch) }).to_string()))
        .await
        .unwrap();
    assert_eq!(next_json(&mut ws).await, json!({ "type": "ready" }));
    ws
}

/// The next text message as JSON (skipping pings), or panic after a second.
async fn next_json(ws: &mut Ws) -> Value {
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(1), ws.next())
            .await
            .expect("a message in time")
            .expect("open")
            .expect("ok");
        match msg {
            Message::Text(t) => return serde_json::from_str(&t).unwrap(),
            Message::Ping(_) | Message::Pong(_) => continue,
            other => panic!("unexpected {other:?}"),
        }
    }
}

/// Wait for the server to close; returns its close code.
async fn close_code(ws: &mut Ws) -> Option<u16> {
    loop {
        match tokio::time::timeout(Duration::from_secs(3), ws.next())
            .await
            .expect("closed in time")
        {
            Some(Ok(Message::Close(frame))) => return frame.map(|f| u16::from(f.code)),
            Some(Ok(_)) => continue,
            _ => return None,
        }
    }
}

async fn publish(addr: SocketAddr, key: Option<&str>, body: Value) -> u16 {
    let mut req = reqwest_like(addr, body);
    if let Some(k) = key {
        req = req.replace("AUTH", &format!("Authorization: Bearer {k}\r\n"));
    } else {
        req = req.replace("AUTH", "");
    }
    raw_http(addr, &req).await
}

/// A tiny HTTP/1.1 client, to avoid pulling in a whole HTTP crate for tests.
fn reqwest_like(addr: SocketAddr, body: Value) -> String {
    let body = body.to_string();
    format!(
        "POST /publish HTTP/1.1\r\nHost: {addr}\r\nAUTHContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

async fn raw_http(addr: SocketAddr, request: &str) -> u16 {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut s = TcpStream::connect(addr).await.unwrap();
    s.write_all(request.as_bytes()).await.unwrap();
    let mut buf = Vec::new();
    s.read_to_end(&mut buf).await.unwrap();
    let head = String::from_utf8_lossy(&buf);
    head.split_whitespace().nth(1).unwrap().parse().unwrap()
}

fn event(channel: &str) -> Value {
    json!({
        "channel": channel,
        "event": { "type": "game_changed", "game_id": "g", "moves": 3 }
    })
}

#[tokio::test]
async fn only_the_three_paths_answer() {
    let addr = start(config()).await;
    let get =
        |path: &str| format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    assert_eq!(raw_http(addr, &get("/healthz")).await, 200);
    assert_eq!(raw_http(addr, &get("/admin")).await, 404);
    assert_eq!(raw_http(addr, &get("/")).await, 404);
}

#[tokio::test]
async fn players_get_only_their_channels_events() {
    let addr = start(config()).await;
    let mut ws = subscribe(addr, &[USER, GAME]).await;
    assert_eq!(publish(addr, Some(KEY), event(OTHER_GAME)).await, 204);
    assert_eq!(publish(addr, Some(KEY), event(GAME)).await, 204);
    // The other game's event never arrives; this one does, as the event alone.
    assert_eq!(
        next_json(&mut ws).await,
        json!({ "type": "game_changed", "game_id": "g", "moves": 3 })
    );
    ws.send(Message::text(r#"{"type":"ping"}"#)).await.unwrap();
    assert_eq!(next_json(&mut ws).await, json!({ "type": "pong" }));
}

#[tokio::test]
async fn publishing_needs_the_key_and_a_valid_channel() {
    let addr = start(config()).await;
    assert_eq!(publish(addr, None, event(GAME)).await, 401);
    assert_eq!(publish(addr, Some("wrong-key"), event(GAME)).await, 401);
    assert_eq!(
        publish(addr, Some(SECRET), event(GAME)).await,
        401,
        "not the token secret"
    );
    assert_eq!(publish(addr, Some(KEY), event("game:../../etc")).await, 400);
    assert_eq!(publish(addr, Some(KEY), event("admin:x")).await, 400);
    assert_eq!(
        publish(
            addr,
            Some(KEY),
            json!({ "channel": GAME, "event": { "type": "anything" } })
        )
        .await,
        400,
        "only known events"
    );
    let huge =
        json!({ "channel": GAME, "pad": "x".repeat(10_000), "event": { "type": "games_changed" } });
    assert_eq!(publish(addr, Some(KEY), huge).await, 413);
}

#[tokio::test]
async fn upgrades_need_an_allowed_origin() {
    let addr = start(config()).await;
    assert_eq!(open(addr, None).await.err(), Some(403));
    assert_eq!(
        open(addr, Some("https://evil.example")).await.err(),
        Some(403)
    );
    assert!(open(addr, Some(ORIGIN)).await.is_ok());
}

#[tokio::test]
async fn bad_late_or_missing_tokens_are_closed() {
    let addr = start(config()).await;
    for token in [
        token_with(
            "another-secret-0123456789abcdefghijklmn",
            RELAY_AUDIENCE,
            now() + 600,
            &[GAME],
        ),
        token_with(SECRET, "ouril-api", now() + 600, &[GAME]),
        token_with(SECRET, RELAY_AUDIENCE, now() - 10, &[GAME]),
        token_with(SECRET, RELAY_AUDIENCE, now() + 600, &["game:not-a-uuid"]),
        token_with(SECRET, RELAY_AUDIENCE, now() + 600, &[]),
        token_with(
            SECRET,
            RELAY_AUDIENCE,
            now() + 600,
            &[GAME, GAME, GAME, GAME, GAME],
        ),
        "not a jwt".to_string(),
    ] {
        let mut ws = open(addr, Some(ORIGIN)).await.unwrap();
        ws.send(Message::text(json!({ "token": token }).to_string()))
            .await
            .unwrap();
        assert_eq!(close_code(&mut ws).await, Some(4003));
    }
    // Nothing sent in time.
    let mut ws = open(addr, Some(ORIGIN)).await.unwrap();
    assert_eq!(close_code(&mut ws).await, Some(4003));
}

#[tokio::test]
async fn players_cannot_send_anything_but_pings() {
    let addr = start(config()).await;
    let mut ws = subscribe(addr, &[GAME]).await;
    ws.send(Message::text(
        json!({ "channel": GAME, "event": { "type": "games_changed" } }).to_string(),
    ))
    .await
    .unwrap();
    assert_eq!(
        close_code(&mut ws).await,
        Some(u16::from(CloseCode::Policy))
    );
}

#[tokio::test]
async fn the_connection_closes_when_its_token_expires() {
    let addr = start(config()).await;
    let mut ws = open(addr, Some(ORIGIN)).await.unwrap();
    ws.send(Message::text(
        json!({ "token": token_with(SECRET, RELAY_AUDIENCE, now() + 1, &[GAME]) }).to_string(),
    ))
    .await
    .unwrap();
    assert_eq!(next_json(&mut ws).await, json!({ "type": "ready" }));
    assert_eq!(close_code(&mut ws).await, Some(4001));
}

#[tokio::test]
async fn connections_per_ip_are_limited() {
    let mut c = config();
    c.max_connections_per_ip = 2;
    let addr = start(c).await;
    let _a = open(addr, Some(ORIGIN)).await.unwrap();
    let _b = open(addr, Some(ORIGIN)).await.unwrap();
    assert_eq!(open(addr, Some(ORIGIN)).await.err(), Some(429));
    drop(_a);
    // The slot frees once the server sees the connection end.
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(open(addr, Some(ORIGIN)).await.is_ok());
}

#[tokio::test]
async fn upgrades_per_minute_are_limited() {
    let mut c = config();
    c.upgrades_per_minute_per_ip = 2;
    let addr = start(c).await;
    assert!(open(addr, Some(ORIGIN)).await.is_ok());
    assert!(open(addr, Some(ORIGIN)).await.is_ok());
    assert_eq!(open(addr, Some(ORIGIN)).await.err(), Some(429));
}
