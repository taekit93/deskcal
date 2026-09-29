use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::error::AppError;

pub const SCOPES: &str =
    "https://www.googleapis.com/auth/calendar.readonly https://www.googleapis.com/auth/tasks";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
}

impl OAuthConfig {
    pub fn from_build_env() -> Self {
        Self {
            client_id: option_env!("GOOGLE_CLIENT_ID").unwrap_or("").to_string(),
            client_secret: option_env!("GOOGLE_CLIENT_SECRET").unwrap_or("").to_string(),
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".into(),
            token_url: "https://oauth2.googleapis.com/token".into(),
        }
    }
}

pub trait TokenStore: Send + Sync {
    fn get(&self) -> Result<Option<String>, AppError>;
    fn set(&self, token: &str) -> Result<(), AppError>;
    fn delete(&self) -> Result<(), AppError>;
}

pub struct KeyringStore {
    entry: keyring::Entry,
}

impl KeyringStore {
    pub fn new() -> Result<Self, AppError> {
        keyring::Entry::new("gcal-widget", "google-refresh-token")
            .map(|entry| Self { entry })
            .map_err(|e| AppError::Storage(e.to_string()))
    }
}

impl TokenStore for KeyringStore {
    fn get(&self) -> Result<Option<String>, AppError> {
        match self.entry.get_password() {
            Ok(t) => Ok(Some(t)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(AppError::Storage(e.to_string())),
        }
    }
    fn set(&self, token: &str) -> Result<(), AppError> {
        self.entry.set_password(token).map_err(|e| AppError::Storage(e.to_string()))
    }
    fn delete(&self) -> Result<(), AppError> {
        match self.entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(AppError::Storage(e.to_string())),
        }
    }
}

#[cfg(test)]
pub struct MemoryStore(Mutex<Option<String>>);

#[cfg(test)]
impl MemoryStore {
    pub fn with(token: &str) -> Self {
        Self(Mutex::new(Some(token.to_string())))
    }
    pub fn empty() -> Self {
        Self(Mutex::new(None))
    }
}

