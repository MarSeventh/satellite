<script>
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open } from "@tauri-apps/plugin-dialog";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";

  export let uploadProgress = null;

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
    if (!isDragging) {
      handleClick();
    }
    isDragging = false;
  }

  // Circumference of the progress ring
  const RADIUS = 28;
  const CIRCUMFERENCE = 2 * Math.PI * RADIUS;

  $: strokeDashoffset = CIRCUMFERENCE * (1 - uploadPercent / 100);
</script>

<div
  class="floating-root"
  class:drag-over={isDragOver}
  on:mousedown={onMouseDown}
  role="button"
  tabindex="0"
  on:keydown={(e) => e.key === "Enter" && handleClick()}
>
  <!-- SVG progress ring -->
  <svg class="ring" width="72" height="72" viewBox="0 0 72 72">
    <!-- Background circle -->
    <circle
      cx="36" cy="36" r={RADIUS}
      class="ring-bg"
      fill="none"
      stroke-width="3"
    />
    <!-- Progress arc -->
    {#if isUploading}
      <circle
        cx="36" cy="36" r={RADIUS}
        class="ring-progress"
        fill="none"
        stroke-width="3"
        stroke-dasharray={CIRCUMFERENCE}
        stroke-dashoffset={strokeDashoffset}
        transform="rotate(-90 36 36)"
      />
    {/if}
  </svg>

  <!-- Main ball -->
  <div class="ball" class:uploading={isUploading} class:drag-active={isDragOver}>
    {#if isDragOver}
      <span class="icon">⬆️</span>
    {:else if isUploading}
      <span class="percent">{Math.round(uploadPercent)}%</span>
    {:else}
      <span class="icon">🛰️</span>
    {/if}
  </div>
</div>

<style>
  .floating-root {
    position: fixed;
    inset: 0;
    width: 72px;
    height: 72px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .ring {
    position: absolute;
    top: 0;
    left: 0;
  }

  .ring-bg {
    stroke: rgba(255, 255, 255, 0.1);
  }

  .ring-progress {
    stroke: #89b4fa;
    stroke-linecap: round;
    transition: stroke-dashoffset 0.3s ease;
  }

  .ball {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: rgba(30, 30, 40, 0.85);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border: 1.5px solid rgba(255, 255, 255, 0.15);
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow:
      0 4px 24px rgba(0, 0, 0, 0.4),
      0 1px 4px rgba(0, 0, 0, 0.3);
    transition: transform 0.2s ease, box-shadow 0.2s ease;
    animation: breathe 3s ease-in-out infinite;
    position: relative;
    z-index: 1;
  }

  .ball:hover {
    transform: scale(1.08);
    box-shadow:
      0 8px 32px rgba(0, 0, 0, 0.5),
      0 2px 8px rgba(137, 180, 250, 0.2);
    animation: none;
  }

  .ball.drag-active {
    transform: scale(1.15);
    border-color: #89b4fa;
    box-shadow:
      0 0 0 2px rgba(137, 180, 250, 0.4),
      0 8px 32px rgba(137, 180, 250, 0.3);
    animation: none;
  }

  .ball.uploading {
    animation: none;
  }

  .icon {
    font-size: 22px;
    line-height: 1;
  }

  .percent {
    font-size: 11px;
    font-weight: 600;
    color: #89b4fa;
    font-family: monospace;
  }

  @keyframes breathe {
    0%, 100% {
      transform: scale(1);
      box-shadow:
        0 4px 24px rgba(0, 0, 0, 0.4),
        0 0 0 0 rgba(137, 180, 250, 0);
    }
    50% {
      transform: scale(1.04);
      box-shadow:
        0 6px 28px rgba(0, 0, 0, 0.45),
        0 0 0 4px rgba(137, 180, 250, 0.1);
    }
  }
</style>
