<script>
  import { invoke } from "@tauri-apps/api/core";
  import { open as shellOpen } from "@tauri-apps/plugin-shell";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import HistoryList from "./HistoryList.svelte";
  import Settings from "./Settings.svelte";
  import RemoteFiles from "./RemoteFiles.svelte";
  import Toast from "./Toast.svelte";
  import { addToast } from "./toastStore.js";
  import { formatUrl } from "./formatUrl.js";

  let activeTab = "history"; // "history" | "settings" | "remote"
  let config = { base_url: "", auth_token: "", upload_folder: "", auto_copy_format: "raw", show_floating: true };
  let isUploading = false;
  let uploadStatus = "";
  let dragOver = false;
  let unlistens = [];

  onMount(async () => {
    config = await invoke("get_config");

    unlistens.push(await listen("upload-progress", (e) => {
      isUploading = true;
      const p = e.payload;
      uploadStatus = `上传中 ${p.current}/${p.total}：${p.filename}`;
    }));

    unlistens.push(await listen("upload-complete", async (e) => {
      isUploading = false;
      uploadStatus = `✅ 已上传 ${e.payload.length} 个文件`;
      setTimeout(() => { uploadStatus = ""; }, 3000);
      // refresh history
      historyKey++;

      // Auto-copy uploaded URLs
      if (config.auto_copy_format && e.payload.length > 0) {
        try {
          const fmt = config.auto_copy_format;
          const text = e.payload
            .map((r) => formatUrl(r.url, r.filename, fmt))
            .join("\n");
          await writeText(text);
          addToast(`已自动复制 ${e.payload.length} 个链接`);
        } catch (_) {}
      }
    }));

    unlistens.push(await listen("tauri://drag-drop", async (e) => {
      dragOver = false;
      if (e.payload?.paths?.length > 0) {
        await uploadFiles(e.payload.paths);
      }
    }));
    unlistens.push(await listen("tauri://drag-over", () => { dragOver = true; }));
    unlistens.push(await listen("tauri://drag-leave", () => { dragOver = false; }));
  });

  onDestroy(() => {
    unlistens.forEach(u => u());
  });

  let historyKey = 0;

  async function uploadFiles(paths) {
    if (!config.base_url) {
      activeTab = "settings";
      uploadStatus = "⚠️ 请先配置 API Endpoint";
      return;
    }
    try {
      isUploading = true;
      await invoke("upload_files", { filePaths: paths });
    } catch (e) {
      isUploading = false;
      uploadStatus = `❌ ${e}`;
      setTimeout(() => { uploadStatus = ""; }, 4000);
    }
  }

  async function handlePickFiles() {
    const selected = await openDialog({ multiple: true, directory: false });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    await uploadFiles(paths);
  }

  async function openAdmin() {
    if (config.base_url) {
      await shellOpen(config.base_url);
    }
  }

  function onConfigSaved(newConfig) {
    config = newConfig;
  }
</script>

