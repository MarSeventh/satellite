<script>
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";

  const appWindow = getCurrentWindow();
  const unlisteners = [];
  let destroyed = false;
  let isDragOver = false;
  let isUploading = false;
  let uploadPercent = 0;
  let feedback = "";
  let feedbackTimer;

  $: label = isUploading ? `正在上传 ${Math.round(uploadPercent)}%`
    : isDragOver ? "松开文件即可上传"
    : feedback === "success" ? "上传完成"
    : feedback === "error" ? "上传失败，请检查设置后重试"
    : "Satellite · 点击打开，拖入文件上传";

  // Register even when initially hidden: Settings can show this window later.
  async function keepListener(pending) {
    const unlisten = await pending;
    if (destroyed) unlisten();
    else unlisteners.push(unlisten);
  }

  function resetFeedback() {
    clearTimeout(feedbackTimer);
    feedback = "";
  }

  function finishUpload(result) {
    isUploading = false;
    resetFeedback();
    feedback = result;
    feedbackTimer = setTimeout(() => { feedback = ""; }, 1800);
  }

  onMount(() => {
    async function initialize() {
      await keepListener(appWindow.onDragDropEvent(({ payload }) => {
        if (payload.type === "enter" || payload.type === "over") {
          isDragOver = !isUploading;
        } else {
          isDragOver = false;
          if (payload.type === "drop" && payload.paths.length && !isUploading) {
            uploadFiles(payload.paths);
          }
        }
      }));
      await keepListener(listen("upload-progress", ({ payload }) => {
        resetFeedback();
        isUploading = true;
        isDragOver = false;
        uploadPercent = Math.max(0, Math.min(100, payload.progress * 100));
      }));
      await keepListener(listen("upload-complete", () => {
        uploadPercent = 100;
        finishUpload("success");
      }));
      await keepListener(listen("upload-failed", () => finishUpload("error")));

      const cfg = await invoke("get_config");
      if (!destroyed && cfg.show_floating !== false) {
        // The native command applies the Windows region before showing the window.
        await invoke("set_floating_visible", { visible: true });
      }
    }
    initialize().catch((error) => console.error("Floating window setup failed:", error));
  });

  onDestroy(() => {
    destroyed = true;
    for (const unlisten of unlisteners) unlisten();
    clearTimeout(feedbackTimer);
    stopTracking();
  });

  async function uploadFiles(paths) {
    resetFeedback();
    isUploading = true;
    uploadPercent = 0;
    try {
      await invoke("upload_files", { filePaths: paths });
    } catch (error) {
      console.error("Upload failed:", error);
      finishUpload("error");
    }
  }

  function openMainWindow() {
    if (!isDragOver && !isUploading) {
      invoke("toggle_main_window").catch(console.error);
    }
  }

  let dragStartX = 0;
  let dragStartY = 0;
  let isDragging = false;

  function stopTracking() {
    window.removeEventListener("mousemove", onMouseMove);
    window.removeEventListener("mouseup", onMouseUp);
    window.removeEventListener("blur", stopTracking);
  }

  function onMouseDown(event) {
    if (event.button !== 0) return;
    event.preventDefault();
    dragStartX = event.screenX;
    dragStartY = event.screenY;
    isDragging = false;
    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
    window.addEventListener("blur", stopTracking);
  }

  async function onMouseMove(event) {
    if (isDragging) return;
    if (Math.abs(event.screenX - dragStartX) > 3 || Math.abs(event.screenY - dragStartY) > 3) {
      isDragging = true;
      stopTracking();
      try {
        await appWindow.startDragging();
      } catch (error) {
        console.error("Window drag failed:", error);
      }
    }
  }

  function onMouseUp(event) {
    stopTracking();
    if (event.button === 0 && !isDragging) openMainWindow();
    isDragging = false;
  }
</script>

