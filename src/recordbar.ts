import { invoke } from "@tauri-apps/api/core";
import { initI18n, t } from "./i18n";

const timerEl = document.getElementById("timer")!;
const statusEl = document.getElementById("status")!;
const dotEl = document.getElementById("dot")!;
const pauseBtn = document.getElementById("pause")!;
const stopBtn = document.getElementById("stop")!;

const startedAt = Date.now();
let paused = false;
let pauseStart = 0;
let pausedTotal = 0;

function pad(n: number): string {
  return String(n).padStart(2, "0");
}

setInterval(() => {
  if (paused) return; // 暂停时计时冻结
  const secs = Math.floor((Date.now() - startedAt - pausedTotal) / 1000);
  timerEl.textContent = `${pad(Math.floor(secs / 60))}:${pad(secs % 60)}`;
}, 250);

pauseBtn.addEventListener("click", async () => {
  const nowPaused = await invoke<boolean>("toggle_pause");
  paused = nowPaused;
  if (nowPaused) {
    pauseStart = Date.now();
    pauseBtn.textContent = "▶ " + t("rec.resume");
    statusEl.textContent = t("rec.paused");
    statusEl.classList.add("paused");
    dotEl.classList.add("paused");
  } else {
    pausedTotal += Date.now() - pauseStart;
    pauseBtn.textContent = "⏸ " + t("rec.pause");
    statusEl.textContent = t("rec.recording");
    statusEl.classList.remove("paused");
    dotEl.classList.remove("paused");
  }
});

stopBtn.addEventListener("click", async () => {
  timerEl.textContent = t("rec.encoding");
  stopBtn.setAttribute("disabled", "true");
  pauseBtn.setAttribute("disabled", "true");
  await invoke("stop_record");
});

// 启动：加载语言包后刷新一次按钮文案
(async () => {
  await initI18n();
  statusEl.textContent = t("rec.recording");
  pauseBtn.textContent = "⏸ " + t("rec.pause");
  stopBtn.textContent = "■ " + t("rec.stop");
})();
