<script>
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";

  let isDragOver = false;
  let isUploading = false;
  let uploadPercent = 0;
  let unlistenDrop = null;
  let unlistenDragOver = null;
  let unlistenDragLeave = null;

  const appWindow = getCurrentWindow();

  onMount(async () => {
    // Check config to see if floating ball should be shown
    try {
      const cfg = await invoke("get_config");
      if (cfg.show_floating === false) return;
    } catch (_) {}

    // Show the floating window after Svelte has mounted and set transparent background
    appWindow.show();

    // Listen for file drag-drop events from Tauri
    unlistenDrop = await listen("tauri://drag-drop", async (event) => {
      isDragOver = false;
      if (event.payload && event.payload.paths && event.payload.paths.length > 0) {
        await uploadFiles(event.payload.paths);
      }
    });

    unlistenDragOver = await listen("tauri://drag-over", () => {
      isDragOver = true;
    });

    unlistenDragLeave = await listen("tauri://drag-leave", () => {
      isDragOver = false;
    });

    await listen("upload-progress", (event) => {
      isUploading = true;
      uploadPercent = event.payload.progress * 100;
    });

    await listen("upload-complete", () => {
      isUploading = false;
      uploadPercent = 100;
      setTimeout(() => { uploadPercent = 0; }, 1500);
    });
  });

  onDestroy(() => {
    if (unlistenDrop) unlistenDrop();
    if (unlistenDragOver) unlistenDragOver();
    if (unlistenDragLeave) unlistenDragLeave();
  });

  async function uploadFiles(paths) {
    isUploading = true;
    uploadPercent = 0;
    try {
      await invoke("upload_files", { filePaths: paths });
    } catch (e) {
      console.error("Upload failed:", e);
      isUploading = false;
    }
  }

  async function handleClick() {
    // Single click: toggle main window
    await invoke("toggle_main_window");
  }

  // --- Drag vs Click detection ---
  let dragStartX = 0;
  let dragStartY = 0;
  let isDragging = false;

  function onMouseDown(e) {
    if (e.button !== 0) return;
    e.preventDefault();
    dragStartX = e.clientX;
    dragStartY = e.clientY;
    isDragging = false;
    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
  }

  async function onMouseMove(e) {
    if (isDragging) return;
    const dx = e.clientX - dragStartX;
    const dy = e.clientY - dragStartY;
    if (Math.abs(dx) > 3 || Math.abs(dy) > 3) {
      isDragging = true;
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
      await appWindow.startDragging();
    }
  }

  function onMouseUp() {
    window.removeEventListener("mousemove", onMouseMove);
    window.removeEventListener("mouseup", onMouseUp);
    // Don't trigger click if we were dragging, or if a file drag-over/upload is active
    if (!isDragging && !isDragOver && !isUploading) {
      handleClick();
    }
    isDragging = false;
  }
</script>

<div class="stage">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="card"
    class:uploading={isUploading}
    class:drag-active={isDragOver}
    on:mousedown={onMouseDown}
  >
    {#if isUploading}
      <div class="progress-track">
        <div class="progress-fill" style="width: {uploadPercent}%"></div>
      </div>
    {/if}

    {#if isDragOver}
      <span class="icon">⬆</span>
    {:else if isUploading}
      <span class="percent">{Math.round(uploadPercent)}%</span>
    {:else}
      <span class="icon">🌙</span>
    {/if}
  </div>
</div>

<style>
  .stage {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    user-select: none;
    -webkit-tap-highlight-color: transparent;
    pointer-events: none;
  }

  .card {
    width: 84px;
    height: 84px;
    border-radius: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    overflow: hidden;
    cursor: pointer;
    pointer-events: auto;
    background:
      radial-gradient(circle at 35% 28%, rgba(255, 255, 255, 0.22), rgba(255, 255, 255, 0) 55%),
      linear-gradient(145deg, rgb(45, 47, 60) 0%, rgb(28, 30, 41) 58%, rgb(17, 19, 26) 100%);
    box-shadow:
      0 8px 24px rgba(0, 0, 0, 0.35),
      inset 0 1px 1px rgba(255, 255, 255, 0.1);
    transition: transform 0.2s ease, box-shadow 0.2s ease;
    animation: breathe 3s ease-in-out infinite;
    will-change: transform;
  }

  .card::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: 26px;
    pointer-events: none;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.08);
  }

  .card:hover {
    transform: scale(1.05);
    animation: none;
  }

  .card.drag-active {
    transform: scale(1.08);
    box-shadow:
      0 10px 28px rgba(137, 180, 250, 0.3),
      inset 0 0 0 2px rgba(137, 180, 250, 0.5);
    animation: none;
  }

  .card.uploading {
    animation: none;
  }

  .progress-track {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: rgba(255, 255, 255, 0.08);
  }

  .progress-fill {
    height: 100%;
    background: #89b4fa;
    transition: width 0.3s ease;
  }

  .icon {
    font-size: 30px;
    line-height: 1;
  }

  .percent {
    font-size: 14px;
    font-weight: 600;
    color: #89b4fa;
    font-family: monospace;
  }

  @keyframes breathe {
    0%, 100% {
      box-shadow:
        0 8px 24px rgba(0, 0, 0, 0.35),
        inset 0 1px 1px rgba(255, 255, 255, 0.1);
    }
    50% {
      box-shadow:
        0 10px 28px rgba(0, 0, 0, 0.4),
        inset 0 1px 1px rgba(255, 255, 255, 0.12);
    }
  }
</style>
