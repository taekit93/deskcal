pub mod calendar;
pub mod models;
pub mod tasks;

use std::sync::Arc;
use std::time::Duration;

use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;

use crate::auth::Auth;
use crate::error::AppError;

const MAX_RETRIES: u32 = 3;

pub struct Google {
    http: reqwest::Client,
    auth: Arc<Auth>,
    calendar_base: String,
    tasks_base: String,
    backoff: Duration,
}

impl Google {
    pub fn new(http: reqwest::Client, auth: Arc<Auth>) -> Self {
        Self {
            http,
            auth,
            calendar_base: "https://www.googleapis.com/calendar/v3".into(),
            tasks_base: "https://tasks.googleapis.com/tasks/v1".into(),
            backoff: Duration::from_millis(500),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_base(http: reqwest::Client, auth: Arc<Auth>, base: &str) -> Self {
        Self {
            http,
            auth,
            calendar_base: format!("{base}/calendar/v3"),
            tasks_base: format!("{base}/tasks/v1"),
            backoff: Duration::from_millis(1),
        }
    }

    pub(crate) async fn send_json<T: DeserializeOwned>(
        &self,
        method: Method,
        url: &str,
        query: &[(&str, String)],
        body: Option<&serde_json::Value>,
    ) -> Result<T, AppError> {
        let mut auth_retried = false;
        let mut attempt = 0u32;
        loop {
            let token = self.auth.access_token().await?;
            let mut req = self.http.request(method.clone(), url).bearer_auth(&token).query(query);
            if let Some(b) = body {
                req = req.json(b);
            }
            let resp = req.send().await?;
            let status = resp.status();
            if status.is_success() {
                return Ok(resp.json::<T>().await?);
            }
            if status == StatusCode::UNAUTHORIZED && !auth_retried {
                auth_retried = true;
                self.auth.invalidate();
                continue;
            }
            if (status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()) && attempt < MAX_RETRIES {
                tokio::time::sleep(self.backoff * 2u32.pow(attempt)).await;
                attempt += 1;
                continue;
            }
            let message = resp.text().await.unwrap_or_default();
            return Err(AppError::Api { status: status.as_u16(), message });
        }
    }
}

pub(crate) fn enc(segment: &str) -> String {
    urlencoding::encode(segment).into_owned()
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::Arc;

    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::Google;
    use crate::auth::{Auth, MemoryStore, OAuthConfig};

    pub async fn mount_token(server: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"access_token": "at", "expires_in": 3600})))
            .mount(server)
            .await;
    }

    /// 토큰 mock은 붙이지 않는다. 호출하는 테스트가 mount_token 또는 직접 mock을 붙인다.
    pub async fn setup() -> (MockServer, Google) {
        let server = MockServer::start().await;
        let cfg = OAuthConfig {
            client_id: "cid".into(),
            client_secret: "cs".into(),
            auth_url: format!("{}/auth", server.uri()),
            token_url: format!("{}/token", server.uri()),
        };
        let auth = Arc::new(Auth::new(cfg, Box::new(MemoryStore::with("r1")), reqwest::Client::new()));
        let google = Google::with_base(reqwest::Client::new(), auth, &server.uri());
        (server, google)
    }
}
