use crate::config;
use crate::db::Database;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, State};

static UPLOADING: AtomicBool = AtomicBool::new(false);

// Chunk size: 4MB
const CHUNK_SIZE: usize = 4 * 1024 * 1024;
// Files larger than 5MB use chunked upload
const CHUNK_THRESHOLD: usize = 5 * 1024 * 1024;

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
}

#[derive(Deserialize, Debug)]
struct ChunkInitResponse {
    #[serde(rename = "uploadId")]
    upload_id: Option<String>,
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
    let client = reqwest::Client::new();
    let total = file_paths.len();
    let mut results: Vec<UploadResult> = Vec::with_capacity(total);
    let upload_url = format!("{}/upload", cfg.base_url.trim_end_matches('/'));

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

        let url = if file_bytes.len() >= CHUNK_THRESHOLD {
            // Chunked upload for large files
            chunked_upload(&app, &client, &upload_url, &cfg, &filename, &mime, &file_bytes, idx, total).await?
        } else {
            // Direct upload for small files
            emit_progress(&app, &filename, (idx as f64 + 0.3) / total as f64, idx + 1, total);

            let part = multipart::Part::bytes(file_bytes)
                .file_name(filename.clone())
                .mime_str(&mime)
                .map_err(|e| e.to_string())?;
            let form = multipart::Form::new().part("file", part);

            let mut req = client.post(&upload_url).multipart(form);
            if !cfg.auth_token.is_empty() {
                req = req.header("Authorization", format!("Bearer {}", cfg.auth_token));
            }
            if !cfg.upload_folder.is_empty() {
                req = req.query(&[("uploadFolder", &cfg.upload_folder)]);
            }

            let resp = req.send().await.map_err(|e| format!("Upload failed: {}", e))?;

            emit_progress(&app, &filename, (idx as f64 + 0.8) / total as f64, idx + 1, total);

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
        emit_progress(&app, &filename, (idx + 1) as f64 / total as f64, idx + 1, total);
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
    file_idx: usize,
    file_total: usize,
) -> Result<String, String> {
    let total_chunks = (file_bytes.len() + CHUNK_SIZE - 1) / CHUNK_SIZE;

    // Step 1: Initialize chunked upload
    let mut init_form = multipart::Form::new()
        .text("initChunked", "true")
        .text("totalChunks", total_chunks.to_string())
        .text("originalFileName", filename.to_string())
        .text("originalFileType", mime.to_string());

    // Add a dummy file part for the init request
    let dummy = multipart::Part::bytes(vec![])
        .file_name(filename.to_string())
        .mime_str(mime)
        .map_err(|e| e.to_string())?;
    init_form = init_form.part("file", dummy);

    let mut req = client.post(upload_url).multipart(init_form);
    if !cfg.auth_token.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", cfg.auth_token));
    }
    if !cfg.upload_folder.is_empty() {
        req = req.query(&[("uploadFolder", &cfg.upload_folder)]);
    }

    let resp = req.send().await.map_err(|e| format!("Chunk init failed: {}", e))?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Chunk init failed: {}", body));
    }

    let init_body = resp.text().await.map_err(|e| e.to_string())?;
    let init_resp: ChunkInitResponse = serde_json::from_str(&init_body)
        .map_err(|_| format!("Failed to parse chunk init response: {}", init_body))?;
    let upload_id = init_resp.upload_id
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
        if !cfg.upload_folder.is_empty() {
            req = req.query(&[("uploadFolder", &cfg.upload_folder)]);
        }

        let resp = req.send().await.map_err(|e| format!("Chunk {} upload failed: {}", chunk_idx, e))?;
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
    if !cfg.upload_folder.is_empty() {
        req = req.query(&[("uploadFolder", &cfg.upload_folder)]);
    }

    let resp = req.send().await.map_err(|e| format!("Chunk merge failed: {}", e))?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Chunk merge failed: {}", body));
    }

    let body = resp.text().await.map_err(|e| e.to_string())?;
    parse_response_url(&cfg.base_url, &body)
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
