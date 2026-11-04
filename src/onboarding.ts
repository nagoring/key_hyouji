import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { HookStatus } from "./types";

type PermissionStatus = { granted: boolean; hook: HookStatus };

const status = document.getElementById("status")!;
const steps = document.getElementById("steps")!;
const openBtn = document.getElementById("openSettings") as HTMLButtonElement;
const restartBtn = document.getElementById("restart") as HTMLButtonElement;
const closeBtn = document.getElementById("close") as HTMLButtonElement;

function render(s: PermissionStatus) {
  const running = s.hook.state === "running";
  if (running) {
    status.className = "status ok";
    status.textContent = "✓ 準備完了：キー入力が画面に表示されます。";
  } else if (s.granted) {
    status.className = "status waiting";
    status.textContent = "許可されました。反映するには key_hyouji を再起動してください。";
  } else {
    status.className = "status waiting";
    status.textContent = "⏳ 入力監視の許可を待っています…";
  }
  steps.classList.toggle("done", s.granted || running);
  openBtn.hidden = s.granted || running;
  restartBtn.hidden = !s.granted || running;
  closeBtn.hidden = !running;
}

async function refresh() {
  render(await invoke<PermissionStatus>("get_permission_status"));
}

openBtn.addEventListener("click", () => void invoke("open_privacy_settings"));
restartBtn.addEventListener("click", () => void invoke("restart_app"));
closeBtn.addEventListener("click", () => void getCurrentWindow().close());

void refresh();
await listen<HookStatus>("hook-status", () => void refresh());
// システム設定から戻ってきたときなどに確認
window.addEventListener("focus", () => void refresh());
setInterval(() => void refresh(), 1500);
