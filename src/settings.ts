import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { applyI18n, initI18n, t, type Key } from "./i18n";
import {
  applyThemeVars,
  hotkeyLabel,
  THEME_PRESETS,
  type AppInfo,
  type Config,
  type MonitorInfo,
  type PresetId,
  type Theme,
} from "./types";

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;

let config: Config;
let info: AppInfo;
let saveTimer: number | undefined;
/** 保存中の件数。自分の保存による config-changed で編集中の値が戻らないようにする */
let inflight = 0;
let toastTimer: number | undefined;

// ---------------------------------------------------------------- 共通

function toast(message: string, ok = false) {
  const el = $("toast");
  el.textContent = message;
  el.classList.toggle("ok", ok);
  el.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => (el.hidden = true), ok ? 1800 : 5000);
}

/** 変更を少し待ってからまとめて保存する（スライダー操作中の連続保存を防ぐ） */
function save(immediate = false) {
  clearTimeout(saveTimer);
  const run = async () => {
    saveTimer = undefined;
    inflight++;
    try {
      const saved = await invoke<Config>("set_config", { config });
      // 範囲外の値などは Rust 側で補正されるので反映する
      if (JSON.stringify(saved) !== JSON.stringify(config)) {
        config = saved;
        render();
      }
    } catch (e) {
      toast(String(e));
      config = await invoke<Config>("get_config");
      render();
    } finally {
      inflight--;
    }
  };
  if (immediate) void run();
  else saveTimer = window.setTimeout(run, 150);
}

function change(mutate: (c: Config) => void, immediate = false) {
  mutate(config);
  renderPreview();
  save(immediate);
}

function setOutput(input: HTMLInputElement, text: string) {
  const out = input.parentElement?.querySelector("output");
  if (out) out.textContent = text;
}

type RangeSpec = {
  id: string;
  get: (c: Config) => number;
  set: (c: Config, v: number) => void;
  format: (v: number) => string;
  /** 外観の値を変えたらプリセットを「カスタム」にする */
  theme?: boolean;
};

const ranges: RangeSpec[] = [
  { id: "displayDuration", get: (c) => c.displayDurationMs / 1000, set: (c, v) => (c.displayDurationMs = Math.round(v * 1000)), format: (v) => `${v.toFixed(1)} ${t("unit.sec")}` },
  { id: "fadeDuration", get: (c) => c.fadeDurationMs, set: (c, v) => (c.fadeDurationMs = v), format: (v) => `${v} ms` },
  { id: "maxItems", get: (c) => c.maxItems, set: (c, v) => (c.maxItems = v), format: (v) => `${v} ${t("unit.items")}`.trim() },
  { id: "mergeTyping", get: (c) => c.mergeTypingMs / 1000, set: (c, v) => (c.mergeTypingMs = Math.round(v * 1000)), format: (v) => `${v.toFixed(1)} ${t("unit.sec")}` },
  { id: "fontSize", get: (c) => c.theme.fontSize, set: (c, v) => (c.theme.fontSize = v), format: (v) => `${v} px` },
  { id: "bgOpacity", get: (c) => c.theme.bgOpacity, set: (c, v) => (c.theme.bgOpacity = v), format: (v) => `${Math.round(v * 100)} %`, theme: true },
  { id: "borderRadius", get: (c) => c.theme.borderRadius, set: (c, v) => (c.theme.borderRadius = v), format: (v) => `${v} px`, theme: true },
  { id: "borderWidth", get: (c) => c.theme.borderWidth, set: (c, v) => (c.theme.borderWidth = v), format: (v) => `${v} px`, theme: true },
];

const colorFields: (keyof Theme & ("textColor" | "bgColor" | "borderColor"))[] = ["textColor", "bgColor", "borderColor"];

const PRESET_LABELS: Record<PresetId, string> = {
  "top-left": "↖", "top-center": "↑", "top-right": "↗",
  "middle-left": "←", "middle-center": "●", "middle-right": "→",
  "bottom-left": "↙", "bottom-center": "↓", "bottom-right": "↘",
};

