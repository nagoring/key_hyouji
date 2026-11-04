import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { initI18n, t } from "./i18n";
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
    status.textContent = t("onb.ready");
  } else if (s.granted) {
    status.className = "status waiting";
    status.textContent = t("onb.granted");
  } else {
    status.className = "status waiting";
    status.textContent = t("onb.waiting");
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

await initI18n(() => void refresh());
void refresh();
await listen<HookStatus>("hook-status", () => void refresh());
// システム設定から戻ってきたときなどに確認
window.addEventListener("focus", () => void refresh());
setInterval(() => void refresh(), 1500);
