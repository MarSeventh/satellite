<script>
  import { invoke } from "@tauri-apps/api/core";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { onMount } from "svelte";
  import { addToast } from "./toastStore.js";
  import { formatUrl, FORMAT_OPTIONS } from "./formatUrl.js";

  let items = [];
  let page = 1;
  let pageSize = 50;
  let total = 0;
  let loading = true;
  let defaultFormat = "raw";
  let openMenuId = null;

  onMount(async () => {
    try {
      const cfg = await invoke("get_config");
      defaultFormat = cfg.auto_copy_format || "raw";
    } catch (_) {}
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

  $: totalPages = Math.max(1, Math.ceil(total / pageSize));

  async function copyFormatted(url, filename, format) {
    try {
      const text = formatUrl(url, filename, format);
      await writeText(text);
      const label = FORMAT_OPTIONS.find((f) => f.value === format)?.label || format;
      addToast(`已复制 ${label} 链接`);
    } catch (e) {
      addToast("复制失败", "error");
    }
    openMenuId = null;
  }

  async function deleteItem(id) {
    if (!confirm("确认删除此记录？")) return;
    try {
      await invoke("delete_history", { id });
      items = items.filter((i) => i.id !== id);
      total--;
      if (items.length === 0 && page > 1) {
        page--;
        await loadHistory();
      }
      addToast("已删除");
    } catch (e) {
      addToast(`删除失败: ${e}`, "error");
    }
  }

  function prevPage() {
    if (page > 1) { page--; loadHistory(); }
  }

  function nextPage() {
    if (page < totalPages) { page++; loadHistory(); }
  }

  function toggleMenu(id) {
    openMenuId = openMenuId === id ? null : id;
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
            <div class="copy-group">
              <button class="btn-copy" on:click={() => copyFormatted(item.url, item.filename, defaultFormat)} title="复制链接">
                📋
              </button>
              <button class="btn-copy btn-dropdown" on:click={() => toggleMenu(item.id)} title="选择格式">
                ▾
              </button>
              {#if openMenuId === item.id}
                <div class="format-menu">
                  {#each FORMAT_OPTIONS as opt}
                    <button class="format-item" on:click={() => copyFormatted(item.url, item.filename, opt.value)}>
                      {opt.label}
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
            <button class="btn-copy btn-delete" on:click={() => deleteItem(item.id)} title="删除">
              🗑️
            </button>
          </div>
        </div>
      {/each}
    </div>
    <div class="footer">
      <button class="page-btn" on:click={prevPage} disabled={page <= 1}>‹ 上一页</button>
      <span class="count">{page}/{totalPages} 页，共 {total} 条</span>
      <button class="page-btn" on:click={nextPage} disabled={page >= totalPages}>下一页 ›</button>
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
    align-items: center;
  }

  .copy-group {
    position: relative;
    display: flex;
    gap: 2px;
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

  .btn-dropdown {
    padding: 6px 6px;
    font-size: 10px;
    border-radius: 0 6px 6px 0;
  }

  .copy-group .btn-copy:first-child {
    border-radius: 6px 0 0 6px;
  }

  .btn-delete:hover {
    background: rgba(239, 68, 68, 0.15);
    border-color: rgba(239, 68, 68, 0.3);
    color: #ef4444;
  }

  .format-menu {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: 4px;
    background: #2a2b35;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 4px;
    z-index: 100;
    min-width: 120px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  }

  .format-item {
    display: block;
    width: 100%;
    background: none;
    border: none;
    color: #e2e8f0;
    padding: 6px 12px;
    font-size: 12px;
    text-align: left;
    border-radius: 4px;
    cursor: pointer;
    transition: background 0.1s;
  }

  .format-item:hover {
    background: rgba(137, 180, 250, 0.15);
    color: #89b4fa;
  }

  .footer {
    padding: 10px 20px;
    border-top: 1px solid rgba(255, 255, 255, 0.07);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
  }

  .count {
    font-size: 12px;
    color: #64748b;
  }

  .page-btn {
    background: none;
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #94a3b8;
    padding: 4px 10px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 12px;
    transition: all 0.15s;
  }

  .page-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.06);
    color: #e2e8f0;
  }

  .page-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }
</style>
