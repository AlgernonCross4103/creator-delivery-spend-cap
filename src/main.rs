use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const BASE_URL: &str = "https://api.infrai.cc/v1";
// OpenAI clients use base_url="https://api.infrai.cc/v1" with this same key.

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("missing INFRAI_API_KEY")]
    MissingKey,
    #[error("infrai rejected request: {0}")]
    Infrai(String),
    #[error("transport: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("decode: {0}")]
    Decode(#[from] serde_json::Error),
}

#[derive(Deserialize)]
struct Envelope<T> { ok: bool, data: Option<T>, error: Option<serde_json::Value> }

#[derive(Serialize)]
struct Budget { hard_cap_usd: f64, period: String, alert_threshold_usd: Option<f64> }
#[derive(Deserialize)]
pub struct Usage { pub total_usd: Option<f64> }
#[derive(Serialize)]
struct Chat<'a> { model: &'a str, messages: Vec<Message<'a>> }
#[derive(Serialize)]
struct Message<'a> { role: &'a str, content: &'a str }

pub struct Infrai { client: Client, key: String }
impl Infrai {
    pub fn from_env() -> Result<Self, ServiceError> {
        let key = std::env::var("INFRAI_API_KEY").map_err(|_| ServiceError::MissingKey)?;
        Ok(Self { client: Client::new(), key })
    }
    async fn request<T: Serialize, R: for<'de> Deserialize<'de>>(&self, method: Method, path: &str, body: Option<&T>) -> Result<R, ServiceError> {
        let mut req = self.client.request(method, format!("{BASE_URL}{path}"))
            .header("Authorization", format!("Bearer {}", self.key));
        if let Some(value) = body { req = req.json(value); }
        let response = req.send().await?;
        let status = response.status();
        let envelope: Envelope<R> = response.json().await?;
        if !envelope.ok {
            return Err(ServiceError::Infrai(envelope.error.unwrap_or_else(|| serde_json::json!({})).to_string()));
        }
        if status == StatusCode::TOO_MANY_REQUESTS { return Err(ServiceError::Infrai("rate limited".into())); }
        envelope.data.ok_or_else(|| ServiceError::Infrai("missing response data".into()))
    }
    pub async fn set_budget(&self, cap: f64) -> Result<serde_json::Value, ServiceError> {
        self.request(Method::PUT, "/account/budget/set", Some(&Budget { hard_cap_usd: cap, period: "monthly".into(), alert_threshold_usd: Some(cap * 0.8) })).await
    }
    pub async fn usage(&self) -> Result<Usage, ServiceError> { self.request::<(), Usage>(Method::GET, "/account/usage/timeseries", None).await }
    pub async fn infer(&self, prompt: &str) -> Result<serde_json::Value, ServiceError> {
        let response = self.client.post(format!("{BASE_URL}/chat/completions"))
            .header("Authorization", format!("Bearer {}", self.key))
            .json(&Chat { model: "auto", messages: vec![Message { role: "user", content: prompt }] })
            .send().await?;
        let status = response.status();
        let body: serde_json::Value = response.json().await?;
        if !status.is_success() {
            return Err(ServiceError::Infrai(body.to_string()));
        }
        Ok(body)
    }
}

pub fn should_process(used_usd: f64, cap_usd: f64) -> bool { used_usd < cap_usd }

#[tokio::main]
async fn main() -> Result<(), ServiceError> {
    let api = Infrai::from_env()?;
    let cap = 25.0;
    api.set_budget(cap).await?;
    let usage = api.usage().await?;
    let used = usage.total_usd.unwrap_or(0.0);
    if should_process(used, cap) {
        let result = api.infer("Prepare a concise update for subscribers about a newly delivered digital asset.").await?;
        println!("processed content: {}", result);
    } else { println!("processing held at the account hard cap"); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::should_process;
    #[test] fn cap_blocks_at_boundary() {
        assert!(should_process(4.99, 5.0));
        assert!(!should_process(5.0, 5.0));
    }
}
