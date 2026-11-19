//! Google sign-in (installed-app loopback flow with PKCE) and the read-only
//! Calendar API calls Signal needs. Nothing here writes to Google.
use super::sync::Event;
use crate::workspace::models::*;
use base64::Engine;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const API: &str = "https://www.googleapis.com/calendar/v3";
pub const CALENDAR_SCOPE: &str = "https://www.googleapis.com/auth/calendar.readonly";
pub const GMAIL_SCOPE: &str = "https://www.googleapis.com/auth/gmail.readonly";
pub const SCOPES: &str = "openid email https://www.googleapis.com/auth/calendar.readonly https://www.googleapis.com/auth/gmail.readonly";

#[derive(Clone, Debug, PartialEq)]
pub struct Client {
    pub id: String,
    pub secret: String,
}

/// Reads the "Desktop app" client file downloaded from Google Cloud Console.
pub fn parse_client(json: &str) -> Result<Client> {
    let v: Value = serde_json::from_str(json)
        .map_err(|_| AppError::validation("This is not a Google client file."))?;
    let c = &v["installed"];
    if c.is_null() {
        return Err(AppError::validation(
            "Create the Google client as type \"Desktop app\" and download its file again.",
        ));
    }
    let id = c["client_id"].as_str().unwrap_or_default().trim();
    let secret = c["client_secret"].as_str().unwrap_or_default().trim();
    if !id.ends_with(".apps.googleusercontent.com")
        || id.len() > 200
        || secret.is_empty()
        || secret.len() > 200
    {
        return Err(AppError::validation(
            "This Google client file is incomplete.",
        ));
    }
    Ok(Client {
        id: id.into(),
        secret: secret.into(),
    })
}

pub fn verifier() -> String {
    // 64 characters from the unreserved set, 244 bits of randomness.
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}
pub fn challenge(verifier: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

pub fn auth_url(
    client_id: &str,
    redirect: &str,
    state: &str,
    challenge: &str,
    hint: Option<&str>,
) -> String {
    let mut u = url::Url::parse(AUTH_URL).expect("URL");
    {
        let mut q = u.query_pairs_mut();
        q.append_pair("client_id", client_id)
            .append_pair("redirect_uri", redirect)
            .append_pair("response_type", "code")
            .append_pair("scope", SCOPES)
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent select_account")
            .append_pair("include_granted_scopes", "true")
            .append_pair("state", state)
            .append_pair("code_challenge", challenge)
            .append_pair("code_challenge_method", "S256");
        if let Some(h) = hint {
            q.append_pair("login_hint", h);
        }
    }
    u.into()
}

const PAGE: &str = "<!doctype html><meta charset=utf-8><title>Signal</title><body style=\"font-family:system-ui;background:#11141b;color:#e8ecf4;display:grid;place-items:center;height:90vh\"><div><h1>{H}</h1><p>{P}</p></div>";

/// What the browser asked the loopback address for, if it was the sign-in redirect.
pub fn redirect_result(request_line: &str, state: &str) -> Option<Result<String>> {
    let path = request_line.split_whitespace().nth(1)?;
    let u = url::Url::parse(&format!("http://127.0.0.1{path}")).ok()?;
    if u.path() != "/" {
        return None;
    }
    let get = |k: &str| {
        u.query_pairs()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.into_owned())
    };
    if get("state").as_deref() != Some(state) {
        return Some(Err(AppError::validation(
            "Google's answer did not match this sign-in. Try again.",
        )));
    }
    if let Some(e) = get("error") {
        return Some(Err(AppError::new(
            "CalendarAuth",
            if e == "access_denied" {
                "Google sign-in was cancelled.".to_string()
            } else {
                format!("Google sign-in failed ({e}).")
            },
        )));
    }
    Some(
        get("code")
            .filter(|c| !c.is_empty())
            .ok_or_else(|| AppError::validation("Google did not return a sign-in code.")),
    )
}

