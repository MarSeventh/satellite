<script>
  import { onMount, onDestroy } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import FloatingBall from "./FloatingBall.svelte";
  import MainWindow from "./MainWindow.svelte";

  const appWindow = getCurrentWindow();
  const isFloating = appWindow.label === "floating";

  let uploadProgress = null;
  let unlisten = null;

  onMount(async () => {
    // Set background based on window type — both windows share the same CSS bundle,
    // so we must set this via JS to avoid the floating window's transparent background
    // overriding the main window's opaque background.
    if (isFloating) {
      document.documentElement.style.background = "transparent";
      document.body.style.background = "transparent";
    } else {
      document.documentElement.style.background = "#1a1b23";
      document.body.style.background = "#1a1b23";
    }

    unlisten = await listen("upload-progress", (event) => {
      uploadProgress = event.payload;
    });

    await listen("upload-complete", () => {
      setTimeout(() => {
        uploadProgress = null;
      }, 1500);
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });
</script>

{#if isFloating}
  <FloatingBall bind:uploadProgress />
{:else}
  <MainWindow />
{/if}
