// Runtime evidence from the clean Windows CI machine; no local app data is read.
import { mkdir, writeFile } from "node:fs/promises";

const output = ".artifacts/floating";
await mkdir(output, { recursive: true });
try {
  const pages = await (await fetch("http://127.0.0.1:9223/json", { signal: AbortSignal.timeout(5000) })).json();
  for (const [index, page] of pages.entries()) {
    if (page.type !== "page") continue;
    const socket = new WebSocket(page.webSocketDebuggerUrl);
    const pending = new Map();
    const messages = [];
    let sequence = 0;
    socket.onmessage = ({ data }) => {
      const message = JSON.parse(data);
      if (message.id) pending.get(message.id)?.(message);
      else if (message.method === "Runtime.consoleAPICalled" || message.method === "Runtime.exceptionThrown") messages.push(message);
    };
    await new Promise((resolve, reject) => {
      socket.onopen = resolve;
      socket.onerror = reject;
    });
    const send = (method, params = {}) => new Promise((resolve, reject) => {
      const id = ++sequence;
      const timer = setTimeout(() => { pending.delete(id); reject(new Error(`Timed out: ${method}`)); }, 5000);
      pending.set(id, (result) => { clearTimeout(timer); pending.delete(id); resolve(result); });
      socket.send(JSON.stringify({ id, method, params }));
    });
    try {
      await send("Runtime.enable");
      const state = await send("Runtime.evaluate", {
        expression: `JSON.stringify({url: location.href, ready: document.readyState, window: window.__TAURI_INTERNALS__?.metadata?.currentWindow, orb: document.querySelector('.orb')?.getBoundingClientRect(), label: document.querySelector('.orb')?.getAttribute('aria-label'), bodyChildren: document.body.children.length})`,
        returnByValue: true,
      });
      console.log("WebView state:", JSON.stringify(state));
      console.log("WebView console:", JSON.stringify(messages));
      await writeFile(`${output}/webview-${index}.json`, JSON.stringify({ state, messages }, null, 2));
      const screenshot = await send("Page.captureScreenshot", { format: "png" });
      if (screenshot.result?.data) await writeFile(`${output}/webview-${index}.png`, Buffer.from(screenshot.result.data, "base64"));
    } finally {
      socket.close();
    }
  }
} catch (error) {
  console.log("WebView diagnostics unavailable:", String(error));
  await writeFile(`${output}/error.txt`, String(error));
}