/// Waits on the loopback listener for Google's redirect and answers the browser.
pub async fn wait_for_code(listener: TcpListener, state: &str) -> Result<String> {
    loop {
        let (mut stream, _) = listener.accept().await?;
        let mut buf = Vec::with_capacity(2048);
        let mut chunk = [0u8; 2048];
        let read = tokio::time::timeout(Duration::from_secs(10), async {
            while !buf.windows(4).any(|w| w == b"\r\n\r\n") && buf.len() < 16 * 1024 {
                let n = stream.read(&mut chunk).await?;
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);
            }
            Ok::<_, std::io::Error>(())
        })
        .await;
        if !matches!(read, Ok(Ok(()))) {
            continue;
        }
        let head = String::from_utf8_lossy(&buf);
        let line = head.lines().next().unwrap_or_default();
        let Some(result) = redirect_result(line, state) else {
            let _ = stream
                .write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await;
            continue;
        };
        let (h, p) = match &result {
            Ok(_) => (
                "Signal is connected",
                "You can close this tab and go back to Signal.",
            ),
            Err(_) => ("Signal was not connected", "Go back to Signal for details."),
        };
        let body = PAGE.replace("{H}", h).replace("{P}", p);
        let reply = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(reply.as_bytes()).await;
        let _ = stream.shutdown().await;
        return result;
    }
}

pub fn http() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| AppError::new("Network", "Could not prepare a secure connection."))
}
pub(crate) fn offline(_: reqwest::Error) -> AppError {
    // Never include the error text: it can contain request URLs and tokens.
    AppError::new(
        "Network",
        "Could not reach Google. Check the internet connection and try again.",
    )
}
fn form(pairs: &[(&str, &str)]) -> String {
    let mut s = url::form_urlencoded::Serializer::new(String::new());
    for (k, v) in pairs {
        s.append_pair(k, v);
    }
    s.finish()
}
async fn token_call(http: &reqwest::Client, body: String) -> Result<Value> {
    let r = http
        .post(TOKEN_URL)
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(body)
        .send()
        .await
        .map_err(offline)?;
    let status = r.status();
    let v: Value = r.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        return Err(match v["error"].as_str() {
            Some("invalid_grant") => AppError::new(
                "CalendarAuth",
                "Google no longer accepts Signal's sign-in for this account. Connect the account again.",
            ),
            Some("invalid_client") | Some("unauthorized_client") => AppError::new(
                "CalendarAuth",
                "Google rejected Signal's client file. Load the client file again.",
            ),
            _ => AppError::new("Network", format!("Google sign-in failed (HTTP {}).", status.as_u16())),
        });
    }
    Ok(v)
}

pub struct Tokens {
    pub access: String,
    pub refresh: String,
    pub email: String,
    /// Read-only mail was granted. Google lets people untick it on the consent screen.
    pub mail: bool,
}
pub fn granted(scope: &str, wanted: &str) -> bool {
    scope.split(' ').any(|s| s == wanted)
}

pub async fn exchange(
    http: &reqwest::Client,
    c: &Client,
    code: &str,
    verifier: &str,
    redirect: &str,
) -> Result<Tokens> {
    let v = token_call(
        http,
        form(&[
            ("code", code),
            ("client_id", &c.id),
            ("client_secret", &c.secret),
            ("redirect_uri", redirect),
            ("grant_type", "authorization_code"),
            ("code_verifier", verifier),
        ]),
    )
    .await?;
    let scope = v["scope"].as_str().unwrap_or_default();
    if !granted(scope, CALENDAR_SCOPE) {
        return Err(AppError::new(
            "CalendarAuth",
            "Signal needs permission to see your calendars. Connect again and tick the calendar box on Google's screen.",
        ));
    }
    let access = v["access_token"].as_str().unwrap_or_default().to_string();
    let refresh = v["refresh_token"].as_str().unwrap_or_default().to_string();
    if access.is_empty() || refresh.is_empty() {
        return Err(AppError::new(
            "CalendarAuth",
            "Google did not grant offline access. Try connecting again.",
        ));
    }
    let email = email_from_id_token(v["id_token"].as_str().unwrap_or_default())?;
    Ok(Tokens {
        access,
        refresh,
        email,
        mail: granted(scope, GMAIL_SCOPE),
    })
}