// ---------------------------------------------------------------- 描画

function renderPreview() {
  const stage = $("previewStage");
  applyThemeVars(stage, config.theme);
  $("previewLabel").textContent = info.platform === "mac" ? "⌘ ⇧ S" : "Ctrl + Shift + S";
  document.querySelectorAll<HTMLButtonElement>("#themePresets button").forEach((b) => {
    b.classList.toggle("selected", b.dataset.preset === config.theme.preset);
  });
}

function render() {
  // 一般
  document.querySelectorAll<HTMLInputElement>('input[name="displayMode"]').forEach((r) => {
    r.checked = r.value === config.displayMode;
  });
  for (const r of ranges) {
    const el = $<HTMLInputElement>(r.id);
    const v = r.get(config);
    el.value = String(v);
    setOutput(el, r.format(v));
  }
  $<HTMLSelectElement>("modifierStyle").value = config.modifierStyle;
  $<HTMLInputElement>("showModifierOnly").checked = config.showModifierOnly;
  $<HTMLInputElement>("autoStart").checked = config.autoStart;
  $<HTMLSelectElement>("language").value = config.language;

  // 外観
  const font = $<HTMLInputElement>("fontFamily");
  if (document.activeElement !== font) font.value = config.theme.fontFamily;
  $<HTMLSelectElement>("fontWeight").value = String(config.theme.fontWeight);
  for (const f of colorFields) $<HTMLInputElement>(f).value = config.theme[f].slice(0, 7);
  $<HTMLInputElement>("shadow").checked = config.theme.shadow;
  $<HTMLSelectElement>("animation").value = config.theme.animation;
  renderPreview();

  // 位置
  const custom = config.position.type === "custom";
  const current = config.position.type === "preset" ? config.position.preset : null;
  document.querySelectorAll<HTMLButtonElement>("#presetGrid button").forEach((b) => {
    b.classList.toggle("selected", b.dataset.preset === current);
  });
  $("customNote").hidden = !custom;
  $<HTMLSelectElement>("monitor").value = monitorValue();
  $<HTMLSelectElement>("monitor").disabled = custom;
  $<HTMLInputElement>("offsetX").value = String(config.offsetX);
  $<HTMLInputElement>("offsetY").value = String(config.offsetY);

  // プライバシー
  $<HTMLInputElement>("maskPassword").checked = config.maskPasswordFields;
  $<HTMLInputElement>("secureIndicator").checked = config.showSecureIndicator;
  $<HTMLSelectElement>("imeMode").value = config.imeMode;
  renderExcluded();

  // ホットキー
  if (!recording) $("hotkeyLabel").textContent = hotkeyLabel(config.pauseHotkey, info.platform === "mac");
}

function monitorValue(): string {
  const m = config.monitor;
  return m.type === "fixed" ? `fixed:${m.name}` : m.type;
}

async function renderMonitors() {
  const monitors = await invoke<MonitorInfo[]>("list_monitors");
  const select = $<HTMLSelectElement>("monitor");
  const options: [string, string][] = [
    ["primary", t("position.monPrimary")],
    ["cursor", t("position.monCursor")],
    ["active", t("position.monActive")],
    ...monitors.map((m): [string, string] => [
      `fixed:${m.name}`,
      `${m.name} (${m.width}×${m.height}${m.primary ? t("position.monMain") : ""})`,
    ]),
  ];
  // 保存済みのモニターが今は接続されていない場合も選択肢に残す
  if (config.monitor.type === "fixed" && !monitors.some((m) => m.name === (config.monitor as { name: string }).name)) {
    options.push([monitorValue(), `${config.monitor.name}${t("position.monDisconnected")}`]);
  }
  select.replaceChildren(
    ...options.map(([value, label]) => {
      const o = document.createElement("option");
      o.value = value;
      o.textContent = label;
      return o;
    }),
  );
  select.value = monitorValue();
}