<div class="stage">
  <button
    type="button"
    class="orb"
    class:uploading={isUploading}
    class:drag-active={isDragOver}
    class:success={feedback === "success"}
    class:error={feedback === "error"}
    aria-label={label}
    aria-busy={isUploading}
    title={label}
    on:mousedown={onMouseDown}
    on:click={(event) => { if (event.detail === 0) openMainWindow(); }}
    on:contextmenu={(event) => event.preventDefault()}
  >
    <span class="light" aria-hidden="true"></span>
    <svg class="progress-ring" class:visible={isUploading} viewBox="0 0 76 76" aria-hidden="true">
      <circle class="track" cx="38" cy="38" r="33" />
      <circle class="progress" cx="38" cy="38" r="33" pathLength="100"
        stroke-dasharray="100" stroke-dashoffset={100 - uploadPercent} />
    </svg>
    <span class="content" aria-hidden="true">
      {#if isDragOver}
        <svg class="glyph upload-glyph" viewBox="0 0 32 32" fill="none">
          <path d="M16 23V8m-6 6 6-6 6 6M8 22v4h16v-4" />
        </svg>
      {:else if isUploading}
        <span class="percent">{Math.round(uploadPercent)}<span class="unit">%</span></span>
      {:else if feedback === "success"}
        <svg class="glyph" viewBox="0 0 32 32" fill="none"><path d="m9 16 5 5 10-10" /></svg>
      {:else if feedback === "error"}
        <svg class="glyph" viewBox="0 0 32 32" fill="none"><path d="M16 9v9m0 5v.1" /></svg>
      {:else}
        <svg class="moon" viewBox="0 0 40 40" fill="none">
          <path d="M26.8 25.7A12 12 0 0 1 14.3 9.2a12 12 0 1 0 12.5 16.5Z" fill="currentColor" />
          <circle class="satellite" cx="28.5" cy="11.5" r="2.2" fill="currentColor" />
        </svg>
      {/if}
    </span>
  </button>
</div>

<style>
  .stage {
    position: fixed;
    inset: 0;
    display: grid;
    place-items: center;
    overflow: hidden;
    background: transparent;
    user-select: none;
    -webkit-tap-highlight-color: transparent;
    pointer-events: none;
  }

  .orb {
    --ink: #506786;
    position: relative;
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    padding: 0;
    border: 0;
    border-radius: 50%;
    overflow: hidden;
    isolation: isolate;
    color: var(--ink);
    cursor: grab;
    pointer-events: auto;
    background:
      radial-gradient(ellipse at 30% 18%, #ffffff 0%, #f3f7fc 22%, transparent 63%),
      radial-gradient(ellipse at 82% 82%, #abbfdb 0%, transparent 64%),
      linear-gradient(145deg, #eaf1fa 15%, #d8e4f2 55%, #c6d4e9);
    box-shadow:
      inset 0 0 0 1px #ffffff80,
      inset 2px 3px 7px #ffffffb3,
      inset -3px -5px 9px #6e88b02e;
    transition: color 220ms ease, box-shadow 220ms ease;
  }

  .orb:active { cursor: grabbing; }
  .orb:focus-visible { outline: 2px solid #5b7eae; outline-offset: -5px; }

  .light {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: radial-gradient(ellipse at 68% 75%, #b7c5ff70, transparent 62%);
    animation: breathe 5s ease-in-out infinite;
    pointer-events: none;
  }

  .content { display: grid; place-items: center; z-index: 1; }
  .moon { width: 36px; height: 36px; transition: transform 300ms cubic-bezier(.2,.8,.2,1); }
  .satellite { opacity: .6; }
  .orb:hover .moon { transform: translateY(-1px) rotate(-10deg); }
  .orb:hover { --ink: #3d608b; box-shadow: inset 0 0 0 1px #ffffffd9, inset 2px 3px 8px #fff, inset -3px -5px 10px #6e88b03b; }

  .orb.drag-active {
    --ink: #357e83;
    box-shadow: inset 0 0 0 2px #81b7b9, inset 0 0 20px #b0e5de99;
  }
  .drag-active .light { background: radial-gradient(circle at 50% 60%, #c1ede9, transparent 75%); animation: none; }
  .uploading .light, .success .light, .error .light { animation: none; }
  .success { --ink: #347a6c; }
  .error { --ink: #a66550; }

  .glyph { width: 32px; height: 32px; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; }
  .upload-glyph { animation: invite 1.2s ease-in-out infinite; }
  .percent { font-size: 18px; font-weight: 500; font-variant-numeric: tabular-nums; letter-spacing: -.6px; }
  .unit { font-size: 10px; margin-left: 1px; opacity: .75; }

  .progress-ring { position: absolute; inset: 0; width: 100%; height: 100%; fill: none; opacity: 0; transition: opacity 180ms ease; transform: rotate(-90deg); }
  .progress-ring.visible { opacity: 1; }
  .track { stroke: #6888b329; stroke-width: 1.5; }
  .progress { stroke: #628ab7; stroke-width: 2; stroke-linecap: round; transition: stroke-dashoffset 300ms ease; }

  @keyframes breathe { 0%, 100% { opacity: .35; } 50% { opacity: .85; } }
  @keyframes invite { 0%, 100% { transform: translateY(1px); } 50% { transform: translateY(-2px); } }
  @media (prefers-reduced-motion: reduce) {
    .light, .upload-glyph { animation: none; }
    .orb, .moon, .progress, .progress-ring { transition: none; }
    .orb:hover .moon { transform: none; }
  }
</style>