pub async fn refresh(http: &reqwest::Client, c: &Client, refresh: &str) -> Result<String> {
    let v = token_call(
        http,
        form(&[
            ("client_id", &c.id),
            ("client_secret", &c.secret),
            ("refresh_token", refresh),
            ("grant_type", "refresh_token"),
        ]),
    )
    .await?;
    v["access_token"]
        .as_str()
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| AppError::new("CalendarAuth", "Google did not return an access token."))
}

/// The ID token comes straight from Google's token endpoint over TLS, so its
/// claims are read without verifying the signature (Google's documented guidance).
pub fn email_from_id_token(token: &str) -> Result<String> {
    let bad = || {
        AppError::new(
            "CalendarAuth",
            "Google did not say which account signed in.",
        )
    };
    let part = token.split('.').nth(1).ok_or_else(bad)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(part.trim_end_matches('='))
        .map_err(|_| bad())?;
    let v: Value = serde_json::from_slice(&bytes).map_err(|_| bad())?;
    v["email"]
        .as_str()
        .filter(|e| e.contains('@'))
        .map(str::to_lowercase)
        .ok_or_else(bad)
}

async fn get(http: &reqwest::Client, access: &str, url: url::Url) -> Result<Value> {
    let r = http
        .get(url)
        .bearer_auth(access)
        .send()
        .await
        .map_err(offline)?;
    let status = r.status();
    let v: Value = r.json().await.unwrap_or(Value::Null);
    match status.as_u16() {
        200 => Ok(v),
        401 => Err(AppError::new(
            "CalendarAuth",
            "Google signed Signal out. Connect the account again.",
        )),
        403 if v.to_string().contains("accessNotConfigured")
            || v.to_string().contains("SERVICE_DISABLED") =>
        {
            Err(AppError::new(
                "Calendar",
                "The Google Calendar API is not enabled in the Google Cloud project.",
            ))
        }
        404 => Err(AppError::new(
            "CalendarGone",
            "This calendar no longer exists or is no longer shared with you.",
        )),
        s => Err(AppError::new(
            "Network",
            format!("Google Calendar answered HTTP {s}. Signal will try again later."),
        )),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalendarInfo {
    pub id: String,
    pub name: String,
    pub color: String,
    pub primary: bool,
}

pub async fn calendars(http: &reqwest::Client, access: &str) -> Result<Vec<CalendarInfo>> {
    let mut out = Vec::new();
    let mut page: Option<String> = None;
    for _ in 0..10 {
        let mut u = url::Url::parse(&format!("{API}/users/me/calendarList")).unwrap();
        u.query_pairs_mut().append_pair("maxResults", "250");
        if let Some(p) = &page {
            u.query_pairs_mut().append_pair("pageToken", p);
        }
        let v = get(http, access, u).await?;
        out.extend(parse_calendars(&v));
        page = v["nextPageToken"].as_str().map(str::to_owned);
        if page.is_none() {
            break;
        }
    }
    Ok(out)
}
pub fn parse_calendars(v: &Value) -> Vec<CalendarInfo> {
    v["items"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|c| c["deleted"] != true)
                .filter_map(|c| {
                    Some(CalendarInfo {
                        id: c["id"].as_str()?.into(),
                        name: c["summaryOverride"]
                            .as_str()
                            .or(c["summary"].as_str())
                            .unwrap_or("Calendar")
                            .into(),
                        color: c["backgroundColor"].as_str().unwrap_or("#7aa2ff").into(),
                        primary: c["primary"] == true,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub async fn events(
    http: &reqwest::Client,
    access: &str,
    calendar: &str,
    from: &str,
    to: &str,
) -> Result<Vec<Event>> {
    let mut out = Vec::new();
    let mut page: Option<String> = None;
    for _ in 0..20 {
        let mut u = url::Url::parse(API).unwrap();
        u.path_segments_mut()
            .unwrap()
            .extend(["calendars", calendar, "events"]);
        {
            let mut q = u.query_pairs_mut();
            q.append_pair("timeMin", from)
                .append_pair("timeMax", to)
                .append_pair("singleEvents", "true")
                .append_pair("showDeleted", "true")
                .append_pair("maxResults", "250");
            if let Some(p) = &page {
                q.append_pair("pageToken", p);
            }
        }
        let v = get(http, access, u).await?;
        out.extend(
            v["items"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(parse_event),
        );
        page = v["nextPageToken"].as_str().map(str::to_owned);
        if page.is_none() {
            return Ok(out);
        }
    }
    Err(AppError::validation(
        "This calendar has too many events to read. Signal reads up to 5,000 at a time.",
    ))
}

const SKIPPED_TYPES: [&str; 4] = ["workingLocation", "outOfOffice", "focusTime", "birthday"];

pub fn parse_event(v: &Value) -> Option<Event> {
    let id = v["id"].as_str()?.to_string();
    let mut e = Event {
        id,
        ..Event::default()
    };
    if v["status"] == "cancelled" {
        e.cancelled = true;
        return Some(e);
    }
    let start = v["start"]["dateTime"]
        .as_str()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
    let end = v["end"]["dateTime"]
        .as_str()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
    let declined = v["attendees"].as_array().is_some_and(|a| {
        a.iter()
            .any(|p| p["self"] == true && p["responseStatus"] == "declined")
    });
    let kind = v["eventType"].as_str().unwrap_or("default");
    let (Some(start), Some(end)) = (start, end) else {
        // All-day events are days, not meetings.
        e.skip = true;
        return Some(e);
    };
    let minutes = (end - start).num_minutes();
    e.skip = declined || SKIPPED_TYPES.contains(&kind) || minutes > 1440;
    e.starts_at = crate::agenda::models::utc(start.with_timezone(&chrono::Utc));
    e.duration_min = minutes.clamp(1, 1440);
    let title: String = v["summary"]
        .as_str()
        .unwrap_or_default()
        .trim()
        .chars()
        .take(300)
        .collect();
    e.title = if title.is_empty() {
        "(No title)".into()
    } else {
        title
    };
    let location = v["location"].as_str().unwrap_or_default().trim();
    let video = v["hangoutLink"].as_str().map(str::to_owned).or_else(|| {
        v["conferenceData"]["entryPoints"]
            .as_array()?
            .iter()
            .find(|p| p["entryPointType"] == "video")?["uri"]
            .as_str()
            .map(str::to_owned)
    });
    let location_link =
        (location.starts_with("https://") || location.starts_with("http://")).then(|| {
            location
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_string()
        });
    e.link_url = video
        .or(location_link)
        .filter(|u| crate::links::validate(u).is_ok());
    let mut agenda = String::new();
    if !location.is_empty() && e.link_url.as_deref() != Some(location) {
        agenda.push_str(&format!("Location: {location}\n\n"));
    }
    agenda.push_str(&plain(v["description"].as_str().unwrap_or_default()));
    e.agenda_md = agenda.trim().chars().take(5000).collect();
    Some(e)
}

/// Google event descriptions are often HTML; Signal stores plain text.
pub fn plain(html: &str) -> String {
    let mut s = html.to_string();
    for (from, to) in [
        ("<br>", "\n"),
        ("<br/>", "\n"),
        ("<br />", "\n"),
        ("</p>", "\n\n"),
        ("</div>", "\n"),
        ("</li>", "\n"),
        ("<li>", "- "),
    ] {
        s = s.replace(from, to).replace(&from.to_uppercase(), to);
    }
    let mut out = String::with_capacity(s.len());
    let mut tag = false;
    for ch in s.chars() {
        match ch {
            '<' => tag = true,
            '>' if tag => tag = false,
            _ if !tag => out.push(ch),
            _ => {}
        }
    }
    let out = out
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    let mut text = String::new();
    let mut blank = 0;
    for line in out.lines().map(str::trim_end) {
        if line.trim().is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        text.push_str(line);
        text.push('\n');
    }
    text.trim().to_string()
}
