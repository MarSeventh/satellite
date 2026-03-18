use crate::config;
use crate::db::Database;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

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

// CloudFlare ImgBed / Telegraph-style response: [{"src": "/file/xxx"}]
#[derive(Deserialize, Debug)]
struct CfResponseItem {
    src: Option<String>,
    url: Option<String>,
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

    let client = reqwest::Client::new();
    let total = file_paths.len();
    let mut results: Vec<UploadResult> = Vec::with_capacity(total);

    for (idx, path_str) in file_paths.iter().enumerate() {
        let file_path = PathBuf::from(path_str);
        let filename = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Emit start progress
        let _ = app.emit(
            "upload-progress",
            UploadProgress {
                filename: filename.clone(),
                progress: 0.0,
                current: idx + 1,
                total,
            },
        );

        let file_bytes = tokio::fs::read(&file_path)
            .await
            .map_err(|e| format!("Failed to read {}: {}", filename, e))?;

        let mime = mime_from_ext(&filename);
        let part = multipart::Part::bytes(file_bytes)
            .file_name(filename.clone())
            .mime_str(&mime)
            .map_err(|e| e.to_string())?;

        let form = multipart::Form::new().part("file", part);

        let upload_url = format!("{}/upload", cfg.base_url.trim_end_matches('/'));

        let mut req = client.post(&upload_url).multipart(form);
        if !cfg.auth_token.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", cfg.auth_token));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| format!("Upload request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Upload failed ({}): {}", status, body));
        }

        let body = resp.text().await.map_err(|e| e.to_string())?;

        // Try parsing as array first, then as single object
        let url = if let Ok(items) = serde_json::from_str::<Vec<CfResponseItem>>(&body) {
            items
                .first()
                .map(|i| parse_upload_url(&cfg.base_url, i))
                .unwrap_or_default()
        } else if let Ok(item) = serde_json::from_str::<CfResponseItem>(&body) {
            parse_upload_url(&cfg.base_url, &item)
        } else {
            // Fallback: try to extract any URL-like string from the response
            body.trim().trim_matches('"').to_string()
        };

        if url.is_empty() {
            return Err(format!("Could not parse upload response: {}", body));
        }

        // Save to local database
        let _ = db.insert(&filename, &url, Some(&url));

        let result = UploadResult {
            filename: filename.clone(),
            url,
        };
        results.push(result);

        // Emit completion progress
        let _ = app.emit(
            "upload-progress",
            UploadProgress {
                filename,
                progress: 1.0,
                current: idx + 1,
                total,
            },
        );
    }

    // Signal all uploads complete
    let _ = app.emit("upload-complete", &results);

    Ok(results)
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
        _ => "application/octet-stream",
    }
    .to_string()
}
