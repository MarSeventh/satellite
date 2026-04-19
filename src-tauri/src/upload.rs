use crate::config;
use crate::db::Database;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, State};

static UPLOADING: AtomicBool = AtomicBool::new(false);

// Chunk size: 4MB
const CHUNK_SIZE: usize = 4 * 1024 * 1024;
// Files larger than 5MB use chunked upload
const CHUNK_THRESHOLD: usize = 5 * 1024 * 1024;
// HuggingFace files larger than 20MB should use direct upload
const HF_DIRECT_THRESHOLD: usize = 20 * 1024 * 1024;

#[derive(Serialize, Clone)]
pub struct UploadProgress {
    pub filename: String,
    pub progress: f64, // 0.0 - 1.0
    pub current: usize,
    pub total: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UploadResult {
    pub filename: String,
    pub url: String,
}

// CloudFlare ImgBed / Telegraph-style response: [{\"src\": \"/file/xxx\"}]
#[derive(Deserialize, Debug)]
struct CfResponseItem {
    src: Option<String>,
    url: Option<String>,
    #[serde(rename = "fileUrl")]
    file_url: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ChunkInitResponse {
    #[serde(rename = "uploadId")]
    upload_id: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
struct ChannelInfo {
    name: String,
}

#[derive(Deserialize, Debug, Default)]
struct ChannelsResponse {
    #[serde(default)]
    telegram: Vec<ChannelInfo>,
    #[serde(default)]
    cfr2: Vec<ChannelInfo>,
    #[serde(default)]
    s3: Vec<ChannelInfo>,
    #[serde(default)]
    discord: Vec<ChannelInfo>,
    #[serde(default)]
    huggingface: Vec<ChannelInfo>,
}

#[derive(Clone, Debug)]
struct UploadChannelSelection {
    upload_channel: String,
    channel_name: Option<String>,
}

#[derive(Deserialize, Debug)]
struct HfUploadAction {
    href: String,
    #[serde(default)]
    header: HashMap<String, Value>,
}

#[derive(Deserialize, Debug)]
struct HfGetUploadUrlResponse {
    #[serde(rename = "fullId")]
    full_id: String,
    #[serde(rename = "filePath")]
    file_path: String,
    #[serde(rename = "channelName")]
    channel_name: Option<String>,
    #[serde(rename = "needsLfs", default = "default_true")]
    needs_lfs: bool,
    #[serde(rename = "alreadyExists", default)]
    already_exists: bool,
    oid: Option<String>,
    #[serde(rename = "uploadAction")]
    upload_action: Option<HfUploadAction>,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize, Debug)]
struct HfCommitUploadResponse {
    src: Option<String>,
    url: Option<String>,
    #[serde(rename = "fileUrl")]
    file_url: Option<String>,
}

fn parse_upload_url(base_url: &str, item: &CfResponseItem) -> String {
    let base = base_url.trim_end_matches('/');
    if let Some(ref url) = item.url {
        if url.starts_with("http") {
            return url.clone();
        }
        return format!("{}{}", base, url);
    }
    if let Some(ref src) = item.src {
        if src.starts_with("http") {
            return src.clone();
        }
        return format!("{}{}", base, src);
    }
    if let Some(ref file_url) = item.file_url {
        if file_url.starts_with("http") {
            return file_url.clone();
        }
        return format!("{}{}", base, file_url);
    }
    String::new()
}

fn parse_response_url(base_url: &str, body: &str) -> Result<String, String> {
    // Try parsing as array first, then as single object
    let url = if let Ok(items) = serde_json::from_str::<Vec<CfResponseItem>>(body) {
        items
            .first()
            .map(|i| parse_upload_url(base_url, i))
            .unwrap_or_default()
    } else if let Ok(item) = serde_json::from_str::<CfResponseItem>(body) {
        parse_upload_url(base_url, &item)
    } else {
        body.trim().trim_matches('"').to_string()
    };

    if url.is_empty() {
        Err(format!("Could not parse upload response: {}", body))
    } else {
        Ok(url)
    }
}

#[tauri::command]
pub async fn upload_files(
    app: AppHandle,
    db: State<'_, Database>,
    file_paths: Vec<String>,
) -> Result<Vec<UploadResult>, String> {
    let cfg = config::load_config();
    if cfg.base_url.is_empty() {
        return Err("API Endpoint not configured".into());
    }

    // Prevent concurrent uploads
    if UPLOADING.swap(true, Ordering::SeqCst) {
        return Ok(vec![]);
    }

    let result = do_upload(app, db, file_paths, cfg).await;
    UPLOADING.store(false, Ordering::SeqCst);
    result
}

async fn do_upload(
    app: AppHandle,
    db: State<'_, Database>,
    file_paths: Vec<String>,
    cfg: config::AppConfig,
) -> Result<Vec<UploadResult>, String> {
    let client = config::build_http_client(&cfg)?;
    let total = file_paths.len();
    let mut results: Vec<UploadResult> = Vec::with_capacity(total);
    let upload_url = format!("{}/upload", cfg.base_url.trim_end_matches('/'));
    let configured_channel = configured_upload_channel(&cfg);
    let detected_channel = if configured_channel.is_some() {
        None
    } else {
        detect_upload_channel(&client, &cfg).await
    };

    for (idx, path_str) in file_paths.iter().enumerate() {
        let file_path = PathBuf::from(path_str);
        let filename = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Emit start progress
        emit_progress(&app, &filename, idx as f64 / total as f64, idx + 1, total);

        let file_bytes = tokio::fs::read(&file_path)
            .await
            .map_err(|e| format!("Failed to read {}: {}", filename, e))?;

        let mime = mime_from_ext(&filename);

        let selected_channel = configured_channel.as_ref().or(detected_channel.as_ref());
        let is_hf_channel = selected_channel
            .as_ref()
            .map(|c| c.upload_channel.as_str() == "huggingface")
            .unwrap_or(false);

        let url = if is_hf_channel && file_bytes.len() >= HF_DIRECT_THRESHOLD {
            hf_direct_upload(
                &app,
                &client,
                &cfg,
                &filename,
                &mime,
                &file_bytes,
                selected_channel,
                idx,
                total,
            )
            .await?
        } else if file_bytes.len() >= CHUNK_THRESHOLD && !is_hf_channel {
            // Chunked upload for large files
            chunked_upload(
                &app,
                &client,
                &upload_url,
                &cfg,
                &filename,
                &mime,
                &file_bytes,
                selected_channel,
                idx,
                total,
            )
            .await?
        } else {
            // Direct upload for small files
            emit_progress(
                &app,
                &filename,
                (idx as f64 + 0.3) / total as f64,
                idx + 1,
                total,
            );

            let part = multipart::Part::bytes(file_bytes)
                .file_name(filename.clone())
                .mime_str(&mime)
                .map_err(|e| e.to_string())?;
            let form = multipart::Form::new().part("file", part);

            let mut req = client.post(&upload_url).multipart(form);
            if !cfg.auth_token.is_empty() {
                req = req.header("Authorization", format!("Bearer {}", cfg.auth_token));
            }
            req = req.query(&build_upload_query(
                &cfg,
                selected_channel,
                None,
                false,
                false,
                false,
            ));

            let resp = req
                .send()
                .await
                .map_err(|e| format!("Upload failed: {}", e))?;

            emit_progress(
                &app,
                &filename,
                (idx as f64 + 0.8) / total as f64,
                idx + 1,
                total,
            );

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(format!("Upload failed ({}): {}", status, body));
            }

            let body = resp.text().await.map_err(|e| e.to_string())?;
            parse_response_url(&cfg.base_url, &body)?
        };

        // Save to local database
        let _ = db.insert(&filename, &url, Some(&url));

        results.push(UploadResult {
            filename: filename.clone(),
            url,
        });

        // Emit completion progress
        emit_progress(
            &app,
            &filename,
            (idx + 1) as f64 / total as f64,
            idx + 1,
            total,
        );
    }