function renderExcluded() {
  const list = $("excludeList");
  if (config.excludedApps.length === 0) {
    const li = document.createElement("li");
    li.className = "empty";
    li.textContent = t("privacy.empty");
    list.replaceChildren(li);
    return;
  }
  list.replaceChildren(
    ...config.excludedApps.map((name, i) => {
      const li = document.createElement("li");
      const span = document.createElement("span");
      span.textContent = name;
      const btn = document.createElement("button");
      btn.textContent = t("privacy.remove");
      btn.addEventListener("click", () => change((c) => c.excludedApps.splice(i, 1), true));
      li.append(span, btn);
      return li;
    }),
  );
}

async function renderRunningApps() {
  const apps = await invoke<string[]>("list_running_apps");
  $("runningApps").replaceChildren(
    ...apps.map((a) => {
      const o = document.createElement("option");
      o.value = a;
      return o;
    }),
  );
}

// ---------------------------------------------------------------- イベント

function setupTabs() {
  const buttons = document.querySelectorAll<HTMLButtonElement>("#tabs button");
  buttons.forEach((b) =>
    b.addEventListener("click", () => {
      buttons.forEach((x) => x.classList.toggle("active", x === b));
      document.querySelectorAll(".tab").forEach((t) => t.classList.toggle("active", t.id === `tab-${b.dataset.tab}`));
      if (b.dataset.tab === "privacy") void renderRunningApps();
      if (b.dataset.tab === "position") void renderMonitors();
    }),
  );
}

function setupGeneral() {
  document.querySelectorAll<HTMLInputElement>('input[name="displayMode"]').forEach((r) =>
    r.addEventListener("change", () => change((c) => (c.displayMode = r.value as Config["displayMode"]), true)),
  );
  for (const r of ranges) {
    const el = $<HTMLInputElement>(r.id);
    el.addEventListener("input", () => {
      const v = Number(el.value);
      setOutput(el, r.format(v));
      change((c) => {
        r.set(c, v);
        if (r.theme) c.theme.preset = "custom";
      });
    });
  }
  $<HTMLSelectElement>("modifierStyle").addEventListener("change", (e) =>
    change((c) => (c.modifierStyle = (e.target as HTMLSelectElement).value as Config["modifierStyle"]), true),
  );
  $<HTMLInputElement>("showModifierOnly").addEventListener("change", (e) =>
    change((c) => (c.showModifierOnly = (e.target as HTMLInputElement).checked), true),
  );
  $<HTMLInputElement>("autoStart").addEventListener("change", (e) =>
    change((c) => (c.autoStart = (e.target as HTMLInputElement).checked), true),
  );
  $<HTMLSelectElement>("language").addEventListener("change", (e) =>
    change((c) => (c.language = (e.target as HTMLSelectElement).value as Config["language"]), true),
  );
  // 修飾キーの表記は macOS でのみ違いがある
  $("modifierStyleRow").hidden = info.platform !== "mac";
}

function setupAppearance() {
  const container = $("themePresets");
  for (const [id, preset] of Object.entries(THEME_PRESETS)) {
    const b = document.createElement("button");
    b.dataset.preset = id;
    const swatch = document.createElement("span");
    swatch.className = "swatch";
    const sample = document.createElement("span");
    sample.textContent = "A";
    const th = { ...config.theme, ...preset.theme };
    sample.style.color = th.textColor;
    sample.style.background = `color-mix(in srgb, ${th.bgColor} ${Math.round(th.bgOpacity * 100)}%, transparent)`;
    sample.style.border = th.borderWidth ? `${th.borderWidth}px solid ${th.borderColor}` : "none";
    sample.style.borderRadius = "4px";
    swatch.append(sample);
    const label = document.createElement("span");
    label.dataset.i18n = `preset.${id}`;
    label.textContent = t(`preset.${id}` as Key);
    b.append(swatch, label);
    b.addEventListener("click", () => {
      change((c) => (c.theme = { ...c.theme, ...preset.theme, preset: id }), true);
      render();
    });
    container.append(b);
  }

  const font = $<HTMLInputElement>("fontFamily");
  font.addEventListener("input", () => change((c) => (c.theme.fontFamily = font.value.trim() || "system-ui")));
  $<HTMLSelectElement>("fontWeight").addEventListener("change", (e) =>
    change((c) => {
      c.theme.fontWeight = Number((e.target as HTMLSelectElement).value);
      c.theme.preset = "custom";
    }, true),
  );
  for (const f of colorFields) {
    $<HTMLInputElement>(f).addEventListener("input", (e) =>
      change((c) => {
        c.theme[f] = (e.target as HTMLInputElement).value;
        c.theme.preset = "custom";
      }),
    );
  }
  $<HTMLInputElement>("shadow").addEventListener("change", (e) =>
    change((c) => {
      c.theme.shadow = (e.target as HTMLInputElement).checked;
      c.theme.preset = "custom";
    }, true),
  );
  $<HTMLSelectElement>("animation").addEventListener("change", (e) =>
    change((c) => (c.theme.animation = (e.target as HTMLSelectElement).value as Theme["animation"]), true),
  );
  $("screenPreview").addEventListener("click", () => void invoke("show_preview"));
}

