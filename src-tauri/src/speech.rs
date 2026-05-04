use base64::Engine;
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde_json::{json, Value};
use sha1::Sha1;
use std::collections::BTreeMap;

type HmacSha1 = Hmac<Sha1>;

const NLS_META_URL: &str = "https://nls-meta.cn-shanghai.aliyuncs.com";
const NLS_GATEWAY: &str = "https://nls-gateway-cn-shanghai.aliyuncs.com";

/// URL percent-encode per Alibaba Cloud POP API spec.
fn percent_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

/// Get a temporary token using Alibaba Cloud RPC-style POP API (GET with HMAC-SHA1).
async fn get_token(
    access_key_id: &str,
    access_key_secret: &str,
) -> Result<String, String> {
    let client = Client::new();

    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let signature_nonce = format!(
        "claudio-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("系统时间错误: {}", e))?
            .as_nanos()
    );

    // Build sorted params (BTreeMap sorts by key alphabetically)
    let mut params: BTreeMap<&str, String> = BTreeMap::new();
    params.insert("AccessKeyId", access_key_id.to_string());
    params.insert("Action", "CreateToken".to_string());
    params.insert("Format", "JSON".to_string());
    params.insert("RegionId", "cn-shanghai".to_string());
    params.insert("SignatureMethod", "HMAC-SHA1".to_string());
    params.insert("SignatureNonce", signature_nonce);
    params.insert("SignatureVersion", "1.0".to_string());
    params.insert("Timestamp", timestamp);
    params.insert("Version", "2019-02-28".to_string());

    // Build canonicalized query string from sorted params
    let canonicalized_query: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    // RPC-style string-to-sign: GET&PercentEncode("/")&PercentEncode(canonicalized_query)
    let string_to_sign = format!(
        "GET&{}&{}",
        percent_encode("/"),
        percent_encode(&canonicalized_query)
    );

    // HMAC-SHA1 with key = AccessKeySecret + "&"
    let signing_key = format!("{}&", access_key_secret);
    let mut mac = HmacSha1::new_from_slice(signing_key.as_bytes())
        .map_err(|e| format!("HMAC 初始化失败: {}", e))?;
    mac.update(string_to_sign.as_bytes());
    let signature_bytes = mac.finalize().into_bytes();
    let signature_base64 = base64::engine::general_purpose::STANDARD.encode(&signature_bytes);

    // Build query params including Signature (raw values, reqwest handles encoding)
    let mut query_params: Vec<(&str, &str)> = Vec::new();
    for (k, v) in &params {
        query_params.push((k, v.as_str()));
    }
    query_params.push(("Signature", signature_base64.as_str()));

    let debug_url = reqwest::Url::parse_with_params(NLS_META_URL, &query_params)
        .map_err(|e| format!("URL 构建失败: {}", e))?;
    eprintln!("[speech] GET {}",
        debug_url.as_str().replace(access_key_id, "***AKID***"));

    // RPC-style GET request — no body, no Date header
    let resp = client
        .get(NLS_META_URL)
        .query(&query_params)
        .send()
        .await
        .map_err(|e| format!("获取 ISI Token 失败: {}", e))?;

    let status = resp.status();
    let raw = resp
        .text()
        .await
        .map_err(|e| format!("读取 Token 响应失败: {}", e))?;

    eprintln!("[speech] response status={} body={}", status, raw);

    if !status.is_success() {
        return Err(format!("获取 Token 失败 ({}): {}", status, raw));
    }

    let body: Value = serde_json::from_str(&raw)
        .map_err(|e| format!("JSON 解析失败: {} | 原始响应: {}", e, raw))?;

    // Try standard Token.Id path, fallback to Data.Token
    let token = body
        .get("Token")
        .and_then(|t| t.get("Id"))
        .and_then(|t| t.as_str())
        .or_else(|| {
            body.get("Data")
                .and_then(|d| d.get("Token"))
                .and_then(|t| t.as_str())
        })
        .ok_or_else(|| {
            format!(
                "Token 响应格式异常: {}",
                serde_json::to_string(&body).unwrap_or_default()
            )
        })?;

    Ok(token.to_string())
}

