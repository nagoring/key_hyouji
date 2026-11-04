import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

type DisplayItem = { id: number; kind: "text" | "combo"; label: string; count: number };
type Update = { type: "new" | "update"; item: DisplayItem };
type Layout = { vertical: "top" | "middle" | "bottom"; horizontal: "left" | "center" | "right"; arranging: boolean };
type Settings = { displayDurationMs: number; fadeDurationMs: number; maxItems: number };
type HookStatus = { state: "starting" | "running" | "needPermission" | "error"; message?: string };
type OverlayState = { layout: Layout; settings: Settings; paused: boolean; hook: HookStatus };

const stack = document.getElementById("stack")!;
const frame = document.getElementById("frame")!;
const notice = document.getElementById("notice")!;

let settings: Settings = { displayDurationMs: 2000, fadeDurationMs: 300, maxItems: 3 };
let hook: HookStatus = { state: "starting" };
let noticeTimer: number | undefined;

type Entry = { el: HTMLElement; label: HTMLElement; count: HTMLElement; timer?: number; leaving: boolean };
// 挿入順 = 古い順
const entries = new Map<number, Entry>();

function scheduleRemoval(id: number, entry: Entry) {
  clearTimeout(entry.timer);
  entry.timer = window.setTimeout(() => removeEntry(id), settings.displayDurationMs);
}

function removeEntry(id: number) {
  const entry = entries.get(id);
  if (!entry || entry.leaving) return;
  entry.leaving = true;
  clearTimeout(entry.timer);
  entries.delete(id);
  entry.el.classList.add("leaving");
  window.setTimeout(() => entry.el.remove(), settings.fadeDurationMs);
}

function show(update: Update) {
  const { item } = update;
  let entry = entries.get(item.id);
  if (!entry) {
    const el = document.createElement("div");
    el.className = `key ${item.kind}`;
    const label = document.createElement("span");
    label.className = "label";
    const count = document.createElement("span");
    count.className = "count";
    el.append(label, count);
    stack.append(el);
    entry = { el, label, count, leaving: false };
    entries.set(item.id, entry);
  } else if (item.kind === "combo") {
    entry.el.classList.remove("bump");
    void entry.el.offsetWidth; // アニメーションを再生し直す
    entry.el.classList.add("bump");
  }
  entry.label.textContent = item.label;
  entry.count.textContent = item.count > 1 ? `×${item.count}` : "";
  scheduleRemoval(item.id, entry);

  // 上限を超えたら古い順に消す（F-06）
  const overflow = entries.size - settings.maxItems;
  if (overflow > 0) {
    [...entries.keys()].slice(0, overflow).forEach(removeEntry);
  }
}

function applyLayout(layout: Layout) {
  stack.className = `stack v-${layout.vertical} h-${layout.horizontal}`;
  frame.hidden = !layout.arranging;
}

function applySettings(s: Settings) {
  settings = s;
  document.documentElement.style.setProperty("--fade-ms", `${s.fadeDurationMs}ms`);
}

function renderNotice(text: string | null, warn = false, durationMs?: number) {
  clearTimeout(noticeTimer);
  if (!text) {
    notice.hidden = true;
    return;
  }
  notice.textContent = text;
  notice.classList.toggle("warn", warn);
  notice.hidden = false;
  if (durationMs) {
    noticeTimer = window.setTimeout(() => renderHookNotice(), durationMs);
  }
}

/** フックの状態に応じた常時表示の通知（権限が無い場合など）。 */
function renderHookNotice() {
  switch (hook.state) {
    case "needPermission":
      renderNotice(
        "キー入力を表示するには「入力監視」の許可が必要です。\nシステム設定 › プライバシーとセキュリティ › 入力監視 で key_hyouji をオンにしてください。",
        true,
      );
      break;
    case "error":
      renderNotice(`キー入力の取得を開始できませんでした: ${hook.message ?? ""}`, true);
      break;
    default:
      renderNotice(null);
  }
}

function applyHook(status: HookStatus) {
  hook = status;
  renderHookNotice();
}

// 配置モード中はドラッグでウィンドウを移動
frame.addEventListener("mousedown", (e) => {
  if (e.button === 0) {
    void getCurrentWindow().startDragging();
  }
});

async function main() {
  await listen<Update>("keystroke", (e) => show(e.payload));
  await listen<Layout>("layout", (e) => applyLayout(e.payload));
  await listen<Settings>("settings", (e) => applySettings(e.payload));
  await listen<HookStatus>("hook-status", (e) => applyHook(e.payload));
  await listen<boolean>("paused", (e) => {
    if (e.payload) {
      entries.forEach((_, id) => removeEntry(id));
      renderNotice("⏸ 一時停止中", false, 1500);
    } else {
      renderNotice("▶ 表示を再開", false, 1500);
    }
  });

  const state = await invoke<OverlayState>("get_overlay_state");
  applySettings(state.settings);
  applyLayout(state.layout);
  applyHook(state.hook);
}

void main();