<div class="main-wrap" class:drag-active={dragOver}>
  <Toast />

  <!-- Header -->
  <header class="header">
    <div class="header-left">
      <span class="logo">🌙</span>
      <span class="app-name">Satellite</span>
    </div>
    <div class="header-right">
      {#if config.base_url}
        <button class="btn-ghost" on:click={openAdmin} title="打开管理后台">
          🌐 管理后台
        </button>
      {/if}
    </div>
  </header>

  <!-- Upload Bar -->
  <div class="upload-bar" class:drag-over={dragOver}>
    {#if dragOver}
      <div class="drop-hint">松开以上传文件 ⬆️</div>
    {:else if isUploading}
      <div class="upload-status uploading">
        <div class="spinner"></div>
        <span>{uploadStatus}</span>
      </div>
    {:else if uploadStatus}
      <div class="upload-status">{uploadStatus}</div>
    {:else}
      <button class="btn-upload" on:click={handlePickFiles}>
        ＋ 选择文件上传
      </button>
      <span class="drag-hint">或将文件拖到悬浮球</span>
    {/if}
  </div>

  <!-- Tabs -->
  <div class="tabs">
    <button
      class="tab-btn"
      class:active={activeTab === "history"}
      on:click={() => activeTab = "history"}
    >
      📋 上传历史
    </button>
    <button
      class="tab-btn"
      class:active={activeTab === "remote"}
      on:click={() => activeTab = "remote"}
    >
      ☁️ 远程文件
    </button>
    <button
      class="tab-btn"
      class:active={activeTab === "settings"}
      on:click={() => activeTab = "settings"}
    >
      ⚙️ 设置
    </button>
  </div>

  <!-- Content -->
  <div class="content">
    {#if activeTab === "history"}
      {#key historyKey}
        <HistoryList />
      {/key}
    {:else if activeTab === "remote"}
      <RemoteFiles />
    {:else}
      <Settings {config} on:saved={(e) => onConfigSaved(e.detail)} />
    {/if}
  </div>
</div>

<style>
  :global(body) {
    margin: 0;
    overflow: hidden;
    color: #e2e8f0;
    color-scheme: dark;
    font-family: Inter, system-ui, sans-serif;
  }

  .main-wrap {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #1a1b23;
    transition: background 0.2s;
  }

  .main-wrap.drag-active {
    background: #1e2333;
    outline: 2px dashed #89b4fa;
    outline-offset: -4px;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 20px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
    background: rgba(255, 255, 255, 0.03);
    -webkit-app-region: drag;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .logo {
    font-size: 20px;
  }

  .app-name {
    font-size: 16px;
    font-weight: 600;
    letter-spacing: 0.02em;
    color: #e2e8f0;
  }

  .header-right {
    -webkit-app-region: no-drag;
  }

  .btn-ghost {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #94a3b8;
    padding: 6px 12px;
    border-radius: 8px;
    cursor: pointer;
    font-size: 13px;
    transition: all 0.15s;
  }

  .btn-ghost:hover {
    background: rgba(255, 255, 255, 0.1);
    color: #e2e8f0;
  }

  .upload-bar {
    padding: 14px 20px;
    display: flex;
    align-items: center;
    gap: 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
    min-height: 58px;
    transition: background 0.2s;
  }

  .upload-bar.drag-over {
    background: rgba(137, 180, 250, 0.08);
  }

  .drop-hint {
    flex: 1;
    text-align: center;
    color: #89b4fa;
    font-size: 15px;
    font-weight: 500;
  }

  .upload-status {
    display: flex;
    align-items: center;
    gap: 8px;
    color: #94a3b8;
    font-size: 13px;
  }

  .upload-status.uploading {
    color: #89b4fa;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(137, 180, 250, 0.3);
    border-top-color: #89b4fa;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .btn-upload {
    background: #89b4fa;
    color: #1a1b23;
    border: none;
    padding: 8px 16px;
    border-radius: 8px;
    cursor: pointer;
    font-size: 13px;
    font-weight: 600;
    transition: all 0.15s;
    white-space: nowrap;
  }

  .btn-upload:hover {
    background: #a3c4fb;
    transform: translateY(-1px);
  }

  .drag-hint {
    color: #475569;
    font-size: 12px;
  }

  .tabs {
    display: flex;
    padding: 0 20px;
    gap: 4px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
  }

  .tab-btn {
    background: none;
    border: none;
    color: #64748b;
    padding: 10px 12px;
    cursor: pointer;
    font-size: 13px;
    border-bottom: 2px solid transparent;
    transition: all 0.15s;
    margin-bottom: -1px;
  }

  .tab-btn:hover {
    color: #94a3b8;
  }

  .tab-btn.active {
    color: #89b4fa;
    border-bottom-color: #89b4fa;
  }

  .content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
</style>