/// ISI RESTful API only supports traditional ISI voices, NOT CosyVoice voices.
/// CosyVoice voices (from 百炼) use lowercase names like "longxiaoxia", "longxiaochun".
/// Map them to a safe default ISI voice.
fn sanitize_voice(voice: &str) -> &str {
    if voice.starts_with("long") || voice.starts_with("cosyvoice") {
        "zhixiaoxia"
    } else {
        voice
    }
}

/// Synthesize speech from text using Alibaba Cloud ISI TTS.
/// Returns base64-encoded WAV audio.
pub async fn synthesize_speech(
    text: &str,
    access_key_id: &str,
    access_key_secret: &str,
    app_key: &str,
    voice: &str,
) -> Result<String, String> {
    if app_key.is_empty() {
        return Err("请先设置 ISI AccessKey 和 AppKey".to_string());
    }

    let token = get_token(access_key_id, access_key_secret).await?;

    let client = Client::new();
    let url = format!("{}/stream/v1/tts", NLS_GATEWAY);

    let voice = sanitize_voice(voice);
    let body = json!({
        "appkey": app_key,
        "token": token,
        "text": text,
        "format": "wav",
        "sample_rate": 16000,
        "voice": voice,
        "speech_rate": 100,
    });

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("TTS 请求失败: {}", e))?;

    let status = resp.status();
    let content_type = resp
        .headers()
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let is_audio = content_type.contains("audio/");

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("读取 TTS 响应失败: {}", e))?
        .to_vec();

    if !status.is_success() || !is_audio {
        let err_text = String::from_utf8_lossy(&bytes);
        eprintln!("[speech] TTS error: status={} body={}", status, err_text);
        // Try to parse JSON error message
        let msg = serde_json::from_str::<Value>(&err_text)
            .ok()
            .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(|s| s.to_string()))
            .unwrap_or_else(|| err_text.to_string());
        return Err(format!("TTS API 错误 ({}): {}", status, msg));
    }

    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(encoded)
}

/// Recognize speech from WAV audio using Alibaba Cloud ISI ASR.
/// Returns the transcribed text.
pub async fn recognize_speech(
    audio_data: Vec<u8>,
    access_key_id: &str,
    access_key_secret: &str,
    app_key: &str,
) -> Result<String, String> {
    if app_key.is_empty() {
        return Err("请先设置 ISI AccessKey 和 AppKey".to_string());
    }

    let token = get_token(access_key_id, access_key_secret).await?;

    let client = Client::new();
    let url = format!(
        "{}/stream/v1/asr?appkey={}&format=wav",
        NLS_GATEWAY, app_key,
    );

    let resp = client
        .post(&url)
        .header("Content-Type", "application/octet-stream")
        .header("X-NLS-Token", &token)
        .body(audio_data)
        .send()
        .await
        .map_err(|e| format!("ASR 请求失败: {}", e))?;

    let status = resp.status();
    let body: Value = resp
        .json()
        .await
        .map_err(|e| format!("解析 ASR 响应失败: {}", e))?;

    eprintln!("[speech] ASR response: status={} body={}", status, body);

    if !status.is_success() {
        let err_msg = body
            .get("error_message")
            .or_else(|| body.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("未知错误");
        return Err(format!("ASR API 错误: {}", err_msg));
    }

    // Flat format: {"status": 200, "result": "...", "request_id": "..."}
    // Also try nested format: {"header": {"status": 20000000}, "payload": {"result": "..."}}
    let text = body
        .get("result")
        .and_then(|t| t.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            body.get("payload")
                .and_then(|p| p.get("result"))
                .and_then(|t| t.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
        })
        .ok_or_else(|| {
            format!(
                "ASR 响应为空或未找到识别结果: {}",
                serde_json::to_string(&body).unwrap_or_default()
            )
        })?;

    Ok(text)
}
