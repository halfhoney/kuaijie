import { convertFileSrc } from "@tauri-apps/api/core";
import { appCacheDir, join } from "@tauri-apps/api/path";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { initI18n } from "./i18n";

const img = document.getElementById("pin-img") as HTMLImageElement;
const file = new URLSearchParams(location.search).get("img") ?? "";

(async () => {
  await initI18n();
  const dir = await appCacheDir();
  img.src = convertFileSrc(await join(dir, file)) + "?t=" + Date.now();
})();

const close = () => getCurrentWindow().close();

window.addEventListener("dblclick", close);
window.addEventListener("contextmenu", (e) => {
  e.preventDefault();
  close();
});
window.addEventListener("keydown", (e) => {
  if (e.key === "Escape") close();
});
