<script>
  import { invoke } from "@tauri-apps/api/core";
  import { createEventDispatcher } from "svelte";
  import { FORMAT_OPTIONS } from "./formatUrl.js";

  export let config = { base_url: "", auth_token: "", upload_folder: "", auto_copy_format: "raw", show_floating: true };

  const dispatch = createEventDispatcher();

  let baseUrl = config.base_url || "";
  let authToken = config.auth_token || "";
  let uploadFolder = config.upload_folder || "";
  let autoCopyFormat = config.auto_copy_format || "raw";
  let showFloating = config.show_floating !== false;
  let saving = false;
  let saveStatus = "";

  async function handleSave() {
    saving = true;
    saveStatus = "";
    try {
      await invoke("save_config", {
        baseUrl,
        authToken,
        uploadFolder,
        autoCopyFormat,
        showFloating,
      });
      // Toggle floating ball visibility
      await invoke("set_floating_visible", { visible: showFloating });
      saveStatus = "✅ 保存成功";
      dispatch("saved", {
        base_url: baseUrl,
        auth_token: authToken,
        upload_folder: uploadFolder,
        auto_copy_format: autoCopyFormat,
        show_floating: showFloating,
      });
      setTimeout(() => { saveStatus = ""; }, 2000);
    } catch (e) {
      saveStatus = `❌ ${e}`;
    } finally {
      saving = false;
    }
  }
</script>

<div class="settings-wrap">
  <div class="settings-content">
    <div class="section">
      <h3 class="section-title">API 配置</h3>
      <p class="section-desc">配置 CloudFlare ImgBed 或兼容的图床 API</p>

      <div class="form-group">
        <label for="base-url">API Endpoint</label>
        <input
          id="base-url"
          type="text"
          bind:value={baseUrl}
          placeholder="https://your-imgbed.example.com"
          class="input"
        />
        <span class="hint">图床的基础 URL，例如 https://example.com</span>
      </div>

      <div class="form-group">
        <label for="auth-token">Auth Token（可选）</label>
        <input
          id="auth-token"
          type="password"
          bind:value={authToken}
          placeholder="Bearer token（如果需要）"
          class="input"
        />
        <span class="hint">如果图床需要认证，填写 Bearer token</span>
      </div>

      <div class="form-group">
        <label for="upload-folder">默认上传目录（可选）</label>
        <input
          id="upload-folder"
          type="text"
          bind:value={uploadFolder}
          placeholder="例如: photos/2024"
          class="input"
        />
        <span class="hint">上传文件时使用的服务端目录路径，留空为默认目录</span>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">上传设置</h3>

      <div class="form-group">
        <label for="auto-copy-format">自动复制格式</label>
        <select id="auto-copy-format" bind:value={autoCopyFormat} class="input">
          {#each FORMAT_OPTIONS as opt}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </select>
        <span class="hint">上传完成后自动复制到剪贴板的链接格式</span>
      </div>

      <div class="form-group">
        <label class="toggle-label">
          <input type="checkbox" bind:checked={showFloating} class="toggle-input" />
          <span class="toggle-text">显示悬浮球</span>
        </label>
        <span class="hint">关闭后悬浮球将隐藏，可通过系统托盘操作</span>
      </div>
    </div>

    <div class="form-actions">
      <button class="btn-save" on:click={handleSave} disabled={saving}>
        {#if saving}
          保存中...
        {:else}
          💾 保存配置
        {/if}
      </button>
      {#if saveStatus}
        <span class="save-status">{saveStatus}</span>
      {/if}
    </div>

    <div class="section">
      <h3 class="section-title">关于</h3>
      <div class="about">
        <p><strong>Satellite</strong> 🛰️</p>
        <p class="version">v0.1.6</p>
        <p class="desc">轻量级 CloudFlare ImgBed 桌面客户端</p>
        <p class="desc">基于 Tauri + Svelte + Rust 构建</p>
      </div>
    </div>
  </div>
</div>

<style>
  .settings-wrap {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
  }

  .settings-content {
    max-width: 600px;
    margin: 0 auto;
  }

  .section {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 12px;
    padding: 20px;
    margin-bottom: 16px;
  }

  .section-title {
    margin: 0 0 6px 0;
    font-size: 16px;
    font-weight: 600;
    color: #e2e8f0;
  }

  .section-desc {
    margin: 0 0 20px 0;
    font-size: 13px;
    color: #64748b;
  }

  .form-group {
    margin-bottom: 18px;
  }

  label {
    display: block;
    margin-bottom: 6px;
    font-size: 13px;
    font-weight: 500;
    color: #94a3b8;
  }

  .input {
    width: 100%;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #e2e8f0;
    padding: 10px 12px;
    border-radius: 8px;
    font-size: 13px;
    font-family: inherit;
    transition: all 0.15s;
  }

  select.input {
    cursor: pointer;
    appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12'%3E%3Cpath fill='%2394a3b8' d='M6 8L1 3h10z'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 12px center;
    padding-right: 32px;
  }

  select.input option {
    background: #1a1b23;
    color: #e2e8f0;
  }

  .input:focus {
    outline: none;
    border-color: #89b4fa;
    background: rgba(255, 255, 255, 0.08);
  }

  .input::placeholder {
    color: #475569;
  }

  .hint {
    display: block;
    margin-top: 4px;
    font-size: 11px;
    color: #475569;
  }

  .toggle-label {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    margin-bottom: 0;
  }

  .toggle-input {
    width: 36px;
    height: 20px;
    appearance: none;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 10px;
    position: relative;
    cursor: pointer;
    transition: background 0.2s;
    flex-shrink: 0;
  }

  .toggle-input::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #64748b;
    transition: all 0.2s;
  }

  .toggle-input:checked {
    background: #89b4fa;
  }

  .toggle-input:checked::after {
    left: 18px;
    background: #fff;
  }

  .toggle-text {
    font-size: 13px;
    color: #e2e8f0;
  }

  .form-actions {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
  }

  .btn-save {
    background: #89b4fa;
    color: #1a1b23;
    border: none;
    padding: 10px 20px;
    border-radius: 8px;
    cursor: pointer;
    font-size: 13px;
    font-weight: 600;
    transition: all 0.15s;
  }

  .btn-save:hover:not(:disabled) {
    background: #a3c4fb;
    transform: translateY(-1px);
  }

  .btn-save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .save-status {
    font-size: 13px;
    color: #94a3b8;
  }

  .about {
    color: #64748b;
    font-size: 13px;
    line-height: 1.6;
  }

  .about p {
    margin: 6px 0;
  }

  .about strong {
    color: #e2e8f0;
    font-size: 15px;
  }

  .version {
    color: #475569;
    font-size: 12px;
    font-family: monospace;
  }

  .desc {
    color: #64748b;
  }
</style>
