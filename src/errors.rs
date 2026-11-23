use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    Cancelled,
    RateLimited,
    Authentication,
    Forbidden,
    NotFound,
    Server,
    Dns,
    Tls,
    Timeout,
    Network,
    PartialBody,
    TooLarge,
    EmptyBody,
    UnsupportedFormat,
    CorruptImage,
    InvalidMetadata,
    InvalidPath,
    Ownership,
    DiskFull,
    Permission,
    Conflict,
    Cache,
    Configuration,
    Queue,
    Trash,
    Compatibility,
    Panic,
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Failure {
    pub code: Code,
    pub message: String,
    pub retry_after: Option<u64>,
    pub retryable: bool,
}
impl Failure {
    pub fn new(code: Code, message: impl Into<String>) -> Self {
        Self {
            code,
            message: redact(&message.into(), false),
            retry_after: None,
            retryable: matches!(
                code,
                Code::RateLimited
                    | Code::Server
                    | Code::Dns
                    | Code::Timeout
                    | Code::Network
                    | Code::PartialBody
            ),
        }
    }
    pub fn classify(message: &str) -> Self {
        let m = message.to_lowercase();
        let code = if m.contains("iptal") {
            Code::Cancelled
        } else if m.contains("429") {
            Code::RateLimited
        } else if m.contains("401") {
            Code::Authentication
        } else if m.contains("403") {
            Code::Forbidden
        } else if m.contains("404") {
            Code::NotFound
        } else if ["500", "502", "503", "504"].iter().any(|s| m.contains(s)) {
            Code::Server
        } else if m.contains("dns") {
            Code::Dns
        } else if m.contains("tls") || m.contains("certificate") {
            Code::Tls
        } else if m.contains("zaman aş") || m.contains("timed out") {
            Code::Timeout
        } else if m.contains("no space left") {
            Code::DiskFull
        } else if m.contains("permission denied") {
            Code::Permission
        } else if m.contains("boyut sınır") || m.contains("too large") {
            Code::TooLarge
        } else if m.contains("biçim") {
            Code::UnsupportedFormat
        } else if m.contains("decode") || m.contains("görsel") || m.contains("image") {
            Code::CorruptImage
        } else if m.contains("bağlantı") || m.contains("connection") {
            Code::Network
        } else {
            Code::Unknown
        };
        Self::new(code, message)
    }
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{:?}]", self.message, self.code)
    }
}
impl std::error::Error for Failure {}
pub fn redact(text: &str, paths: bool) -> String {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut hide_next = false;
        for part in line.split_whitespace() {
            if hide_next {
                out.push("[gizli]".to_string());
                hide_next = false;
                continue;
            }
            let lower = part.to_lowercase();
            if ["authorization:", "cookie:", "set-cookie:"]
                .iter()
                .any(|s| lower.starts_with(s))
            {
                out.push("[gizli-başlık]".into());
                break;
            }
            if matches!(lower.as_str(), "bearer" | "basic") {
                out.push(part.into());
                hide_next = true;
                continue;
            }
            if let Ok(mut url) = url::Url::parse(part)
                && matches!(url.scheme(), "http" | "https")
            {
                let q = url
                    .query_pairs()
                    .filter(|(k, _)| !secret_key(k))
                    .map(|(k, v)| (k.into_owned(), v.into_owned()))
                    .collect::<Vec<_>>();
                url.set_query(None);
                let _ = url.set_username("");
                let _ = url.set_password(None);
                url.set_fragment(None);
                if paths {
                    url.set_path("/[anonim]");
                } else if !q.is_empty() {
                    url.query_pairs_mut().extend_pairs(q);
                }
                out.push(url.to_string());
                continue;
            }
            if paths && (part.starts_with('/') || lower.starts_with("file://")) {
                out.push("[yerel-yol]".into());
                break;
            }
            if part.split_once('=').is_some_and(|(k, _)| secret_key(k)) {
                out.push("[gizli]".into());
            } else {
                out.push(part.to_string());
            }
        }
    }
    out.join(" ").chars().take(2048).collect()
}

fn secret_key(key: &str) -> bool {
    let key = key.to_lowercase();
    [
        "key",
        "apikey",
        "api_key",
        "token",
        "access_token",
        "password",
        "secret",
        "signature",
        "authorization",
        "cookie",
        "login",
        "user_id",
    ]
    .iter()
    .any(|v| key == *v || key.ends_with(&format!("_{v}")))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secrets_are_not_exported() {
        let s = redact(
            "https://example.org/a?q=mountain&apikey=TOPSECRET&token=MORE",
            false,
        );
        assert!(!s.contains("TOPSECRET") && !s.contains("MORE"));
        assert!(s.contains("mountain"));
        assert!(!redact("/home/example/private.jpg", true).contains("private.jpg"));
    }
    #[test]
    fn errors_have_machine_codes() {
        assert_eq!(Failure::classify("HTTP 429").code, Code::RateLimited);
        assert_eq!(Failure::classify("TLS bağlantısı").code, Code::Tls);
    }
}
#[cfg(test)]
mod header_tests {
    #[test]
    fn headers_and_paths_do_not_escape() {
        for secret in [
            "Authorization: Bearer TOKEN123",
            "Cookie: session=COOKIE123",
            "Bearer BEARER123",
            "api_key=KEY123",
            "https://u:PASSWORD@example.org/private-title?token=TOKEN123",
            "/home/a/secret folder/name.jpg",
        ] {
            let r = super::redact(secret, true);
            for text in [
                "TOKEN123",
                "COOKIE123",
                "BEARER123",
                "KEY123",
                "PASSWORD",
                "private-title",
                "folder/name.jpg",
            ] {
                assert!(!r.contains(text), "{r}");
            }
        }
    }
}
