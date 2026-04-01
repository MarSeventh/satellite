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

  // Circumference of the progress ring
  const RADIUS = 28;
  const CIRCUMFERENCE = 2 * Math.PI * RADIUS;

  $: strokeDashoffset = CIRCUMFERENCE * (1 - uploadPercent / 100);
</script>

<div class="floating-stage">
  <div
    class="floating-root"
    class:drag-over={isDragOver}
    on:mousedown={onMouseDown}
    role="button"
    tabindex="0"
    on:keydown={(e) => e.key === "Enter" && handleClick()}
  >
    <!-- SVG progress ring -->
    <svg class="ring" width="72" height="72" viewBox="0 0 72 72" aria-hidden="true">
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
</div>

<style>
  .floating-stage {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    user-select: none;
    -webkit-tap-highlight-color: transparent;
  }

  .floating-root {
    position: relative;
    width: 72px;
    height: 72px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    background: transparent;
    outline: none;
  }

  .floating-root:focus,
  .floating-root:focus-visible {
    outline: none;
  }

  .ring {
    position: absolute;
    inset: 0;
    pointer-events: none;
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
    background:
      radial-gradient(circle at 30% 28%, rgba(255, 255, 255, 0.18), rgba(255, 255, 255, 0) 30%),
      linear-gradient(145deg, rgb(45, 47, 60) 0%, rgb(28, 30, 41) 58%, rgb(17, 19, 26) 100%);
    border: 1.5px solid rgba(255, 255, 255, 0.15);
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow:
      inset 0 1px 1px rgba(255, 255, 255, 0.12),
      inset 0 -10px 18px rgba(0, 0, 0, 0.35);
    transition: transform 0.2s ease, border-color 0.2s ease, background 0.2s ease, box-shadow 0.2s ease;
    animation: breathe 3s ease-in-out infinite;
    position: relative;
    z-index: 1;
    overflow: hidden;
    will-change: transform;
  }

  .ball::before,
  .ball::after {
    content: "";
    position: absolute;
    border-radius: 50%;
    pointer-events: none;
  }

  .ball::before {
    inset: 4px;
    background: radial-gradient(circle at 35% 30%, rgba(255, 255, 255, 0.18), rgba(255, 255, 255, 0) 36%);
    opacity: 0.9;
  }

  .ball::after {
    inset: 1px;
    border: 1px solid rgba(255, 255, 255, 0.06);
  }

  .ball:hover {
    transform: scale(1.08);
    background:
      radial-gradient(circle at 30% 28%, rgba(255, 255, 255, 0.22), rgba(255, 255, 255, 0) 30%),
      linear-gradient(145deg, rgb(49, 52, 66) 0%, rgb(31, 34, 46) 58%, rgb(20, 22, 31) 100%);
    animation: none;
  }

  .ball.drag-active {
    transform: scale(1.15);
    border-color: #89b4fa;
    background:
      radial-gradient(circle at 30% 28%, rgba(186, 214, 255, 0.24), rgba(255, 255, 255, 0) 30%),
      linear-gradient(145deg, rgb(43, 55, 80) 0%, rgb(26, 34, 52) 58%, rgb(15, 20, 33) 100%);
    box-shadow:
      inset 0 0 0 1px rgba(137, 180, 250, 0.26),
      inset 0 -10px 18px rgba(24, 38, 66, 0.45);
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
        inset 0 1px 1px rgba(255, 255, 255, 0.12),
        inset 0 -10px 18px rgba(0, 0, 0, 0.35);
    }
    50% {
      transform: scale(1.04);
      box-shadow:
        inset 0 1px 1px rgba(255, 255, 255, 0.14),
        inset 0 -12px 20px rgba(0, 0, 0, 0.38);
    }
  }
</style>
