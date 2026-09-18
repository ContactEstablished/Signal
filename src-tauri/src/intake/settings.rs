use crate::workspace::models::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub endpoint: String,
    pub model: String,
    pub whisper_model: String,
    pub microphone: String,
    pub consent: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            model: String::new(),
            whisper_model: "base.en".into(),
            microphone: String::new(),
            consent: false,
        }
    }
}
pub fn root(app: &AppHandle) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|_| AppError::new("File", "Application storage is unavailable."))?
        .join(if app.state::<crate::db::RuntimeState>().seeded {
            "voice-dev"
        } else {
            "voice"
        });
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
pub async fn load(app: &AppHandle) -> Result<Settings> {
    let pool = crate::workspace::pool(app).await?;
    let raw: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key='voice' ")
        .fetch_optional(&pool)
        .await?;
    raw.map(|v| {
        serde_json::from_str(&v).map_err(|_| AppError::validation("Voice settings are invalid."))
    })
    .transpose()
    .map(|v| v.unwrap_or_default())
}
pub fn endpoint(value: &str) -> Result<url::Url> {
    let u = url::Url::parse(value)
        .map_err(|_| AppError::validation("Enter the provider's HTTPS API base URL."))?;
    if u.scheme() != "https"
        || u.host_str().is_none()
        || !u.username().is_empty()
        || u.password().is_some()
        || u.query().is_some()
        || u.fragment().is_some()
    {
        return Err(AppError::validation(
            "Use an HTTPS API base URL without credentials, query or fragment.",
        ));
    }
    Ok(u)
}
pub async fn save(app: &AppHandle, mut settings: Settings, key: Option<String>) -> Result<()> {
    settings.endpoint = settings.endpoint.trim().trim_end_matches('/').to_owned();
    if !settings.endpoint.is_empty() {
        endpoint(&settings.endpoint)?;
    }
    settings.model = settings.model.trim().to_owned();
    choice(&settings.whisper_model, &["base.en", "small.en"])?;
    if settings.model.len() > 200 || settings.microphone.len() > 500 {
        return Err(AppError::validation("Voice settings are too long."));
    }
    if let Some(key) = key {
        if key.len() > 4096 {
            return Err(AppError::validation("API key is too long."));
        }
        let path = root(app)?.join("credential.bin");
        if key.trim().is_empty() {
            if path.exists() {
                std::fs::remove_file(path)?;
            }
        } else {
            std::fs::write(path, protect(key.trim().as_bytes(), false)?)?;
        }
    }
    let pool = crate::workspace::pool(app).await?;
    sqlx::query("INSERT INTO settings(key,value) VALUES('voice',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value").bind(serde_json::to_string(&settings).unwrap()).execute(&pool).await?;
    Ok(())
}
pub fn key(app: &AppHandle) -> Result<String> {
    let bytes = std::fs::read(root(app)?.join("credential.bin"))
        .map_err(|_| AppError::validation("Save an API key in Voice & AI settings."))?;
    String::from_utf8(protect(&bytes, true)?)
        .map_err(|_| AppError::validation("Save the API key again."))
}
#[cfg(windows)]
pub(super) fn protect(bytes: &[u8], decrypt: bool) -> Result<Vec<u8>> {
    use windows_sys::Win32::{Foundation::LocalFree, Security::Cryptography::*};
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    unsafe {
        let ok = if decrypt {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptProtectData(
                &input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok == 0 {
            return Err(AppError::new("Credential", "Windows could not protect or unlock this API key. Save it again for this Windows user."));
        }
        let result = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(output.pbData as _);
        Ok(result)
    }
}
#[cfg(not(windows))]
pub(super) fn protect(_: &[u8], _: bool) -> Result<Vec<u8>> {
    Err(AppError::validation(
        "Voice credentials currently require Windows.",
    ))
}