    let _ = app.emit("upload-complete", &results);
    Ok(results)
}

async fn chunked_upload(
    app: &AppHandle,
    client: &reqwest::Client,
    upload_url: &str,
    cfg: &config::AppConfig,
    filename: &str,
    mime: &str,
    file_bytes: &[u8],
    channel: Option<&UploadChannelSelection>,
    file_idx: usize,
    file_total: usize,
) -> Result<String, String> {
    let total_chunks = (file_bytes.len() + CHUNK_SIZE - 1) / CHUNK_SIZE;

    // Step 1: Initialize chunked upload
    let init_form = multipart::Form::new()
        .text("initChunked", "true")
        .text("totalChunks", total_chunks.to_string())
        .text("originalFileName", filename.to_string())
        .text("originalFileType", mime.to_string());

    let mut req = client.post(upload_url).multipart(init_form);
    if !cfg.auth_token.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", cfg.auth_token));
    }
    req = req.query(&build_upload_query(cfg, channel, None, true, false, false));

    let resp = req
        .send()
        .await
        .map_err(|e| format!("Chunk init failed: {}", e))?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Chunk init failed: {}", body));
    }

    let init_body = resp.text().await.map_err(|e| e.to_string())?;
    let init_resp: ChunkInitResponse = serde_json::from_str(&init_body)
        .map_err(|_| format!("Failed to parse chunk init response: {}", init_body))?;
    let upload_id = init_resp
        .upload_id
        .ok_or_else(|| format!("No uploadId in response: {}", init_body))?;

    // Step 2: Upload each chunk
    for chunk_idx in 0..total_chunks {
        let start = chunk_idx * CHUNK_SIZE;
        let end = std::cmp::min(start + CHUNK_SIZE, file_bytes.len());
        let chunk_data = file_bytes[start..end].to_vec();

        let chunk_part = multipart::Part::bytes(chunk_data)
            .file_name(filename.to_string())
            .mime_str(mime)
            .map_err(|e| e.to_string())?;

        let chunk_form = multipart::Form::new()
            .text("chunked", "true")
            .text("uploadId", upload_id.clone())
            .text("chunkIndex", chunk_idx.to_string())
            .text("totalChunks", total_chunks.to_string())
            .text("originalFileName", filename.to_string())
            .text("originalFileType", mime.to_string())
            .part("file", chunk_part);

        let mut req = client.post(upload_url).multipart(chunk_form);
        if !cfg.auth_token.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", cfg.auth_token));
        }
        req = req.query(&build_upload_query(cfg, channel, None, false, true, false));

        let resp = req
            .send()
            .await
            .map_err(|e| format!("Chunk {} upload failed: {}", chunk_idx, e))?;
        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Chunk {} failed: {}", chunk_idx, body));
        }

        // Emit per-chunk progress
        let chunk_progress = (chunk_idx + 1) as f64 / total_chunks as f64;
        let file_progress = (file_idx as f64 + chunk_progress * 0.9) / file_total as f64;
        emit_progress(app, filename, file_progress, file_idx + 1, file_total);
    }

    // Step 3: Merge chunks
    let merge_form = multipart::Form::new()
        .text("chunked", "true")
        .text("merge", "true")
        .text("uploadId", upload_id)
        .text("totalChunks", total_chunks.to_string())
        .text("originalFileName", filename.to_string())
        .text("originalFileType", mime.to_string());

    let mut req = client.post(upload_url).multipart(merge_form);
    if !cfg.auth_token.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", cfg.auth_token));
    }
    req = req.query(&build_upload_query(
        cfg,
        channel,
        Some(true),
        false,
        true,
        true,
    ));

    let resp = req
        .send()
        .await
        .map_err(|e| format!("Chunk merge failed: {}", e))?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Chunk merge failed: {}", body));
    }

    let body = resp.text().await.map_err(|e| e.to_string())?;
    parse_response_url(&cfg.base_url, &body)
}