let arranging = false;

function renderArranging() {
  const b = $("arrange");
  b.textContent = t(arranging ? "position.arrangeDone" : "position.arrangeStart");
  b.classList.toggle("on", arranging);
}

function setupPosition() {
  const grid = $("presetGrid");
  for (const [id, label] of Object.entries(PRESET_LABELS) as [PresetId, string][]) {
    const b = document.createElement("button");
    b.dataset.preset = id;
    b.textContent = label;
    b.addEventListener("click", () => change((c) => (c.position = { type: "preset", preset: id }), true));
    grid.append(b);
  }
  $("resetPosition").addEventListener("click", async () => {
    config = await invoke<Config>("reset_position");
    render();
  });
  $("arrange").addEventListener("click", async () => {
    await invoke("set_arranging", { on: !arranging });
  });
  $<HTMLSelectElement>("monitor").addEventListener("change", (e) => {
    const v = (e.target as HTMLSelectElement).value;
    change((c) => {
      c.monitor = v.startsWith("fixed:")
        ? { type: "fixed", name: v.slice("fixed:".length) }
        : { type: v as "primary" | "cursor" | "active" };
    }, true);
  });
  for (const id of ["offsetX", "offsetY"] as const) {
    $<HTMLInputElement>(id).addEventListener("change", (e) => {
      const v = Number((e.target as HTMLInputElement).value);
      if (Number.isFinite(v)) change((c) => (c[id] = v), true);
    });
  }
}

/** OS によって異なる説明文 */
function renderPlatformTexts() {
  const mac = info.platform === "mac";
  $("excludeHint").textContent = t(mac ? "privacy.excludeHintMac" : "privacy.excludeHintWin");
  $("passwordHint").textContent = t(mac ? "privacy.passwordHintMac" : "privacy.passwordHintWin");
  $("autoStartHint").textContent = t(mac ? "general.autoStartHintMac" : "general.autoStartHintWin");
  // macOS は OS が常に遮断するのでチェックボックスは不要
  $("maskPasswordWrap").hidden = mac;
}

function setupPrivacy() {
  $<HTMLInputElement>("maskPassword").addEventListener("change", (e) =>
    change((c) => (c.maskPasswordFields = (e.target as HTMLInputElement).checked), true),
  );
  $<HTMLInputElement>("secureIndicator").addEventListener("change", (e) =>
    change((c) => (c.showSecureIndicator = (e.target as HTMLInputElement).checked), true),
  );
  $<HTMLSelectElement>("imeMode").addEventListener("change", (e) =>
    change((c) => (c.imeMode = (e.target as HTMLSelectElement).value as Config["imeMode"]), true),
  );
  const input = $<HTMLInputElement>("excludeInput");
  const add = () => {
    const name = input.value.trim();
    if (!name) return;
    if (config.excludedApps.some((a) => a.toLowerCase() === name.toLowerCase())) {
      toast(t("privacy.duplicate"));
      return;
    }
    input.value = "";
    change((c) => c.excludedApps.push(name), true);
    renderExcluded();
  };
  $("excludeAdd").addEventListener("click", add);
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") add();
  });
  $("refreshApps").addEventListener("click", () => void renderRunningApps());
}

