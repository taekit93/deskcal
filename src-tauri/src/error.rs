use serde::Serialize;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    #[error("로그인이 필요합니다")]
    NotLoggedIn,
    #[error("로그인이 만료되었습니다")]
    AuthExpired,
    #[error("네트워크 오류: {0}")]
    Network(String),
    #[error("Google API 오류 ({status}): {message}")]
    Api { status: u16, message: String },
    #[error("설정 오류: {0}")]
    Config(String),
    #[error("로그인 실패: {0}")]
    Login(String),
    #[error("저장소 오류: {0}")]
    Storage(String),
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_connect() || e.is_timeout() || e.is_request() {
            AppError::Network(e.to_string())
        } else {
            AppError::Api {
                status: e.status().map(|s| s.as_u16()).unwrap_or(0),
                message: e.to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_variant_serializes_with_kind_only() {
        let v = serde_json::to_value(AppError::NotLoggedIn).unwrap();
        assert_eq!(v, serde_json::json!({ "kind": "NotLoggedIn" }));
    }

    #[test]
    fn api_variant_serializes_status_and_message() {
        let v = serde_json::to_value(AppError::Api { status: 404, message: "nf".into() }).unwrap();
        assert_eq!(v, serde_json::json!({ "kind": "Api", "message": { "status": 404, "message": "nf" } }));
    }
}
