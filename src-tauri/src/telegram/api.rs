use crate::workspace::models::*;
use serde_json::{json, Value};
use std::future::Future;
use std::time::Duration;

/// The slice of the Telegram Bot API that Signal uses. The real implementation talks to
/// Telegram over HTTPS; tests substitute a recording fake.
pub trait Api: Send + Sync {
    fn updates(&self, offset: i64, timeout_s: u64) -> impl Future<Output = Result<Vec<Value>>> + Send;
    fn send(
        &self,
        chat: i64,
        text: &str,
        markup: Option<&Value>,
    ) -> impl Future<Output = Result<i64>> + Send;
    fn edit(
        &self,
        chat: i64,
        message_id: i64,
        text: &str,
        markup: Option<&Value>,
    ) -> impl Future<Output = Result<()>> + Send;
    fn answer(&self, callback_id: &str, text: &str) -> impl Future<Output = Result<()>> + Send;
    fn download(&self, file_id: &str, max_bytes: u64) -> impl Future<Output = Result<Vec<u8>>> + Send;
}

pub const API_BASE: &str = "https://api.telegram.org";

pub struct HttpApi {
    base: String,
    token: String,
    http: reqwest::Client,
}

fn network(_: reqwest::Error) -> AppError {
    // reqwest errors can embed the request URL, which contains the bot token.
    AppError::new("Network", "Could not reach Telegram.")
}

impl HttpApi {
    pub fn new(base: &str, token: &str) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| AppError::new("Network", "Could not start the Telegram connection."))?;
        Ok(Self {
            base: base.trim_end_matches('/').to_owned(),
            token: token.to_owned(),
            http,
        })
    }

    async fn call(&self, method: &str, body: Value, timeout_s: u64) -> Result<Value> {
        let response = self
            .http
            .post(format!("{}/bot{}/{}", self.base, self.token, method))
            .timeout(Duration::from_secs(timeout_s))
            .json(&body)
            .send()
            .await
            .map_err(network)?;
        let status = response.status().as_u16();
        let value: Value = response.json().await.unwrap_or(Value::Null);
        if value["ok"] == true {
            return Ok(value["result"].clone());
        }
        let description = value["description"].as_str().unwrap_or("Request failed.");
        Err(match status {
            401 => AppError::new("TelegramAuth", "Telegram rejected the bot token."),
            409 => AppError::new(
                "TelegramConflict",
                "Another Signal installation (or webhook) is already receiving this bot's messages.",
            ),
            429 => AppError::new("TelegramRate", "Telegram asked Signal to slow down."),
            _ => AppError::new("Telegram", format!("Telegram error {status}: {description}")),
        })
    }

    pub async fn me(&self) -> Result<Value> {
        self.call("getMe", json!({}), 20).await
    }

    /// Long polling cannot work while a webhook is registered.
    pub async fn clear_webhook(&self) -> Result<()> {
        self.call("deleteWebhook", json!({"drop_pending_updates": false}), 20)
            .await
            .map(|_| ())
    }
}

impl Api for HttpApi {
    async fn updates(&self, offset: i64, timeout_s: u64) -> Result<Vec<Value>> {
        let result = self
            .call(
                "getUpdates",
                json!({"offset":offset,"timeout":timeout_s,"allowed_updates":["message","callback_query"]}),
                timeout_s + 20,
            )
            .await?;
        Ok(result.as_array().cloned().unwrap_or_default())
    }

    async fn send(&self, chat: i64, text: &str, markup: Option<&Value>) -> Result<i64> {
        let mut body = json!({"chat_id":chat,"text":text,"disable_web_page_preview":true});
        if let Some(markup) = markup {
            body["reply_markup"] = markup.clone();
        }
        let sent = self.call("sendMessage", body, 30).await?;
        sent["message_id"]
            .as_i64()
            .ok_or_else(|| AppError::new("Telegram", "Telegram returned no message ID."))
    }

    async fn edit(&self, chat: i64, message_id: i64, text: &str, markup: Option<&Value>) -> Result<()> {
        let mut body = json!({"chat_id":chat,"message_id":message_id,"text":text,"disable_web_page_preview":true});
        body["reply_markup"] = markup.cloned().unwrap_or_else(|| json!({"inline_keyboard":[]}));
        match self.call("editMessageText", body, 30).await {
            Ok(_) => Ok(()),
            // Re-sending an identical card is not a failure.
            Err(e) if e.message.contains("message is not modified") => Ok(()),
            Err(e) => Err(e),
        }
    }

    async fn answer(&self, callback_id: &str, text: &str) -> Result<()> {
        self.call(
            "answerCallbackQuery",
            json!({"callback_query_id":callback_id,"text":text}),
            20,
        )
        .await
        .map(|_| ())
    }

    async fn download(&self, file_id: &str, max_bytes: u64) -> Result<Vec<u8>> {
        let file = self.call("getFile", json!({"file_id":file_id}), 30).await?;
        if file["file_size"].as_u64().is_some_and(|n| n > max_bytes) {
            return Err(AppError::validation("That recording is too large."));
        }
        let path = file["file_path"]
            .as_str()
            .ok_or_else(|| AppError::new("Telegram", "Telegram returned no file path."))?;
        let mut response = self
            .http
            .get(format!("{}/file/bot{}/{}", self.base, self.token, path))
            .timeout(Duration::from_secs(120))
            .send()
            .await
            .map_err(network)?;
        if !response.status().is_success() {
            return Err(AppError::new("Telegram", "Telegram could not provide the recording."));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(network)? {
            if bytes.len() as u64 + chunk.len() as u64 > max_bytes {
                return Err(AppError::validation("That recording is too large."));
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
}
