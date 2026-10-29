<script>
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import FloatingBall from "./FloatingBall.svelte";
  import MainWindow from "./MainWindow.svelte";

  const appWindow = getCurrentWindow();
  const isFloating = appWindow.label === "floating";

  // Set the background before the child mounts and shows the native window.
  const background = isFloating ? "transparent" : "#1a1b23";
  document.documentElement.style.background = background;
  document.body.style.background = background;
  if (isFloating) document.documentElement.style.colorScheme = "normal";
</script>

{#if isFloating}
  <FloatingBall />
{:else}
  <MainWindow />
{/if}