fn configured_upload_channel(cfg: &config::AppConfig) -> Option<UploadChannelSelection> {
    let normalized = match cfg.upload_channel.trim().to_ascii_lowercase().as_str() {
        "" => return None,
        "huggingface" => "huggingface",
        "telegram" => "telegram",
        "cfr2" => "cfr2",
        "s3" => "s3",
        "discord" => "discord",
        _ => return None,
    };

    let channel_name = cfg.channel_name.trim();
    Some(UploadChannelSelection {
        upload_channel: normalized.to_string(),
        channel_name: if channel_name.is_empty() {
            None
        } else {
            Some(channel_name.to_string())
        },
    })
}

async fn detect_upload_channel(
    client: &reqwest::Client,
    cfg: &config::AppConfig,
) -> Option<UploadChannelSelection> {
    let channels_url = format!("{}/api/channels", cfg.base_url.trim_end_matches('/'));
    let mut req = client.get(channels_url);
    if !cfg.auth_token.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", cfg.auth_token));
    }

    let resp = req.send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let channels = resp.json::<ChannelsResponse>().await.ok()?;

    let other_enabled = !channels.telegram.is_empty()
        || !channels.cfr2.is_empty()
        || !channels.s3.is_empty()
        || !channels.discord.is_empty();
    if !channels.huggingface.is_empty() && !other_enabled {
        return Some(UploadChannelSelection {
            upload_channel: "huggingface".to_string(),
            channel_name: channels
                .huggingface
                .first()
                .map(|c| c.name.clone())
                .filter(|s| !s.is_empty()),
        });
    }

    None
}

