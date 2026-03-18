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
