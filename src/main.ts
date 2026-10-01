import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { initI18n, applyI18n, t, setLang } from "./i18n";

interface HistoryItem {
  id: number;
  kind: string;
  path: string;
  width: number;
  height: number;
  created_at: string;
  size: number;
}

interface Settings {
  save_dir: string | null;
  autostart: boolean;
  language: string;
  gif_fps: number;
  gif_max_width: number;
  video_fps: number;
  video_bitrate_k: number;
  hotkey_screenshot: string;
  hotkey_video: string;
  hotkey_gif: string;
  hotkey_color: string;
}

const historyEl = document.getElementById("history")!;
const emptyEl = document.getElementById("empty")!;
const toastEl = document.getElementById("toast")!;
const savePathEl = document.getElementById("save-path")!;

let toastTimer: number | undefined;

function toast(msg: string) {
  toastEl.textContent = msg;
  toastEl.classList.add("show");
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => toastEl.classList.remove("show"), 2200);
}

function fileName(p: string): string {
  return p.split(/[\\/]/).pop() ?? p;
}

function dirName(p: string): string {
  return p.replace(/[\\/][^\\/]*$/, "");
}

function formatTime(iso: string): string {
  const d = new Date(iso);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function formatSize(bytes: number): string {
  if (bytes <= 0) return "";
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

function render(items: HistoryItem[]) {
  emptyEl.classList.toggle("show", items.length === 0);
  historyEl.innerHTML = "";
  for (const item of items) {
    const card = document.createElement("div");
    card.className = "card";
    const media =
      item.kind === "video"
        ? `<video class="thumb" src="${convertFileSrc(item.path)}" preload="metadata" muted data-act="open"></video>`
        : `<img class="thumb" src="${convertFileSrc(item.path)}" alt="" data-act="open" />`;
    const size = formatSize(item.size);
    card.innerHTML = `
      <div class="thumb-wrap">
        ${media}
        <span class="badge badge-${item.kind}">${t("kind." + item.kind)}</span>
      </div>
      <div class="card-body">
        <div class="card-name" title="${item.path}">${fileName(item.path)}</div>
        <div class="card-meta">${item.width} × ${item.height}${size ? " · " + size : ""}</div>
        <div class="card-meta">${formatTime(item.created_at)}</div>
      </div>
      <div class="card-ops">
        <button data-act="open">${t("card.open")}</button>
        <button data-act="folder">${t("card.folder")}</button>
        <button data-act="delete" class="danger">${t("card.delete")}</button>
      </div>
    `;
    // 视频缩略图取 0.5s 处的帧，避免黑乎乎的第一帧
    const video = card.querySelector("video");
    if (video) {
      video.addEventListener("loadedmetadata", () => {
        video.currentTime = Math.min(0.5, (video.duration || 1) / 2);
      });
    }
    card.addEventListener("click", async (e) => {
      const act = (e.target as HTMLElement).dataset.act;
      if (!act) return;
      try {
        if (act === "open") {
          await invoke("open_path", { path: item.path });
        } else if (act === "folder") {
          await invoke("open_path", { path: dirName(item.path) });
        } else if (act === "delete") {
          await invoke("delete_history", { id: item.id });
          toast(t("toast.deleted"));
          await refresh();
        }
      } catch (err) {
        toast(t("toast.failed", { err: String(err) }));
      }
    });
    historyEl.appendChild(card);
  }
}

let allItems: HistoryItem[] = [];
let searchKeyword = "";

function applyFilter() {
  const kw = searchKeyword.trim().toLowerCase();
  const filtered = kw
    ? allItems.filter(
        (i) =>
          fileName(i.path).toLowerCase().includes(kw) ||
          t("kind." + i.kind).toLowerCase().includes(kw) ||
          i.kind.toLowerCase().includes(kw)
      )
    : allItems;
  render(filtered);
}

async function refresh() {
  try {
    allItems = await invoke<HistoryItem[]>("list_history");
    applyFilter();
  } catch (err) {
    console.error("加载历史记录失败", err);
  }
}

// 搜索框
document.getElementById("search-input")!.addEventListener("input", (e) => {
  searchKeyword = (e.target as HTMLInputElement).value;
  applyFilter();
});

// 打开保存目录
document.getElementById("btn-open-folder")!.addEventListener("click", async () => {
  try {
    const dir = await invoke<string>("get_save_dir");
    await invoke("open_path", { path: dir });
  } catch (err) {
    toast(t("toast.failed", { err: String(err) }));
  }
});

// 列表/网格视图切换
const listBtn = document.getElementById("view-list-btn")!;
const gridBtn = document.getElementById("view-grid-btn")!;
function setViewMode(list: boolean) {
  historyEl.classList.toggle("list", list);
  historyEl.classList.toggle("grid", !list);
  listBtn.classList.toggle("active", list);
  gridBtn.classList.toggle("active", !list);
}
listBtn.addEventListener("click", () => setViewMode(true));
gridBtn.addEventListener("click", () => setViewMode(false));

async function refreshSavePath() {
  try {
    const p = await invoke<string>("get_save_dir");
    savePathEl.textContent = t("history.saveTo") + p;
  } catch { /* 忽略 */ }
}

document.getElementById("btn-region")!.addEventListener("click", async () => {
  try {
    await invoke("start_region_capture");
  } catch (err) {
    toast(t("toast.failed", { err: String(err) }));
  }
});

document.getElementById("btn-full")!.addEventListener("click", async () => {
  try {
    await invoke<HistoryItem>("capture_fullscreen");
    toast(t("toast.savedCopied"));
    await refresh();
  } catch (err) {
    toast(t("toast.failed", { err: String(err) }));
  }
});

document.getElementById("btn-gif")!.addEventListener("click", async () => {
  try {
    await invoke("start_gif_capture");
  } catch (err) {
    toast(`${err}`);
  }
});

document.getElementById("btn-video")!.addEventListener("click", async () => {
  try {
    await invoke("start_video_capture");
  } catch (err) {
    toast(`${err}`);
  }
});

document.getElementById("btn-color")!.addEventListener("click", async () => {
  try {
    await invoke("start_color_capture");
  } catch (err) {
    toast(`${err}`);
  }
});

// 取色完成提示
listen<string>("color-copied", (e) => {
  toast(t("toast.colorCopied", { hex: e.payload }));
});

// 截图/录制完成后，由后端广播事件刷新列表
listen<string>("history-updated", async (e) => {
  const msg =
    e.payload === "gif"
      ? t("toast.gifSaved")
      : e.payload === "video"
        ? t("toast.videoSaved")
        : t("toast.savedCopied");
  toast(msg);
  await refresh();
});

// 录制失败提示
listen<string>("record-error", (e) => {
  toast(t("toast.recordFailed", { err: e.payload }));
});

// 窗口重新获得焦点时也刷新一次（兜底）
window.addEventListener("focus", () => {
  refresh();
  refreshSavePath();
});

// 空状态提示里的热键跟随设置
function renderEmptyHint(hotkey: string) {
  const el = document.getElementById("empty-hint");
  if (el) el.textContent = t("history.emptyHint", { key: hotkey });
}

// ============ 设置页 ============

let settings: Settings | null = null;

const tabHome = document.getElementById("tab-home")!;
const tabHistory = document.getElementById("tab-history")!;
const tabSettings = document.getElementById("tab-settings")!;
const tabAbout = document.getElementById("tab-about")!;
const viewHome = document.getElementById("view-home")!;
const viewHistory = document.getElementById("view-history")!;
const viewSettings = document.getElementById("view-settings")!;
const viewAbout = document.getElementById("view-about")!;

type TabName = "home" | "history" | "settings" | "about";

function switchTab(name: TabName) {
  tabHome.classList.toggle("active", name === "home");
  tabHistory.classList.toggle("active", name === "history");
  tabSettings.classList.toggle("active", name === "settings");
  tabAbout.classList.toggle("active", name === "about");
  viewHome.hidden = name !== "home";
  viewHistory.hidden = name !== "history";
  viewSettings.hidden = name !== "settings";
  viewAbout.hidden = name !== "about";
}
tabHome.addEventListener("click", () => switchTab("home"));
tabHistory.addEventListener("click", () => switchTab("history"));
tabSettings.addEventListener("click", () => switchTab("settings"));
tabAbout.addEventListener("click", () => switchTab("about"));

// 关于页：版本号 + 许可证入口
import { getVersion } from "@tauri-apps/api/app";
getVersion().then((v) => {
  document.getElementById("about-version")!.textContent = v;
});
document.getElementById("btn-licenses")!.addEventListener("click", async () => {
  try {
    await invoke("open_licenses");
  } catch (err) {
    toast(t("toast.failed", { err: String(err) }));
  }
});

/** 下拉选项文案按语言重新生成（选项标签含参数，不适合 data-i18n） */
function localizeSelects() {
  document.querySelectorAll("option[data-opt]").forEach((opt) => {
    const o = opt as HTMLOptionElement;
    o.textContent = `${o.value}${o.closest("#set-gif-width") ? " px" : " fps"} · ${t(o.dataset.opt!)}`;
  });
  document.querySelectorAll("option[data-opt-bitrate]").forEach((opt) => {
    const o = opt as HTMLOptionElement;
    o.textContent = `${t(o.dataset.optBitrate!)} · ${o.dataset.mbps} Mbps`;
  });
}

function fillSettingsForm(s: Settings) {
  document.getElementById("set-save-dir")!.textContent = s.save_dir ?? t("settings.defaultDir");
  (document.getElementById("set-autostart") as HTMLInputElement).checked = s.autostart;
  (document.getElementById("set-language") as HTMLSelectElement).value = s.language;
  (document.getElementById("hk-screenshot") as HTMLInputElement).value = s.hotkey_screenshot;
  (document.getElementById("hk-video") as HTMLInputElement).value = s.hotkey_video;
  (document.getElementById("hk-gif") as HTMLInputElement).value = s.hotkey_gif;
  (document.getElementById("hk-color") as HTMLInputElement).value = s.hotkey_color;
  (document.getElementById("set-gif-fps") as HTMLSelectElement).value = String(s.gif_fps);
  (document.getElementById("set-gif-width") as HTMLSelectElement).value = String(s.gif_max_width);
  (document.getElementById("set-video-fps") as HTMLSelectElement).value = String(s.video_fps);
  (document.getElementById("set-video-bitrate") as HTMLSelectElement).value = String(s.video_bitrate_k);
  renderEmptyHint(s.hotkey_screenshot);
  // 首页卡片上的热键提示同步
  document.getElementById("hk-chip-region")!.textContent = s.hotkey_screenshot;
  document.getElementById("hk-chip-video")!.textContent = s.hotkey_video;
  document.getElementById("hk-chip-gif")!.textContent = s.hotkey_gif;
  document.getElementById("hk-chip-color")!.textContent = s.hotkey_color;
}

async function loadSettings() {
  try {
    const old = settings;
    settings = await invoke<Settings>("get_settings");
    // 语言变了：重新渲染界面文案
    if (!old || old.language !== settings.language) {
      setLang(settings.language);
      applyI18n();
      localizeSelects();
      refresh();
      refreshSavePath();
    }
    fillSettingsForm(settings);
  } catch (err) {
    toast(t("toast.loadSettingsFailed", { err: String(err) }));
  }
}

async function persistSettings(next: Settings, successMsg = t("toast.settingsSaved")) {
  try {
    await invoke("save_settings", { settings: next });
    settings = next;
    toast(successMsg);
    refreshSavePath();
  } catch (err) {
    toast(t("toast.saveFailed", { err: String(err) }));
    if (settings) fillSettingsForm(settings); // 回滚界面
  }
}

function bindSelect(id: string, apply: (v: string, s: Settings) => Settings) {
  document.getElementById(id)!.addEventListener("change", async (e) => {
    if (!settings) return;
    const v = (e.target as HTMLSelectElement).value;
    await persistSettings(apply(v, settings));
  });
}

bindSelect("set-gif-fps", (v, s) => ({ ...s, gif_fps: Number(v) }));
bindSelect("set-gif-width", (v, s) => ({ ...s, gif_max_width: Number(v) }));
bindSelect("set-video-fps", (v, s) => ({ ...s, video_fps: Number(v) }));
bindSelect("set-video-bitrate", (v, s) => ({ ...s, video_bitrate_k: Number(v) }));
bindSelect("set-language", (v, s) => ({ ...s, language: v }));

document.getElementById("btn-browse")!.addEventListener("click", async () => {
  if (!settings) return;
  const dir = await invoke<string | null>("pick_save_dir");
  if (dir) {
    await persistSettings({ ...settings, save_dir: dir }, t("toast.dirUpdated"));
  }
});

document.getElementById("btn-reset-dir")!.addEventListener("click", async () => {
  if (!settings) return;
  await persistSettings({ ...settings, save_dir: null }, t("toast.dirReset"));
});

// 开机自启开关
document.getElementById("set-autostart")!.addEventListener("change", async (e) => {
  if (!settings) return;
  const on = (e.target as HTMLInputElement).checked;
  await persistSettings(
    { ...settings, autostart: on },
    on ? t("toast.autostartOn") : t("toast.autostartOff")
  );
});

// 热键捕获：点击输入框后按下组合键
function bindHotkey(id: string, field: "hotkey_screenshot" | "hotkey_video" | "hotkey_gif" | "hotkey_color") {
  const el = document.getElementById(id) as HTMLInputElement;
  el.addEventListener("keydown", async (e) => {
    e.preventDefault();
    if (!settings) return;
    if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;
    const mods: string[] = [];
    if (e.ctrlKey) mods.push("Ctrl");
    if (e.shiftKey) mods.push("Shift");
    if (e.altKey) mods.push("Alt");
    if (e.metaKey) mods.push("Win");
    if (mods.length === 0) {
      toast(t("toast.needModifier"));
      return;
    }
    let key = e.key.toUpperCase();
    if (key === " ") key = "SPACE";
    if (key.length > 1 && !/^F\d{1,2}$|^(SPACE|TAB|ENTER)$/.test(key)) return;
    const combo = [...mods, key].join("+");
    await persistSettings({ ...settings, [field]: combo }, t("toast.hotkeyUpdated"));
    el.blur();
  });
}
bindHotkey("hk-screenshot", "hotkey_screenshot");
bindHotkey("hk-video", "hotkey_video");
bindHotkey("hk-gif", "hotkey_gif");
bindHotkey("hk-color", "hotkey_color");

// ============ 自定义标题栏（无边框窗口） ============

const appWin = getCurrentWindow();

document.getElementById("tb-min")!.addEventListener("click", () => appWin.minimize());
document.getElementById("tb-max")!.addEventListener("click", () => appWin.toggleMaximize());
document.getElementById("tb-close")!.addEventListener("click", () => appWin.close());

// 双击标题栏切换最大化
document.querySelector(".titlebar")!.addEventListener("dblclick", (e) => {
  if ((e.target as HTMLElement).hasAttribute("data-tauri-drag-region")) {
    appWin.toggleMaximize();
  }
});

// 托盘勾选「开机自启」或切换语言后同步刷新
listen("settings-changed", () => {
  loadSettings();
});

// ============ 启动 ============

(async () => {
  await initI18n();
  localizeSelects();
  await loadSettings();
  refresh();
  refreshSavePath();
})();
