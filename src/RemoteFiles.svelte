<script>
  import { invoke } from "@tauri-apps/api/core";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { onMount } from "svelte";
  import { addToast } from "./toastStore.js";
  import { formatUrl, FORMAT_OPTIONS } from "./formatUrl.js";

  let files = [];
  let directories = [];
  let totalCount = 0;
  let returnedCount = 0;
  let currentDir = "";
  let searchQuery = "";
  let searchInput = "";
  let start = 0;
  let count = 50;
  let loading = false;
  let error = "";
  let defaultFormat = "raw";
  let baseUrl = "";

  onMount(async () => {
    try {
      const cfg = await invoke("get_config");
      defaultFormat = cfg.auto_copy_format || "raw";
      baseUrl = cfg.base_url || "";
    } catch (_) {}
    await loadFiles();
  });

  async function loadFiles() {
    loading = true;
    error = "";
    try {
      const result = await invoke("list_remote_files", {
        start,
        count,
        dir: currentDir,
        search: searchQuery,
      });
      files = result.files || [];
      directories = result.directories || [];
      totalCount = result.total_count || 0;
      returnedCount = result.returned_count || files.length;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function navigateToDir(dir) {
    currentDir = dir;
    start = 0;
    searchQuery = "";
    searchInput = "";
    loadFiles();
  }

  function handleSearch() {
    searchQuery = searchInput;
    start = 0;
    loadFiles();
  }

  function clearSearch() {
    searchInput = "";
    searchQuery = "";
    start = 0;
    loadFiles();
  }

  $: breadcrumbs = currentDir
    .split("/")
    .filter(Boolean)
    .reduce((acc, part) => {
      const path = acc.length > 0 ? acc[acc.length - 1].path + "/" + part : part;
      return [...acc, { name: part, path }];
    }, []);

  // Use returnedCount to determine if there's a next page when totalCount is 0
  $: hasNextPage = totalCount > 0 ? (start + count < totalCount) : (returnedCount >= count);
  $: hasPrevPage = start > 0;
  $: currentPage = Math.floor(start / count) + 1;
  $: displayTotal = totalCount > 0 ? totalCount : (start + returnedCount);

  function prevPage() {
    if (hasPrevPage) {
      start -= count;
      if (start < 0) start = 0;
      loadFiles();
    }
  }

  function nextPage() {
    if (hasNextPage) {
      start += count;
      loadFiles();
    }
  }

  function getFileUrl(file) {
    // file.name is the full path like "folder/image.jpg"
    const name = file.name || "";
    if (name.startsWith("http")) return name;
    return `${baseUrl}/file/${name}`;
  }

  function getDisplayName(file) {
    // Extract just the filename from path like "folder/image.jpg" -> "image.jpg"
    const name = file.name || "unknown";
    const parts = name.split("/");
    return parts[parts.length - 1] || name;
  }

  function getFilePath(file) {
    return file.name || "";
  }

  function isImage(filename) {
    const ext = filename.split(".").pop().toLowerCase();
    return ["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "avif"].includes(ext);
  }

  async function copyFile(file, format) {
    try {
      const url = getFileUrl(file);
      const name = getDisplayName(file);
      const text = formatUrl(url, name, format);
      await writeText(text);
      const label = FORMAT_OPTIONS.find((f) => f.value === format)?.label || format;
      addToast(`已复制 ${label} 链接`);
    } catch (e) {
      addToast("复制失败", "error");
    }
  }

  async function deleteFile(file) {
    const name = getDisplayName(file);
    if (!confirm(`确认删除远程文件 "${name}"？`)) return;
    try {
      const path = getFilePath(file);
      await invoke("delete_remote_file", { path });
      addToast("已删除远程文件");
      await loadFiles();
    } catch (e) {
      addToast(`删除失败: ${e}`, "error");
    }
  }
</script>

<div class="remote-wrap">
  <!-- Breadcrumb -->
  <div class="breadcrumb">
    <button class="crumb" on:click={() => navigateToDir("")}>🏠 /</button>
    {#each breadcrumbs as crumb}
      <span class="crumb-sep">›</span>
      <button class="crumb" on:click={() => navigateToDir(crumb.path)}>{crumb.name}</button>
    {/each}
  </div>

  <!-- Search -->
  <div class="search-bar">
    <input
      type="text"
      class="search-input"
      bind:value={searchInput}
      placeholder="搜索文件..."
      on:keydown={(e) => e.key === "Enter" && handleSearch()}
    />
    {#if searchQuery}
      <button class="search-btn" on:click={clearSearch}>✕</button>
    {:else}
      <button class="search-btn" on:click={handleSearch}>🔍</button>
    {/if}
  </div>

  {#if loading}
    <div class="center-state">
      <div class="spinner-large"></div>
      <p>加载中...</p>
    </div>
  {:else if error}
    <div class="center-state error-state">
      <p>❌ {error}</p>
      <button class="btn-retry" on:click={loadFiles}>重试</button>
    </div>
  {:else}
    <div class="file-list">
      <!-- Directories -->
      {#each directories as dir}
        <button class="file-item dir-item" on:click={() => navigateToDir(currentDir ? `${currentDir}/${dir}` : dir)}>
          <div class="file-left">
            <div class="file-thumb">📁</div>
            <div class="file-name">{dir}</div>
          </div>
        </button>
      {/each}

      <!-- Files -->
      {#each files as file}
        <div class="file-item">
          <div class="file-left">
            {#if isImage(getDisplayName(file))}
              <div class="file-thumb img-thumb">
                <img src={getFileUrl(file)} alt={getDisplayName(file)} loading="lazy" />
              </div>
            {:else}
              <div class="file-thumb">📄</div>
            {/if}
            <div class="file-name" title={getDisplayName(file)}>{getDisplayName(file)}</div>
          </div>
          <div class="file-right">
            <button class="btn-sm" on:click={() => copyFile(file, defaultFormat)} title="复制链接">📋</button>
            <button class="btn-sm btn-del" on:click={() => deleteFile(file)} title="删除">🗑️</button>
          </div>
        </div>
      {/each}

      {#if files.length === 0 && directories.length === 0}
        <div class="center-state">
          <p>此目录下没有文件</p>
        </div>
      {/if}
    </div>

    <div class="footer">
      <button class="page-btn" on:click={prevPage} disabled={!hasPrevPage}>‹ 上一页</button>
      <span class="page-info">第 {currentPage} 页{#if totalCount > 0}，共 {totalCount} 个文件{/if}</span>
      <button class="page-btn" on:click={nextPage} disabled={!hasNextPage}>下一页 ›</button>
    </div>
  {/if}
</div>

<style>
  .remote-wrap {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .breadcrumb {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 10px 20px;
    font-size: 13px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
    flex-wrap: wrap;
  }

  .crumb {
    background: none;
    border: none;
    color: #89b4fa;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 13px;
    transition: background 0.1s;
  }

  .crumb:hover {
    background: rgba(137, 180, 250, 0.15);
  }

  .crumb-sep {
    color: #475569;
  }

  .search-bar {
    display: flex;
    padding: 8px 20px;
    gap: 8px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
  }

  .search-input {
    flex: 1;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #e2e8f0;
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 13px;
    font-family: inherit;
  }

  .search-input:focus {
    outline: none;
    border-color: #89b4fa;
  }

  .search-input::placeholder {
    color: #475569;
  }

  .search-btn {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: #94a3b8;
    padding: 6px 10px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    transition: all 0.15s;
  }

  .search-btn:hover {
    background: rgba(137, 180, 250, 0.15);
    color: #89b4fa;
  }

  .center-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: #64748b;
    padding: 40px;
  }

  .error-state {
    color: #ef4444;
  }

  .btn-retry {
    margin-top: 12px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #94a3b8;
    padding: 6px 16px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
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

  .file-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px 20px;
  }

  .file-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    margin-bottom: 4px;
    border-radius: 8px;
    transition: background 0.1s;
  }

  .file-item:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .dir-item {
    cursor: pointer;
  }

  .dir-item:hover {
    background: rgba(137, 180, 250, 0.08);
  }

  .file-left {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: 1;
    min-width: 0;
  }

  .file-thumb {
    width: 36px;
    height: 36px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 20px;
    flex-shrink: 0;
  }

  .img-thumb {
    overflow: hidden;
    background: rgba(255, 255, 255, 0.05);
  }

  .img-thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .file-name {
    font-size: 13px;
    color: #e2e8f0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .file-right {
    display: flex;
    gap: 4px;
  }

  .btn-sm {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: #94a3b8;
    padding: 4px 8px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
    transition: all 0.15s;
  }

  .btn-sm:hover {
    background: rgba(137, 180, 250, 0.15);
    color: #89b4fa;
  }

  .btn-del:hover {
    background: rgba(239, 68, 68, 0.15);
    color: #ef4444;
  }

  .footer {
    padding: 10px 20px;
    border-top: 1px solid rgba(255, 255, 255, 0.07);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
  }

  .page-info {
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
