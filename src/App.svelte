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
      // On macOS we use native NSWindow transparency for rounded corners.
      // On Windows the floating ball is square and fills the entire window,
      // so we just match the card's dark background — no transparency needed.
      const isMac = navigator.userAgent.includes("Mac");
      const bg = isMac ? "transparent" : "#1c1e29";
      document.documentElement.style.background = bg;
      document.documentElement.style.colorScheme = "normal";
      document.body.style.background = bg;
      const appDiv = document.getElementById("app");
      if (appDiv) appDiv.style.background = bg;
      document.documentElement.style.setProperty("background-color", bg, "important");
      document.body.style.setProperty("background-color", bg, "important");
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