fn build_upload_query(
    cfg: &config::AppConfig,
    channel: Option<&UploadChannelSelection>,
    include_upload_folder: Option<bool>,
    init_chunked: bool,
    chunked: bool,
    merge: bool,
) -> Vec<(&'static str, String)> {
    let mut query: Vec<(&'static str, String)> = Vec::new();

    if init_chunked {
        query.push(("initChunked", "true".to_string()));
    }
    if chunked {
        query.push(("chunked", "true".to_string()));
    }
    if merge {
        query.push(("merge", "true".to_string()));
    }
    let should_include_upload_folder = include_upload_folder.unwrap_or(true);
    if should_include_upload_folder && !cfg.upload_folder.is_empty() {
        query.push(("uploadFolder", cfg.upload_folder.clone()));
    }

    if let Some(ch) = channel {
        query.push(("uploadChannel", ch.upload_channel.clone()));
        if let Some(name) = &ch.channel_name {
            if !name.is_empty() {
                query.push(("channelName", name.clone()));
            }
        }
    }

    query
}

fn value_as_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

async fn hf_direct_upload(
    app: &AppHandle,
    client: &reqwest::Client,
    cfg: &config::AppConfig,
    filename: &str,
    mime: &str,
    file_bytes: &[u8],
    channel: Option<&UploadChannelSelection>,
    file_idx: usize,
    file_total: usize,
) -> Result<String, String> {
    let mut hasher = Sha256::new();
    hasher.update(file_bytes);
    let sha256 = format!("{:x}", hasher.finalize());
    let sample_len = std::cmp::min(512, file_bytes.len());
    let file_sample = BASE64_STANDARD.encode(&file_bytes[..sample_len]);

    let get_upload_url = format!(
        "{}/upload/huggingface/getUploadUrl",
        cfg.base_url.trim_end_matches('/')
    );

    let mut get_req = client.post(&get_upload_url).json(&json!({
        "fileName": filename,
        "fileType": mime,
        "fileSize": file_bytes.len(),
        "sha256": sha256,
        "fileSample": file_sample,
        "channelName": channel.and_then(|c| c.channel_name.clone()),
        "uploadFolder": if cfg.upload_folder.is_empty() { Value::Null } else { Value::String(cfg.upload_folder.clone()) }
    }));
    if !cfg.auth_token.is_empty() {
        get_req = get_req.header("Authorization", format!("Bearer {}", cfg.auth_token));
    }
    let get_resp = get_req
        .send()
        .await
        .map_err(|e| format!("HF getUploadUrl failed: {}", e))?;
    if !get_resp.status().is_success() {
        let status = get_resp.status();
        let body = get_resp.text().await.unwrap_or_default();
        return Err(format!("HF getUploadUrl failed ({}): {}", status, body));
    }
    let get_resp_json = get_resp
        .json::<HfGetUploadUrlResponse>()
        .await
        .map_err(|e| format!("HF getUploadUrl parse failed: {}", e))?;

    emit_progress(
        app,
        filename,
        (file_idx as f64 + 0.35) / file_total as f64,
        file_idx + 1,
        file_total,
    );

    if get_resp_json.needs_lfs && !get_resp_json.already_exists {
        let upload_action = get_resp_json
            .upload_action
            .as_ref()
            .ok_or_else(|| "HF uploadAction missing".to_string())?;

        if let Some(chunk_size_value) = upload_action.header.get("chunk_size") {
            let chunk_size = value_as_string(chunk_size_value)
                .and_then(|s| s.parse::<usize>().ok())
                .ok_or_else(|| "HF chunk_size invalid".to_string())?;
            if chunk_size == 0 {
                return Err("HuggingFace chunk_size must be positive, got 0".to_string());
            }

            let total_parts = (file_bytes.len() + chunk_size - 1) / chunk_size;
            let mut parts = Vec::with_capacity(total_parts);

            for part_idx in 0..total_parts {
                let part_number = part_idx + 1;
                let key = part_number.to_string();
                let part_url = upload_action
                    .header
                    .get(&key)
                    .and_then(value_as_string)
                    .ok_or_else(|| {
                        format!("HF part upload url missing for part {}", part_number)
                    })?;

                let start = part_idx * chunk_size;
                let end = std::cmp::min(start + chunk_size, file_bytes.len());
                let chunk = file_bytes[start..end].to_vec();

                let part_resp = client
                    .put(&part_url)
                    .body(chunk)
                    .send()
                    .await
                    .map_err(|e| format!("HF part {} upload failed: {}", part_number, e))?;
                if !part_resp.status().is_success() {
                    let status = part_resp.status();
                    let body = part_resp.text().await.unwrap_or_default();
                    return Err(format!(
                        "HF part {} upload failed ({}): {}",
                        part_number, status, body
                    ));
                }

                let etag = part_resp
                    .headers()
                    .get("etag")
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| format!("HF part {} ETag missing", part_number))?
                    .to_string();
                parts.push(json!({
                    "partNumber": part_number,
                    "etag": etag
                }));

                let part_progress = (part_number as f64 / total_parts as f64) * 0.4;
                emit_progress(
                    app,
                    filename,
                    (file_idx as f64 + 0.35 + part_progress) / file_total as f64,
                    file_idx + 1,
                    file_total,
                );
            }

            let complete_payload = json!({
                "oid": get_resp_json.oid.clone().unwrap_or_else(|| sha256.clone()),
                "parts": parts
            });
            let complete_resp = client
                .post(&upload_action.href)
                .header("Content-Type", "application/vnd.git-lfs+json")
                .json(&complete_payload)
                .send()
                .await
                .map_err(|e| format!("HF multipart complete failed: {}", e))?;
            if !complete_resp.status().is_success() {
                let status = complete_resp.status();
                let body = complete_resp.text().await.unwrap_or_default();
                return Err(format!(
                    "HF multipart complete failed ({}): {}",
                    status, body
                ));
            }
        } else {
            let mut put_req = client.put(&upload_action.href).body(file_bytes.to_vec());
            for (k, v) in &upload_action.header {
                if k == "chunk_size" || k.chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }
                if let Some(value) = value_as_string(v) {
                    put_req = put_req.header(k, value);
                }
            }
            let put_resp = put_req
                .send()
                .await
                .map_err(|e| format!("HF direct upload failed: {}", e))?;
            if !put_resp.status().is_success() {
                let status = put_resp.status();
                let body = put_resp.text().await.unwrap_or_default();
                return Err(format!("HF direct upload failed ({}): {}", status, body));
            }
        }
    }

    emit_progress(
        app,
        filename,
        (file_idx as f64 + 0.8) / file_total as f64,
        file_idx + 1,
        file_total,
    );

    let commit_url = format!(
        "{}/upload/huggingface/commitUpload",
        cfg.base_url.trim_end_matches('/')
    );
    let commit_payload = json!({
        "fullId": get_resp_json.full_id,
        "filePath": get_resp_json.file_path,
        "sha256": sha256,
        "fileSize": file_bytes.len(),
        "fileName": filename,
        "fileType": mime,
        "channelName": get_resp_json.channel_name.or_else(|| channel.and_then(|c| c.channel_name.clone()))
    });
    let mut commit_req = client.post(&commit_url).json(&commit_payload);
    if !cfg.auth_token.is_empty() {
        commit_req = commit_req.header("Authorization", format!("Bearer {}", cfg.auth_token));
    }
    let commit_resp = commit_req
        .send()
        .await
        .map_err(|e| format!("HF commitUpload failed: {}", e))?;
    if !commit_resp.status().is_success() {
        let status = commit_resp.status();
        let body = commit_resp.text().await.unwrap_or_default();
        return Err(format!("HF commitUpload failed ({}): {}", status, body));
    }

    let commit_body = commit_resp.text().await.map_err(|e| e.to_string())?;
    if let Ok(item) = serde_json::from_str::<HfCommitUploadResponse>(&commit_body) {
        let cf_item = CfResponseItem {
            src: item.src,
            url: item.url,
            file_url: item.file_url,
        };
        let parsed = parse_upload_url(&cfg.base_url, &cf_item);
        if !parsed.is_empty() {
            return Ok(parsed);
        }
    }
    parse_response_url(&cfg.base_url, &commit_body)
}

fn emit_progress(app: &AppHandle, filename: &str, progress: f64, current: usize, total: usize) {
    let _ = app.emit(
        "upload-progress",
        UploadProgress {
            filename: filename.to_string(),
            progress,
            current,
            total,
        },
    );
}

fn mime_from_ext(filename: &str) -> String {
    let ext = filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        "tiff" | "tif" => "image/tiff",
        "mp4" => "video/mp4",
        "mp3" => "audio/mpeg",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
    .to_string()
}