#[cfg(test)]
impl TokenStore for MemoryStore {
    fn get(&self) -> Result<Option<String>, AppError> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn set(&self, token: &str) -> Result<(), AppError> {
        *self.0.lock().unwrap() = Some(token.to_string());
        Ok(())
    }
    fn delete(&self) -> Result<(), AppError> {
        *self.0.lock().unwrap() = None;
        Ok(())
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct TokenError {
    error: String,
}

pub struct Auth {
    cfg: OAuthConfig,
    store: Box<dyn TokenStore>,
    http: reqwest::Client,
    cached: Mutex<Option<(String, Instant)>>,
}

impl Auth {
    pub fn new(cfg: OAuthConfig, store: Box<dyn TokenStore>, http: reqwest::Client) -> Self {
        Self { cfg, store, http, cached: Mutex::new(None) }
    }

    pub fn is_logged_in(&self) -> bool {
        matches!(self.store.get(), Ok(Some(_)))
    }

    pub fn invalidate(&self) {
        *self.cached.lock().unwrap() = None;
    }

    pub fn logout(&self) -> Result<(), AppError> {
        self.invalidate();
        self.store.delete()
    }

    pub async fn access_token(&self) -> Result<String, AppError> {
        let cached = self.cached.lock().unwrap().clone();
        if let Some((token, expires_at)) = cached {
            if Instant::now() < expires_at {
                return Ok(token);
            }
        }
        let refresh = self.store.get()?.ok_or(AppError::NotLoggedIn)?;
        let params = [
            ("client_id", self.cfg.client_id.as_str()),
            ("client_secret", self.cfg.client_secret.as_str()),
            ("refresh_token", refresh.as_str()),
            ("grant_type", "refresh_token"),
        ];
        Ok(self.request_token(&params).await?.access_token)
    }

    pub async fn login<F>(&self, open_browser: F) -> Result<(), AppError>
    where
        F: FnOnce(&str) -> Result<(), AppError>,
    {
        if self.cfg.client_id.is_empty() {
            return Err(AppError::Config("GOOGLE_CLIENT_ID가 설정되지 않았습니다 (.env 확인)".into()));
        }
        let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| AppError::Login(e.to_string()))?;
        let port = listener.local_addr().map_err(|e| AppError::Login(e.to_string()))?.port();
        let redirect_uri = format!("http://127.0.0.1:{port}");
        let (verifier, challenge) = pkce_pair();
        let state = random_token(16);

        open_browser(&build_auth_url(&self.cfg, &redirect_uri, &challenge, &state))?;

        let code = tokio::time::timeout(LOGIN_TIMEOUT, wait_for_code(&listener, &state))
            .await
            .map_err(|_| AppError::Login("시간 초과 (5분)".into()))??;

        let params = [
            ("client_id", self.cfg.client_id.as_str()),
            ("client_secret", self.cfg.client_secret.as_str()),
            ("code", code.as_str()),
            ("code_verifier", verifier.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ];
        let token = self.request_token(&params).await?;
        if token.refresh_token.is_none() {
            return Err(AppError::Login("refresh token을 받지 못했습니다".into()));
        }
        Ok(())
    }

    async fn request_token(&self, params: &[(&str, &str)]) -> Result<TokenResponse, AppError> {
        let resp = self.http.post(&self.cfg.token_url).form(params).send().await?;
        let status = resp.status();
        if status.is_success() {
            let token: TokenResponse = resp.json().await?;
            if let Some(refresh) = &token.refresh_token {
                self.store.set(refresh)?;
            }
            let expires_at = Instant::now() + Duration::from_secs(token.expires_in.saturating_sub(60));
            *self.cached.lock().unwrap() = Some((token.access_token.clone(), expires_at));
            return Ok(token);
        }
        let body = resp.text().await.unwrap_or_default();
        let code = serde_json::from_str::<TokenError>(&body).map(|e| e.error).unwrap_or_default();
        if code == "invalid_grant" {
            self.invalidate();
            self.store.delete()?;
            return Err(AppError::AuthExpired);
        }
        Err(AppError::Api { status: status.as_u16(), message: body })
    }
}

fn random_token(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

pub(crate) fn challenge_for(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

pub(crate) fn pkce_pair() -> (String, String) {
    let verifier = random_token(32);
    let challenge = challenge_for(&verifier);
    (verifier, challenge)
}

pub(crate) fn build_auth_url(cfg: &OAuthConfig, redirect_uri: &str, challenge: &str, state: &str) -> String {
    url::Url::parse_with_params(
        &cfg.auth_url,
        &[
            ("client_id", cfg.client_id.as_str()),
            ("redirect_uri", redirect_uri),
            ("response_type", "code"),
            ("scope", SCOPES),
            ("code_challenge", challenge),
            ("code_challenge_method", "S256"),
            ("state", state),
            ("access_type", "offline"),
            ("prompt", "consent"),
        ],
    )
    .expect("auth_url is a valid URL")
    .to_string()
}

#[derive(Debug)]
pub(crate) enum CallbackResult {
    Ignore,
    Done(Result<String, AppError>),
}

pub(crate) fn parse_callback(request_line: &str, expected_state: &str) -> CallbackResult {
    let mut parts = request_line.split_whitespace();
    if parts.next() != Some("GET") {
        return CallbackResult::Ignore;
    }
    let target = parts.next().unwrap_or("");
    let Ok(url) = url::Url::parse(&format!("http://127.0.0.1{target}")) else {
        return CallbackResult::Ignore;
    };
    if url.path() != "/" {
        return CallbackResult::Ignore;
    }
    let q: HashMap<String, String> = url.query_pairs().into_owned().collect();
    if let Some(err) = q.get("error") {
        return CallbackResult::Done(Err(AppError::Login(err.clone())));
    }
    if q.get("state").map(String::as_str) != Some(expected_state) {
        return CallbackResult::Done(Err(AppError::Login("state 불일치".into())));
    }
    match q.get("code") {
        Some(code) => CallbackResult::Done(Ok(code.clone())),
        None => CallbackResult::Ignore,
    }
}

async fn wait_for_code(listener: &TcpListener, state: &str) -> Result<String, AppError> {
    loop {
        let (mut sock, _) = listener.accept().await.map_err(|e| AppError::Login(e.to_string()))?;
        let mut buf = vec![0u8; 4096];
        let n = sock.read(&mut buf).await.unwrap_or(0);
        let request = String::from_utf8_lossy(&buf[..n]).to_string();
        let line = request.lines().next().unwrap_or("");
        match parse_callback(line, state) {
            CallbackResult::Ignore => {
                let _ = sock.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
            }
            CallbackResult::Done(result) => {
                let msg = if result.is_ok() {
                    "로그인 완료. 이 창을 닫아도 됩니다."
                } else {
                    "로그인 실패. 위젯에서 다시 시도해 주세요."
                };
                let body = format!("<!doctype html><meta charset=\"utf-8\"><title>gcal-widget</title><p style=\"font:16px sans-serif\">{msg}</p>");
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = sock.write_all(resp.as_bytes()).await;
                return result;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn cfg_for(uri: &str) -> OAuthConfig {
        OAuthConfig {
            client_id: "cid".into(),
            client_secret: "csecret".into(),
            auth_url: format!("{uri}/auth"),
            token_url: format!("{uri}/token"),
        }
    }

    fn query(url: &str) -> HashMap<String, String> {
        url::Url::parse(url).unwrap().query_pairs().into_owned().collect()
    }

    #[test]
    fn pkce_challenge_matches_rfc7636_example() {
        assert_eq!(
            challenge_for("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn pkce_pair_is_consistent() {
        let (verifier, challenge) = pkce_pair();
        assert!(verifier.len() >= 43);
        assert_eq!(challenge_for(&verifier), challenge);
    }

    #[test]
    fn auth_url_has_required_params() {
        let url = build_auth_url(&cfg_for("https://x"), "http://127.0.0.1:5000", "chal", "st");
        let q = query(&url);
        assert_eq!(q["client_id"], "cid");
        assert_eq!(q["redirect_uri"], "http://127.0.0.1:5000");
        assert_eq!(q["response_type"], "code");
        assert_eq!(q["scope"], SCOPES);
        assert_eq!(q["code_challenge"], "chal");
        assert_eq!(q["code_challenge_method"], "S256");
        assert_eq!(q["state"], "st");
        assert_eq!(q["access_type"], "offline");
        assert_eq!(q["prompt"], "consent");
    }

    #[test]
    fn callback_with_code_and_matching_state() {
        let r = parse_callback("GET /?state=abc&code=4%2F0Ab HTTP/1.1", "abc");
        assert!(matches!(r, CallbackResult::Done(Ok(ref c)) if c == "4/0Ab"), "{r:?}");
    }

    #[test]
    fn callback_with_wrong_state_fails() {
        let r = parse_callback("GET /?state=evil&code=x HTTP/1.1", "abc");
        assert!(matches!(r, CallbackResult::Done(Err(AppError::Login(_)))), "{r:?}");
    }

    #[test]
    fn callback_access_denied_fails() {
        let r = parse_callback("GET /?error=access_denied&state=abc HTTP/1.1", "abc");
        assert!(matches!(r, CallbackResult::Done(Err(AppError::Login(ref m))) if m == "access_denied"), "{r:?}");
    }

    #[test]
    fn favicon_request_is_ignored() {
        assert!(matches!(parse_callback("GET /favicon.ico HTTP/1.1", "abc"), CallbackResult::Ignore));
    }

    #[tokio::test]
    async fn access_token_is_cached_until_expiry() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains("refresh_token=r1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"access_token": "at1", "expires_in": 3600})))
            .expect(1)
            .mount(&server)
            .await;
        let auth = Auth::new(cfg_for(&server.uri()), Box::new(MemoryStore::with("r1")), reqwest::Client::new());
        assert_eq!(auth.access_token().await.unwrap(), "at1");
        assert_eq!(auth.access_token().await.unwrap(), "at1");
    }

    #[tokio::test]
    async fn invalidate_forces_refresh() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"access_token": "at", "expires_in": 3600})))
            .expect(2)
            .mount(&server)
            .await;
        let auth = Auth::new(cfg_for(&server.uri()), Box::new(MemoryStore::with("r1")), reqwest::Client::new());
        auth.access_token().await.unwrap();
        auth.invalidate();
        auth.access_token().await.unwrap();
    }

    #[tokio::test]
    async fn not_logged_in_without_refresh_token() {
        let auth = Auth::new(cfg_for("http://127.0.0.1:1"), Box::new(MemoryStore::empty()), reqwest::Client::new());
        assert!(!auth.is_logged_in());
        assert!(matches!(auth.access_token().await, Err(AppError::NotLoggedIn)));
    }

    #[tokio::test]
    async fn invalid_grant_clears_store_and_reports_expired() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({"error": "invalid_grant"})))
            .mount(&server)
            .await;
        let auth = Auth::new(cfg_for(&server.uri()), Box::new(MemoryStore::with("r1")), reqwest::Client::new());
        assert!(matches!(auth.access_token().await, Err(AppError::AuthExpired)));
        assert!(!auth.is_logged_in());
    }

    #[tokio::test]
    async fn login_exchanges_code_via_loopback_and_stores_refresh_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=authorization_code"))
            .and(body_string_contains("code=abc"))
            .and(body_string_contains("code_verifier="))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"access_token": "at", "expires_in": 3600, "refresh_token": "r2"}),
            ))
            .expect(1)
            .mount(&server)
            .await;
        let auth = Auth::new(cfg_for(&server.uri()), Box::new(MemoryStore::empty()), reqwest::Client::new());
        auth.login(|url| {
            let q = query(url);
            let target = format!("{}/?code=abc&state={}", q["redirect_uri"], q["state"]);
            tokio::spawn(async move {
                let _ = reqwest::get(target).await;
            });
            Ok(())
        })
        .await
        .unwrap();
        assert!(auth.is_logged_in());
        assert_eq!(auth.access_token().await.unwrap(), "at");
    }

    #[tokio::test]
    async fn login_without_client_id_is_config_error() {
        let mut cfg = cfg_for("http://127.0.0.1:1");
        cfg.client_id.clear();
        let auth = Auth::new(cfg, Box::new(MemoryStore::empty()), reqwest::Client::new());
        assert!(matches!(auth.login(|_| Ok(())).await, Err(AppError::Config(_))));
    }
}
