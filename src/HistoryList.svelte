<script>
  import { invoke } from "@tauri-apps/api/core";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { onMount } from "svelte";

  let items = [];
  let page = 1;
  let pageSize = 50;
  let total = 0;
  let loading = true;

  onMount(async () => {
    await loadHistory();
  });

  async function loadHistory() {
    loading = true;
    try {
      items = await invoke("get_history", { page, pageSize });
      total = await invoke("get_history_count");
    } catch (e) {
      console.error("Failed to load history:", e);
    } finally {
      loading = false;
    }
  }

  async function copyUrl(url) {
    try {
      await writeText(url);
      // Show brief feedback
      const btn = event.target;
      const orig = btn.textContent;
      btn.textContent = "✓";
      setTimeout(() => { btn.textContent = orig; }, 1000);
    } catch (e) {
      console.error("Copy failed:", e);
    }
  }

  function formatDate(dateStr) {
    const d = new Date(dateStr);
    const now = new Date();
    const diff = now - d;
    const minutes = Math.floor(diff / 60000);
    const hours = Math.floor(diff / 3600000);
    const days = Math.floor(diff / 86400000);

    if (minutes < 1) return "刚刚";
    if (minutes < 60) return `${minutes} 分钟前`;
    if (hours < 24) return `${hours} 小时前`;
    if (days < 7) return `${days} 天前`;
    return d.toLocaleDateString("zh-CN");
  }

  function isImage(filename) {
    const ext = filename.split(".").pop().toLowerCase();
    return ["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "avif"].includes(ext);
  }
</script>

<div class="history-wrap">
  {#if loading}
    <div class="loading">
      <div class="spinner-large"></div>
      <p>加载中...</p>
    </div>
  {:else if items.length === 0}
    <div class="empty">
      <div class="empty-icon">📭</div>
      <p>还没有上传记录</p>
      <p class="empty-hint">拖放文件到悬浮球开始上传</p>
    </div>
  {:else}
    <div class="list">
      {#each items as item (item.id)}
        <div class="item">
          <div class="item-left">
            {#if isImage(item.filename)}
              <div class="thumb">
                <img src={item.url} alt={item.filename} loading="lazy" />
              </div>
            {:else}
              <div class="thumb file-icon">📄</div>
            {/if}
            <div class="item-info">
              <div class="filename" title={item.filename}>{item.filename}</div>
              <div class="meta">
                <span class="time">{formatDate(item.created_at)}</span>
              </div>
            </div>
          </div>
          <div class="item-right">
            <button class="btn-copy" on:click={() => copyUrl(item.url)} title="复制链接">
              📋
            </button>
          </div>
        </div>
      {/each}
    </div>
    <div class="footer">
      <span class="count">共 {total} 条记录</span>
    </div>
  {/if}
</div>

<style>
  .history-wrap {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .loading, .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: #64748b;
  }

  .spinner-large {
    width: 32px;
    height: 32px;
    border: 3px solid rgba(137, 180, 250, 0.2);
    border-top-color: #89b4fa;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .empty-icon {
    font-size: 48px;
    margin-bottom: 12px;
  }

  .empty p {
    margin: 4px 0;
    font-size: 14px;
  }

  .empty-hint {
    color: #475569;
    font-size: 12px;
  }

  .list {
    flex: 1;
    overflow-y: auto;
    padding: 12px 20px;
  }

  .item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px;
    margin-bottom: 8px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 10px;
    transition: all 0.15s;
    animation: slideIn 0.3s ease;
  }

  @keyframes slideIn {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .item:hover {
    background: rgba(255, 255, 255, 0.06);
    border-color: rgba(255, 255, 255, 0.1);
  }

  .item-left {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
    min-width: 0;
  }

  .thumb {
    width: 48px;
    height: 48px;
    border-radius: 8px;
    overflow: hidden;
    flex-shrink: 0;
    background: rgba(255, 255, 255, 0.05);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .file-icon {
    font-size: 24px;
  }

  .item-info {
    flex: 1;
    min-width: 0;
  }

  .filename {
    font-size: 13px;
    color: #e2e8f0;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-bottom: 4px;
  }

  .meta {
    display: flex;
    gap: 8px;
    font-size: 11px;
    color: #64748b;
  }

  .item-right {
    display: flex;
    gap: 6px;
  }

  .btn-copy {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: #94a3b8;
    padding: 6px 10px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    transition: all 0.15s;
  }

  .btn-copy:hover {
    background: rgba(137, 180, 250, 0.15);
    border-color: rgba(137, 180, 250, 0.3);
    color: #89b4fa;
  }

  .footer {
    padding: 10px 20px;
    border-top: 1px solid rgba(255, 255, 255, 0.07);
    text-align: center;
  }

  .count {
    font-size: 12px;
    color: #64748b;
  }
</style>
