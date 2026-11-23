use super::*;
use crate::errors::{Code, Failure};
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Retry {
    After(u64),
    Stop,
}
pub fn retry_after(value: &str) -> Option<u64> {
    let value = value.trim();
    if let Ok(n) = value.parse::<u64>() {
        return Some(n);
    }
    let p = value.split_whitespace().collect::<Vec<_>>();
    if p.len() != 6 || p[5] != "GMT" {
        return None;
    }
    let month = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]
    .iter()
    .position(|m| *m == p[2])?
        + 1;
    let day = p[1].parse::<u8>().ok()?;
    let year = p[3].parse::<u32>().ok()?;
    let iso = format!("{year:04}-{month:02}-{day:02}T{}Z", p[4]);
    let stamp = crate::providers::parse_date(&iso);
    if stamp <= 0 {
        return None;
    }
    Some((stamp as u64).saturating_sub(crate::runtime::now()))
}
pub fn policy(status: u16, attempt: u8, after: Option<u64>) -> Retry {
    if attempt >= 2 {
        return Retry::Stop;
    }
    if status == 429 {
        if let Some(delay) = after
            && delay <= 10
        {
            return Retry::After(delay.max(1));
        }
        return Retry::Stop;
    }
    if matches!(status, 502..=504) {
        if let Some(delay) = after {
            if delay > 10 {
                return Retry::Stop;
            }
            return Retry::After(delay.max(1));
        }
        return Retry::After(1u64 << attempt);
    }
    Retry::Stop
}
impl Network {
    pub(crate) fn request(
        &self,
        url: &str,
        seconds: u64,
        cancel: &AtomicBool,
    ) -> Result<ureq::http::Response<ureq::Body>> {
        self.request_inner(url, seconds, cancel, false)
    }
    pub(crate) fn request_range(
        &self,
        url: &str,
        seconds: u64,
        cancel: &AtomicBool,
    ) -> Result<ureq::http::Response<ureq::Body>> {
        self.request_inner(url, seconds, cancel, true)
    }
    fn request_inner(
        &self,
        url: &str,
        seconds: u64,
        cancel: &AtomicBool,
        range: bool,
    ) -> Result<ureq::http::Response<ureq::Body>> {
        valid_url(url)?;
        let host = url::Url::parse(url)?
            .host_str()
            .unwrap_or_default()
            .to_string();
        for attempt in 0..3u8 {
            check(cancel)?;
            if let Some(until) = self
                .cooldowns
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&host)
                .copied()
                && until > crate::runtime::now()
            {
                let mut error = Failure::new(
                    Code::RateLimited,
                    "Kaynak bekleme süresi istedi; süre dolmadan tekrar istek gönderilmiyor",
                );
                error.retry_after = Some(until);
                return Err(error.into());
            }
            let _span = crate::diagnostics::Span::new(None, "http");
            let mut request = self.agent.get(url).header(
                "User-Agent",
                concat!(
                    "Duvar/",
                    env!("CARGO_PKG_VERSION"),
                    " (+https://github.com/mo4tal/duvar)"
                ),
            );
            if range {
                request = request.header("Range", "bytes=0-131071");
            }
            let result = request
                .config()
                .http_status_as_error(false)
                .timeout_global(Some(Duration::from_secs(seconds)))
                .build()
                .call();
            let response = match result {
                Ok(r) => r,
                Err(e) => {
                    return Err(match e {
                        ureq::Error::HostNotFound => {
                            Failure::new(Code::Dns, "Kaynağın DNS adresi çözülemedi")
                        }
                        ureq::Error::Timeout(_) => {
                            Failure::new(Code::Timeout, "Bağlantı/yanıt zaman aşımına uğradı")
                        }
                        ureq::Error::Rustls(_) | ureq::Error::Tls(_) => Failure::new(
                            Code::Tls,
                            "TLS sertifikası veya güvenli bağlantı doğrulanamadı",
                        ),
                        _ => Failure::new(Code::Network, "Kaynakla bağlantı kurulamadı"),
                    }
                    .into());
                }
            };
            let status = response.status().as_u16();
            if (200..300).contains(&status) {
                return Ok(response);
            }
            let after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(retry_after);
            if let Retry::After(delay) = policy(status, attempt, after) {
                let delay = if host == "wallhaven.cc" {
                    delay.max(2)
                } else {
                    delay
                };
                crate::diagnostics::log(
                    None,
                    "http",
                    "retry",
                    &format!("HTTP {status}, {} sn", delay),
                    None,
                );
                drop(response);
                for _ in 0..delay * 20 {
                    check(cancel)?;
                    std::thread::sleep(Duration::from_millis(50));
                }
                continue;
            }
            let code = match status {
                401 => Code::Authentication,
                403 => Code::Forbidden,
                404 => Code::NotFound,
                429 => Code::RateLimited,
                500..=599 => Code::Server,
                _ => Code::Network,
            };
            let mut failure = Failure::new(code, format!("Kaynak HTTP {status} yanıtı verdi"));
            if status == 429 || after.is_some() {
                let seconds = after.unwrap_or(60);
                let until = crate::runtime::now().saturating_add(seconds);
                self.cooldowns
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(host.clone(), until);
                failure.retry_after = Some(until);
            }
            return Err(failure.into());
        }
        unreachable!()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rate_limit_is_not_blindly_retried() {
        assert_eq!(policy(429, 0, Some(120)), Retry::Stop);
        assert_eq!(policy(429, 0, Some(2)), Retry::After(2));
        assert_eq!(policy(503, 2, None), Retry::Stop);
        assert_eq!(policy(401, 0, None), Retry::Stop);
        assert_eq!(retry_after("3"), Some(3));
        assert_eq!(retry_after("bad"), None);
    }
}