// ホットキーの記録
let recording = false;

const CODE_OK = /^(Key[A-Z]|Digit[0-9]|F([1-9]|1[0-9]|2[0-4])|Space|Enter|Tab|Backspace|Delete|Insert|Home|End|PageUp|PageDown|Arrow(Up|Down|Left|Right)|Minus|Equal|BracketLeft|BracketRight|Backslash|Semicolon|Quote|Comma|Period|Slash|Backquote|Numpad[0-9])$/;

function hotkeyFromEvent(e: KeyboardEvent): string | null {
  if (!CODE_OK.test(e.code)) return null;
  const mods: string[] = [];
  if (e.ctrlKey) mods.push("Control");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (e.metaKey) mods.push("Super");
  // 修飾キーなしは誤爆するので受け付けない（F キーは単体可）
  if (mods.length === 0 && !/^F\d+$/.test(e.code)) return null;
  return [...mods, e.code].join("+");
}

function stopRecording() {
  recording = false;
  $("hotkeyLabel").classList.remove("recording");
  $("hotkeyLabel").textContent = hotkeyLabel(config.pauseHotkey, info.platform === "mac");
  $("hotkeyRecord").textContent = t("hotkey.change");
  window.removeEventListener("keydown", onRecordKey, true);
}

function onRecordKey(e: KeyboardEvent) {
  e.preventDefault();
  e.stopPropagation();
  if (e.key === "Escape") {
    stopRecording();
    return;
  }
  const spec = hotkeyFromEvent(e);
  if (!spec) {
    $("hotkeyLabel").textContent = t("hotkey.needModifier");
    return;
  }
  stopRecording();
  change((c) => (c.pauseHotkey = spec), true);
  $("hotkeyLabel").textContent = hotkeyLabel(spec, info.platform === "mac");
}

function setupHotkey() {
  $("hotkeyRecord").addEventListener("click", () => {
    if (recording) {
      stopRecording();
      return;
    }
    recording = true;
    $("hotkeyLabel").classList.add("recording");
    $("hotkeyLabel").textContent = t("hotkey.recording");
    $("hotkeyRecord").textContent = t("hotkey.cancel");
    window.addEventListener("keydown", onRecordKey, true);
  });
  $("hotkeyReset").addEventListener("click", () => change((c) => (c.pauseHotkey = "Control+Alt+Shift+KeyK"), true));
}

function setupAbout() {
  $("version").textContent = info.version;
  $("configPath").textContent = info.configPath;
  $("resetAll").addEventListener("click", async () => {
    if (!confirm(t("about.resetConfirm"))) return;
    config = await invoke<Config>("reset_config");
    render();
    toast(t("about.resetDone"), true);
  });
}

/** 言語が変わったら動的に作った文言も作り直す */
function onLanguageChanged() {
  applyI18n();
  document.title = t("app.settingsTitle");
  renderPlatformTexts();
  renderArranging();
  if (!recording) $("hotkeyRecord").textContent = t("hotkey.change");
  void renderMonitors();
  render();
}

async function main() {
  await initI18n(onLanguageChanged);
  [config, info, arranging] = await Promise.all([
    invoke<Config>("get_config"),
    invoke<AppInfo>("get_app_info"),
    invoke<boolean>("is_arranging"),
  ]);
  setupTabs();
  setupGeneral();
  setupAppearance();
  setupPosition();
  setupPrivacy();
  setupHotkey();
  setupAbout();
  renderPlatformTexts();
  $("hotkeyRecord").textContent = t("hotkey.change");
  document.title = t("app.settingsTitle");
  await renderMonitors();
  render();
  renderArranging();

  // トレイなど他の場所での変更を反映
  await listen<Config>("config-changed", (e) => {
    if (saveTimer !== undefined || inflight > 0) return;
    if (JSON.stringify(e.payload) !== JSON.stringify(config)) {
      config = e.payload;
      render();
    }
  });
  await listen<boolean>("arranging", (e) => {
    arranging = e.payload;
    renderArranging();
  });
}

void main();
