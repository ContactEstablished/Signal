use crate::workspace::models::*;
use serde_json::{json, Value};
use std::future::Future;
use std::time::Duration;

/// The slice of the Telegram Bot API that Signal uses. The real implementation talks to
/// Telegram over HTTPS; tests substitute a recording fake.
pub trait Api: Send + Sync {
    fn updates(
        &self,
        offset: i64,
        timeout_s: u64,
    ) -> impl Future<Output = Result<Vec<Value>>> + Send;
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
    fn download(
        &self,
        file_id: &str,
        max_bytes: u64,
    ) -> impl Future<Output = Result<Vec<u8>>> + Send;
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

    async fn edit(
        &self,
        chat: i64,
        message_id: i64,
        text: &str,
        markup: Option<&Value>,
    ) -> Result<()> {
        let mut body = json!({"chat_id":chat,"message_id":message_id,"text":text,"disable_web_page_preview":true});
        body["reply_markup"] = markup
            .cloned()
            .unwrap_or_else(|| json!({"inline_keyboard":[]}));
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
            return Err(AppError::new(
                "Telegram",
                "Telegram could not provide the recording.",
            ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    const TOKEN: &str = "123456789:AAHdqTcvCH1vGWJxfSeofSAs0K5PALDsaw";

    /// A scripted Telegram: answers each connection in order and records the requests.
    fn telegram(responses: Vec<(u16, Vec<u8>)>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let mut seen = Vec::new();
            for (status, body) in responses {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                let mut buf = [0u8; 4096];
                loop {
                    let n = socket.read(&mut buf).unwrap();
                    request.extend_from_slice(&buf[..n]);
                    if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&request[..end]).to_lowercase();
                        let len = head
                            .lines()
                            .find_map(|l| {
                                l.strip_prefix("content-length: ")
                                    .and_then(|v| v.parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                        if request.len() >= end + 4 + len {
                            break;
                        }
                    }
                }
                let head = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                socket.write_all(head.as_bytes()).unwrap();
                socket.write_all(&body).unwrap();
                seen.push(String::from_utf8(request).unwrap());
            }
            seen
        });
        (base, handle)
    }
    fn ok(result: Value) -> (u16, Vec<u8>) {
        (
            200,
            json!({"ok":true,"result":result}).to_string().into_bytes(),
        )
    }
    fn fail(status: u16, description: &str) -> (u16, Vec<u8>) {
        (
            status,
            json!({"ok":false,"error_code":status,"description":description})
                .to_string()
                .into_bytes(),
        )
    }

    #[tokio::test]
    async fn requests_use_the_bot_api_shapes_telegram_expects() {
        let (base, server) = telegram(vec![
            ok(json!([{"update_id":9,"message":{"text":"hi"}}])),
            ok(json!({"message_id":55})),
            ok(json!(true)),
            fail(400, "Bad Request: message is not modified"),
            ok(json!(true)),
        ]);
        let api = HttpApi::new(&base, TOKEN).unwrap();
        let updates = api.updates(9, 25).await.unwrap();
        assert_eq!(updates[0]["update_id"], 9);
        let markup = json!({"inline_keyboard":[[{"text":"Yes","callback_data":"k|a|1|y"}]]});
        assert_eq!(api.send(7, "hello", Some(&markup)).await.unwrap(), 55);
        api.edit(7, 55, "changed", None).await.unwrap();
        // Telegram refuses an identical edit; that is not a failure.
        api.edit(7, 55, "changed", None).await.unwrap();
        api.answer("cb1", "Done").await.unwrap();

        let seen = server.join().unwrap();
        let body = |i: usize| -> Value {
            serde_json::from_str(seen[i].split("\r\n\r\n").nth(1).unwrap()).unwrap()
        };
        assert!(seen[0].starts_with(&format!("POST /bot{TOKEN}/getUpdates ")));
        assert_eq!(
            body(0),
            json!({"offset":9,"timeout":25,"allowed_updates":["message","callback_query"]})
        );
        assert!(seen[1].starts_with(&format!("POST /bot{TOKEN}/sendMessage ")));
        assert_eq!(body(1)["chat_id"], 7);
        assert_eq!(body(1)["reply_markup"], markup);
        assert!(seen[2].contains("/editMessageText "));
        assert_eq!(
            body(2)["reply_markup"],
            json!({"inline_keyboard":[]}),
            "an edit without buttons clears them"
        );
        assert!(seen[4].contains("/answerCallbackQuery "));
        assert_eq!(body(4)["callback_query_id"], "cb1");
    }

    #[tokio::test]
    async fn failures_are_classified_and_never_echo_the_token() {
        let (base, server) = telegram(vec![
            fail(401, "Unauthorized"),
            fail(409, "Conflict: terminated by other getUpdates request"),
            fail(429, "Too Many Requests: retry after 5"),
            fail(500, "boom"),
        ]);
        let api = HttpApi::new(&base, TOKEN).unwrap();
        let codes: Vec<_> = [
            api.updates(0, 1).await.unwrap_err(),
            api.updates(0, 1).await.unwrap_err(),
            api.updates(0, 1).await.unwrap_err(),
            api.send(1, "x", None).await.unwrap_err(),
        ]
        .into_iter()
        .map(|e| {
            assert!(!e.message.contains(TOKEN), "{}", e.message);
            e.code
        })
        .collect();
        assert_eq!(
            codes,
            [
                "TelegramAuth",
                "TelegramConflict",
                "TelegramRate",
                "Telegram"
            ]
        );
        server.join().unwrap();

        // An unreachable server reports a plain network failure with no URL or token in it.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let dead = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);
        let error = HttpApi::new(&dead, TOKEN)
            .unwrap()
            .send(1, "x", None)
            .await
            .unwrap_err();
        assert_eq!(error.code, "Network");
        assert!(
            !error.message.contains(TOKEN) && !error.message.contains("http"),
            "{}",
            error.message
        );
    }

    #[tokio::test]
    async fn downloads_are_size_limited_before_and_while_streaming() {
        let (base, server) = telegram(vec![
            ok(json!({"file_path":"voice/file_1.oga","file_size":4})),
            (200, vec![1, 2, 3, 4]),
            ok(json!({"file_path":"voice/big.oga","file_size":5_000_000})),
            ok(json!({"file_path":"voice/liar.oga","file_size":3})),
            (200, vec![0; 64]),
        ]);
        let api = HttpApi::new(&base, TOKEN).unwrap();
        assert_eq!(api.download("a", 1_000).await.unwrap(), vec![1, 2, 3, 4]);
        assert!(
            api.download("b", 1_000_000).await.is_err(),
            "declared size is refused without downloading"
        );
        assert!(
            api.download("c", 10).await.is_err(),
            "a file larger than it claimed is cut off"
        );
        let seen = server.join().unwrap();
        assert!(seen[1].starts_with(&format!("GET /file/bot{TOKEN}/voice/file_1.oga ")));
        assert_eq!(seen.len(), 5);
    }

    #[tokio::test]
    async fn the_bot_is_identified_and_a_webhook_is_cleared_before_polling() {
        let (base, server) = telegram(vec![
            ok(json!({"id":1,"username":"signal_dev_bot"})),
            ok(json!(true)),
        ]);
        let api = HttpApi::new(&base, TOKEN).unwrap();
        assert_eq!(api.me().await.unwrap()["username"], "signal_dev_bot");
        api.clear_webhook().await.unwrap();
        let seen = server.join().unwrap();
        assert!(seen[0].contains("/getMe ") && seen[1].contains("/deleteWebhook "));
    }
}
