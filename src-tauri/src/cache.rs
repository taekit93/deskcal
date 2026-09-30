use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use crate::error::AppError;

pub struct Cache {
    path: PathBuf,
    lock: Mutex<()>,
}

impl Cache {
    pub fn new(path: PathBuf) -> Self {
        Self { path, lock: Mutex::new(()) }
    }

    fn read_map(&self) -> HashMap<String, Value> {
        fs::read_to_string(&self.path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let _guard = self.lock.lock().unwrap();
        self.read_map().remove(key).and_then(|v| serde_json::from_value(v).ok())
    }

    /// 저장된 캐시를 모두 지운다 (로그아웃 시 이전 계정 데이터 제거).
    pub fn clear(&self) -> Result<(), AppError> {
        let _guard = self.lock.lock().unwrap();
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(AppError::Storage(e.to_string())),
        }
    }

    pub fn put<T: Serialize>(&self, key: &str, value: &T) -> Result<(), AppError> {
        let _guard = self.lock.lock().unwrap();
        let mut map = self.read_map();
        let value = serde_json::to_value(value).map_err(|e| AppError::Storage(e.to_string()))?;
        map.insert(key.to_string(), value);
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(|e| AppError::Storage(e.to_string()))?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let text = serde_json::to_string(&map).map_err(|e| AppError::Storage(e.to_string()))?;
        fs::write(&tmp, text).map_err(|e| AppError::Storage(e.to_string()))?;
        fs::rename(&tmp, &self.path).map_err(|e| AppError::Storage(e.to_string()))
    }
}

pub fn with_fallback<T: Serialize + DeserializeOwned>(
    cache: &Cache,
    key: &str,
    result: Result<T, AppError>,
    mark_stale: impl FnOnce(&mut T),
) -> Result<T, AppError> {
    match result {
        Ok(value) => {
            if let Err(e) = cache.put(key, &value) {
                log::warn!("cache write failed: {e}");
            }
            Ok(value)
        }
        Err(AppError::Network(msg)) => match cache.get::<T>(key) {
            Some(mut value) => {
                mark_stale(&mut value);
                Ok(value)
            }
            None => Err(AppError::Network(msg)),
        },
        // 세션이 만료·철회되면 다음 로그인이 다른 계정일 수 있으므로 이전 계정 캐시를 지운다.
        Err(AppError::AuthExpired) => {
            if let Err(e) = cache.clear() {
                log::warn!("cache clear failed: {e}");
            }
            Err(AppError::AuthExpired)
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Snap {
        items: Vec<String>,
        stale: bool,
    }

    fn snap(items: &[&str]) -> Snap {
        Snap { items: items.iter().map(|s| s.to_string()).collect(), stale: false }
    }

    fn cache() -> (tempfile::TempDir, Cache) {
        let dir = tempfile::tempdir().unwrap();
        let c = Cache::new(dir.path().join("sub").join("cache.json"));
        (dir, c)
    }

    #[test]
    fn missing_key_is_none() {
        let (_d, c) = cache();
        assert_eq!(c.get::<Snap>("month:2026-09"), None);
    }

    #[test]
    fn clear_removes_everything_and_is_idempotent() {
        let (_d, c) = cache();
        c.put("a", &snap(&["1"])).unwrap();
        c.clear().unwrap();
        assert_eq!(c.get::<Snap>("a"), None);
        c.clear().unwrap();
    }

    #[test]
    fn put_then_get_roundtrip_and_keys_are_independent() {
        let (_d, c) = cache();
        c.put("a", &snap(&["1"])).unwrap();
        c.put("b", &snap(&["2"])).unwrap();
        assert_eq!(c.get::<Snap>("a"), Some(snap(&["1"])));
        assert_eq!(c.get::<Snap>("b"), Some(snap(&["2"])));
    }

    #[test]
    fn corrupt_file_reads_none_and_put_recovers() {
        let (_d, c) = cache();
        std::fs::create_dir_all(c.path.parent().unwrap()).unwrap();
        std::fs::write(&c.path, "garbage").unwrap();
        assert_eq!(c.get::<Snap>("a"), None);
        c.put("a", &snap(&["x"])).unwrap();
        assert_eq!(c.get::<Snap>("a"), Some(snap(&["x"])));
    }

    #[test]
    fn fallback_success_is_cached() {
        let (_d, c) = cache();
        let out = with_fallback(&c, "k", Ok(snap(&["new"])), |s| s.stale = true).unwrap();
        assert_eq!(out, snap(&["new"]));
        assert_eq!(c.get::<Snap>("k"), Some(snap(&["new"])));
    }

    #[test]
    fn fallback_network_error_returns_stale_cache() {
        let (_d, c) = cache();
        c.put("k", &snap(&["old"])).unwrap();
        let out = with_fallback::<Snap>(&c, "k", Err(AppError::Network("down".into())), |s| s.stale = true).unwrap();
        assert_eq!(out.items, vec!["old".to_string()]);
        assert!(out.stale);
    }

    #[test]
    fn fallback_network_error_without_cache_is_error() {
        let (_d, c) = cache();
        let out = with_fallback::<Snap>(&c, "k", Err(AppError::Network("down".into())), |s| s.stale = true);
        assert!(matches!(out, Err(AppError::Network(_))));
    }

    #[test]
    fn fallback_auth_expired_clears_previous_account_cache() {
        let (_d, c) = cache();
        c.put("k", &snap(&["old"])).unwrap();
        c.put("other-month", &snap(&["old2"])).unwrap();
        let out = with_fallback::<Snap>(&c, "k", Err(AppError::AuthExpired), |s| s.stale = true);
        assert!(matches!(out, Err(AppError::AuthExpired)));
        assert_eq!(c.get::<Snap>("other-month"), None);
    }

    #[test]
    fn fallback_auth_error_is_not_masked_by_cache() {
        let (_d, c) = cache();
        c.put("k", &snap(&["old"])).unwrap();
        let out = with_fallback::<Snap>(&c, "k", Err(AppError::AuthExpired), |s| s.stale = true);
        assert!(matches!(out, Err(AppError::AuthExpired)));
    }
}
