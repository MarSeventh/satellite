<script>
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import FloatingBall from "./FloatingBall.svelte";
  import MainWindow from "./MainWindow.svelte";

  const appWindow = getCurrentWindow();
  const isFloating = appWindow.label === "floating";

  onMount(async () => {
    // Set background based on window type — both windows share the same CSS bundle,
    // so we must set this via JS to avoid the floating window's transparent background
    // overriding the main window's opaque background.
    if (isFloating) {
      document.documentElement.style.background = "transparent";
      document.documentElement.style.colorScheme = "normal";
      document.body.style.background = "transparent";
      const appDiv = document.getElementById("app");
      if (appDiv) appDiv.style.background = "transparent";
      // Override any Tailwind/reset backgrounds
      document.documentElement.style.setProperty("background-color", "transparent", "important");
      document.body.style.setProperty("background-color", "transparent", "important");
    } else {
      document.documentElement.style.background = "#1a1b23";
      document.body.style.background = "#1a1b23";
    }
  });
</script>

{#if isFloating}
  <FloatingBall />
{:else}
  <MainWindow />
{/if}
