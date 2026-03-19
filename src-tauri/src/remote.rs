use crate::config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ListResponse {
    #[serde(default)]
    pub files: Vec<serde_json::Value>,
    #[serde(default)]
    pub directories: Vec<String>,
    #[serde(rename = "totalCount", default)]
    pub total_count: u32,
    #[serde(rename = "returnedCount", default)]
    pub returned_count: u32,
}

#[tauri::command]
pub async fn list_remote_files(
    start: u32,
    count: u32,
    dir: String,
    search: String,
) -> Result<ListResponse, String> {
    let cfg = config::load_config();
    if cfg.base_url.is_empty() {
        return Err("API Endpoint 未配置".into());
    }
    if cfg.auth_token.is_empty() {
        return Err("需要 Auth Token 才能访问远程文件列表".into());
    }

    let client = reqwest::Client::new();
    let url = format!("{}/api/manage/list", cfg.base_url.trim_end_matches('/'));

    let mut query_params: Vec<(&str, String)> = vec![
        ("start", start.to_string()),
        ("count", count.to_string()),
    ];
    if !dir.is_empty() {
        query_params.push(("dir", dir));
    }
    if !search.is_empty() {
        query_params.push(("search", search));
    }

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", cfg.auth_token))
        .query(&query_params)
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("列表请求失败 ({}): {}", status, body));
    }

    resp.json::<ListResponse>()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))
}

#[tauri::command]
pub async fn delete_remote_file(path: String) -> Result<bool, String> {
    let cfg = config::load_config();
    if cfg.base_url.is_empty() || cfg.auth_token.is_empty() {
        return Err("需要配置 API Endpoint 和 Auth Token".into());
    }

    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/manage/delete/{}",
        cfg.base_url.trim_end_matches('/'),
        path.trim_start_matches('/')
    );

    let resp = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", cfg.auth_token))
        .send()
        .await
        .map_err(|e| format!("删除请求失败: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("删除失败 ({}): {}", status, body));
    }

    Ok(true)
}
