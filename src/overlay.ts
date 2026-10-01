import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { initI18n, applyI18n, t } from "./i18n";

const dimEl = document.getElementById("dim")!;
const boxEl = document.getElementById("box")!;
const sizeEl = document.getElementById("size")!;
const tipEl = document.getElementById("tip")!;
const toolbarEl = document.getElementById("toolbar")!;
const paletteEl = document.getElementById("palette")!;
const bgEl = document.getElementById("bg") as HTMLImageElement;
const canvas = document.getElementById("canvas") as HTMLCanvasElement;

// mode: screenshot（截图）| gif（GIF 录制选区）| video（录屏选区）| color（取色）
const mode = new URLSearchParams(location.search).get("mode") ?? "screenshot";
const isColor = mode === "color";

// ============ 取色放大镜 ============

const magEl = document.getElementById("mag")!;
const magCanvas = document.getElementById("mag-canvas") as HTMLCanvasElement;
const magCtx = magCanvas.getContext("2d")!;
const magValue = document.getElementById("mag-value")!;
const MAG = 120;
const ZOOM = 10;
const CELLS = MAG / ZOOM;
let lastColorQuery = 0;

function updateMag(cx: number, cy: number) {
  // 位置：跟随鼠标，靠近右/下边缘时翻到另一侧
  let mx = cx + 24;
  let my = cy + 24;
  if (mx + MAG + 8 > window.innerWidth) mx = cx - MAG - 24;
  if (my + MAG + 44 > window.innerHeight) my = cy - MAG - 64;
  magEl.style.left = `${mx}px`;
  magEl.style.top = `${my}px`;

  if (!bgEl.src) return;
  const sx = cx * ratio;
  const sy = cy * ratio;
  magCtx.imageSmoothingEnabled = false;
  magCtx.clearRect(0, 0, MAG, MAG);
  magCtx.fillStyle = "#000";
  magCtx.fillRect(0, 0, MAG, MAG);
  magCtx.drawImage(bgEl, sx - CELLS / 2, sy - CELLS / 2, CELLS, CELLS, 0, 0, MAG, MAG);

  // 颜色值从 Rust 侧内存快照读真实像素（无 JPEG 色差），节流 40ms
  const now = Date.now();
  if (now - lastColorQuery > 40) {
    lastColorQuery = now;
    invoke<string>("overlay_color_at", { x: Math.round(sx), y: Math.round(sy) })
      .then((hex) => (magValue.textContent = hex))
      .catch(() => {});
  }
}

async function commitColor(e: MouseEvent) {
  tipEl.textContent = t("overlay.copying");
  try {
    await invoke("pick_color_commit", {
      x: Math.round(e.clientX * ratio),
      y: Math.round(e.clientY * ratio),
    });
  } catch (err) {
    tipEl.textContent = t("overlay.failed", { err: String(err) });
  }
}

function tipFor(m: string): string {
  return t(`overlay.tip.${m}`);
}
// 语言包加载完成后再设置提示语（见文件末尾启动逻辑）

// ============ 快照背景 ============

// 快照物理像素 / 显示尺寸 = 坐标换算比例
let ratio = window.devicePixelRatio || 1;

// 注意：不能用 asset:// 直链，跨域图片会污染 canvas 导致无法导出；
// 改为从 Rust 读原始字节，转 Blob URL（同源，画布可正常合成导出）
async function loadBackground() {
  try {
    const buf = (await invoke("read_snapshot")) as ArrayBuffer | Uint8Array;
    const bytes = buf instanceof Uint8Array ? buf : new Uint8Array(buf);
    const blob = new Blob([bytes as unknown as BlobPart], { type: "image/jpeg" });
    bgEl.src = URL.createObjectURL(blob);
    await bgEl.decode();
    ratio = bgEl.naturalWidth / bgEl.clientWidth;
  } catch (err) {
    console.error("加载快照背景失败", err);
    tipEl.textContent = t("overlay.initFailed");
  }
  sizeCanvas();
}

function sizeCanvas() {
  canvas.width = Math.round(window.innerWidth * ratio);
  canvas.height = Math.round(window.innerHeight * ratio);
  canvas.style.width = `${window.innerWidth}px`;
  canvas.style.height = `${window.innerHeight}px`;
}

// ============ 标注数据 ============

type Shape =
  | { t: "rect" | "ellipse"; x1: number; y1: number; x2: number; y2: number; color: string }
  | { t: "arrow"; x1: number; y1: number; x2: number; y2: number; color: string }
  | { t: "pen"; points: { x: number; y: number }[]; color: string }
  | { t: "mosaic"; x1: number; y1: number; x2: number; y2: number }
  | { t: "text"; x: number; y: number; text: string; color: string };

const shapes: Shape[] = [];
let activeShape: Shape | null = null;
let currentTool: string | null = null;
let currentColor = "#ef4444";

const STROKE = 3;
const TEXT_FONT = "18px 'Microsoft YaHei', 'PingFang SC', sans-serif";

function drawShape(ctx: CanvasRenderingContext2D, s: Shape) {
  if (s.t === "mosaic") {
    const x = Math.min(s.x1, s.x2);
    const y = Math.min(s.y1, s.y2);
    const w = Math.abs(s.x2 - s.x1);
    const h = Math.abs(s.y2 - s.y1);
    if (w < 3 || h < 3) return;
    const sw = w * ratio;
    const sh = h * ratio;
    const tw = Math.max(1, Math.round(sw / 14));
    const th = Math.max(1, Math.round(sh / 14));
    const tmp = document.createElement("canvas");
    tmp.width = tw;
    tmp.height = th;
    const tctx = tmp.getContext("2d")!;
    // 从快照图取样缩小，再放大回去形成马赛克
    tctx.drawImage(bgEl, x * ratio, y * ratio, sw, sh, 0, 0, tw, th);
    ctx.save();
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(tmp, 0, 0, tw, th, x, y, w, h);
    ctx.restore();
    return;
  }

  ctx.lineWidth = STROKE;
  ctx.strokeStyle = (s as { color: string }).color;
  ctx.fillStyle = (s as { color: string }).color;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";

  switch (s.t) {
    case "rect": {
      ctx.strokeRect(
        Math.min(s.x1, s.x2),
        Math.min(s.y1, s.y2),
        Math.abs(s.x2 - s.x1),
        Math.abs(s.y2 - s.y1)
      );
      break;
    }
    case "ellipse": {
      const cx = (s.x1 + s.x2) / 2;
      const cy = (s.y1 + s.y2) / 2;
      ctx.beginPath();
      ctx.ellipse(cx, cy, Math.abs(s.x2 - s.x1) / 2, Math.abs(s.y2 - s.y1) / 2, 0, 0, Math.PI * 2);
      ctx.stroke();
      break;
    }
    case "arrow": {
      const angle = Math.atan2(s.y2 - s.y1, s.x2 - s.x1);
      const headLen = 14;
      ctx.beginPath();
      ctx.moveTo(s.x1, s.y1);
      ctx.lineTo(s.x2, s.y2);
      ctx.moveTo(s.x2, s.y2);
      ctx.lineTo(s.x2 - headLen * Math.cos(angle - 0.45), s.y2 - headLen * Math.sin(angle - 0.45));
      ctx.moveTo(s.x2, s.y2);
      ctx.lineTo(s.x2 - headLen * Math.cos(angle + 0.45), s.y2 - headLen * Math.sin(angle + 0.45));
      ctx.stroke();
      break;
    }
    case "pen": {
      if (s.points.length < 2) return;
      ctx.beginPath();
      ctx.moveTo(s.points[0].x, s.points[0].y);
      for (const p of s.points) ctx.lineTo(p.x, p.y);
      ctx.stroke();
      break;
    }
    case "text": {
      ctx.font = TEXT_FONT;
      ctx.textBaseline = "top";
      ctx.fillText(s.text, s.x, s.y);
      break;
    }
  }
}

function redrawCanvas() {
  const ctx = canvas.getContext("2d")!;
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  if (shapes.length === 0 && !activeShape) return;
  // 统一用「CSS 坐标 × ratio」的变换绘制
  ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
  ctx.save();
  // 标注只显示在选区内（与微信一致）
  if (sel.w > 0) {
    ctx.beginPath();
    ctx.rect(sel.x, sel.y, sel.w, sel.h);
    ctx.clip();
  }
  for (const s of shapes) drawShape(ctx, s);
  if (activeShape) drawShape(ctx, activeShape);
  ctx.restore();
}

// ============ 选区状态机 ============

type State = "idle" | "dragging" | "selected" | "moving" | "resizing" | "drawing";

let state: State = "idle";
let sel = { x: 0, y: 0, w: 0, h: 0 };
let anchor = { x: 0, y: 0 };
let resizeDir = "";
let moveOffset = { x: 0, y: 0 };
let beforeResize = { x: 0, y: 0, w: 0, h: 0 };

function render() {
  boxEl.style.left = `${sel.x}px`;
  boxEl.style.top = `${sel.y}px`;
  boxEl.style.width = `${sel.w}px`;
  boxEl.style.height = `${sel.h}px`;
  sizeEl.textContent = `${Math.round(sel.w * ratio)} × ${Math.round(sel.h * ratio)}`;
  sizeEl.classList.toggle("below", sel.y < 36);
  redrawCanvas();
}

function positionToolbar() {
  const tw = toolbarEl.offsetWidth;
  let tx = sel.x + sel.w - tw;
  let ty = sel.y + sel.h + 8;
  if (ty + 48 > window.innerHeight) ty = sel.y - 48;
  if (ty < 0) ty = sel.y + 8;
  tx = Math.max(8, Math.min(tx, window.innerWidth - tw - 8));
  toolbarEl.style.left = `${tx}px`;
  toolbarEl.style.top = `${ty}px`;
  // 颜色条跟在工具条上方
  paletteEl.style.left = `${tx}px`;
  paletteEl.style.top = `${Math.max(8, ty - 40)}px`;
}

function enterSelected() {
  state = "selected";
  boxEl.classList.add("selected");
  document.body.style.cursor = "default";
  if (mode === "screenshot") {
    toolbarEl.classList.add("show");
    positionToolbar();
    tipEl.textContent = t("overlay.selected");
  }
}

function resetToIdle() {
  state = "idle";
  boxEl.classList.remove("selected");
  boxEl.style.display = "none";
  dimEl.style.display = "block";
  toolbarEl.classList.remove("show");
  paletteEl.classList.remove("show");
  document.body.style.cursor = "crosshair";
  tipEl.textContent = tipFor(mode);
}

function inSelection(px: number, py: number): boolean {
  return px >= sel.x && px <= sel.x + sel.w && py >= sel.y && py <= sel.y + sel.h;
}

// ============ 鼠标交互 ============

window.addEventListener("mousedown", (e) => {
  if (isColor) {
    if (e.button === 0) commitColor(e);
    return;
  }
  if (e.button !== 0) return;
  const target = e.target as HTMLElement;
  // 工具条 / 颜色条 / 文字输入框的事件各自处理
  if (target.closest("#toolbar") || target.closest("#palette") || target.closest(".text-input")) return;

  // 画图工具激活时：在选区内按下开始绘制
  if (state === "selected" && currentTool) {
    if (currentTool === "text") {
      // 必须拦截 mousedown 默认行为，否则默认的焦点转移会立刻 blur 掉新输入框
      e.preventDefault();
      startTextInput(e.clientX, e.clientY);
      return;
    }
    state = "drawing";
    anchor = { x: e.clientX, y: e.clientY };
    if (currentTool === "pen") {
      activeShape = { t: "pen", points: [{ x: e.clientX, y: e.clientY }], color: currentColor };
    } else if (currentTool === "mosaic") {
      activeShape = { t: "mosaic", x1: e.clientX, y1: e.clientY, x2: e.clientX, y2: e.clientY };
    } else {
      activeShape = {
        t: currentTool as "rect" | "ellipse" | "arrow",
        x1: e.clientX,
        y1: e.clientY,
        x2: e.clientX,
        y2: e.clientY,
        color: currentColor,
      };
    }
    return;
  }

  const dir = target.dataset.dir;
  if (state === "selected" && dir) {
    state = "resizing";
    resizeDir = dir;
    beforeResize = { ...sel };
    anchor = { x: e.clientX, y: e.clientY };
    return;
  }
  if (state === "selected" && inSelection(e.clientX, e.clientY)) {
    state = "moving";
    moveOffset = { x: e.clientX - sel.x, y: e.clientY - sel.y };
    return;
  }
  // 其他情况：开始新的框选（标注清空）
  state = "dragging";
  shapes.length = 0;
  activeShape = null;
  boxEl.classList.remove("selected");
  toolbarEl.classList.remove("show");
  paletteEl.classList.remove("show");
  document.body.style.cursor = "crosshair";
  anchor = { x: e.clientX, y: e.clientY };
  sel = { x: anchor.x, y: anchor.y, w: 0, h: 0 };
  dimEl.style.display = "none";
  boxEl.style.display = "block";
  render();
});

window.addEventListener("mousemove", (e) => {
  if (isColor) {
    updateMag(e.clientX, e.clientY);
    return;
  }
  if (state === "dragging") {
    sel = {
      x: Math.min(e.clientX, anchor.x),
      y: Math.min(e.clientY, anchor.y),
      w: Math.abs(e.clientX - anchor.x),
      h: Math.abs(e.clientY - anchor.y),
    };
    render();
  } else if (state === "moving") {
    sel.x = Math.max(0, Math.min(e.clientX - moveOffset.x, window.innerWidth - sel.w));
    sel.y = Math.max(0, Math.min(e.clientY - moveOffset.y, window.innerHeight - sel.h));
    render();
    if (mode === "screenshot") positionToolbar();
  } else if (state === "resizing") {
    const dx = e.clientX - anchor.x;
    const dy = e.clientY - anchor.y;
    let { x, y, w, h } = beforeResize;
    if (resizeDir.includes("e")) w = beforeResize.w + dx;
    if (resizeDir.includes("s")) h = beforeResize.h + dy;
    if (resizeDir.includes("w")) {
      x = beforeResize.x + dx;
      w = beforeResize.w - dx;
    }
    if (resizeDir.includes("n")) {
      y = beforeResize.y + dy;
      h = beforeResize.h - dy;
    }
    if (w < 0) {
      x += w;
      w = -w;
    }
    if (h < 0) {
      y += h;
      h = -h;
    }
    sel = { x: Math.max(0, x), y: Math.max(0, y), w: Math.max(1, w), h: Math.max(1, h) };
    render();
    if (mode === "screenshot") positionToolbar();
  } else if (state === "drawing" && activeShape) {
    if (activeShape.t === "pen") {
      activeShape.points.push({ x: e.clientX, y: e.clientY });
    } else {
      activeShape.x2 = e.clientX;
      activeShape.y2 = e.clientY;
    }
    redrawCanvas();
  }
});

window.addEventListener("mouseup", async (e) => {
  if (e.button !== 0) return;
  if (state === "dragging") {
    if (sel.w < 5 || sel.h < 5) {
      resetToIdle();
      return;
    }
    if (mode === "screenshot") {
      enterSelected();
    } else {
      await startRecording();
    }
  } else if (state === "moving" || state === "resizing") {
    enterSelected();
  } else if (state === "drawing") {
    if (activeShape) {
      // 太小的形状（误触）丢弃
      const tiny =
        activeShape.t !== "pen" &&
        Math.abs(activeShape.x2 - activeShape.x1) < 3 &&
        Math.abs(activeShape.y2 - activeShape.y1) < 3;
      if (!tiny) shapes.push(activeShape);
      activeShape = null;
      updateUndoButton();
    }
    state = "selected";
    redrawCanvas();
  }
});

// ============ 文字标注 ============

function startTextInput(x: number, y: number) {
  const input = document.createElement("input");
  input.className = "text-input";
  input.style.left = `${x}px`;
  input.style.top = `${y}px`;
  input.style.color = currentColor;
  document.body.appendChild(input);
  input.focus();

  let done = false;
  const commit = () => {
    if (done) return;
    done = true;
    const v = input.value.trim();
    if (v) {
      shapes.push({ t: "text", x, y, text: v, color: currentColor });
      updateUndoButton();
      redrawCanvas();
    }
    input.remove();
  };
  input.addEventListener("keydown", (e) => {
    e.stopPropagation(); // 防止 Esc 冒泡触发取消整个截图
    if (e.key === "Enter") commit();
    if (e.key === "Escape") {
      done = true;
      input.remove();
    }
  });
  input.addEventListener("blur", commit);
}

// ============ 工具条 ============

function updateUndoButton() {
  const btn = toolbarEl.querySelector('[data-op="undo"]') as HTMLButtonElement;
  btn.disabled = shapes.length === 0;
}

toolbarEl.addEventListener("click", async (e) => {
  const btn = (e.target as HTMLElement).closest("button");
  if (!btn) return;

  // 画图工具切换
  const tool = btn.dataset.tool;
  if (tool) {
    if (currentTool === tool) {
      currentTool = null;
      btn.classList.remove("active");
      paletteEl.classList.remove("show");
    } else {
      currentTool = tool;
      toolbarEl.querySelectorAll("[data-tool]").forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      if (tool !== "mosaic") {
        paletteEl.classList.add("show");
        positionToolbar();
      } else {
        paletteEl.classList.remove("show");
      }
    }
    return;
  }

  const op = btn.dataset.op;
  if (op) await doAction(op);
});

/** 执行收尾操作：undo / cancel / save / pin / done */
async function doAction(op: string) {
  if (op === "undo") {
    shapes.pop();
    updateUndoButton();
    redrawCanvas();
    return;
  }
  if (op === "cancel") {
    await invoke("cancel_capture");
    return;
  }

  // 保存 / 钉住 / 完成：前端合成最终图（快照区域 + 标注）后交给后端
  const labels: Record<string, string> = { done: t("overlay.copying"), save: t("overlay.saving"), pin: t("overlay.pinning") };
  tipEl.textContent = labels[op] ?? "…";
  try {
    // 原始像素直传（二进制通道 + 尺寸走请求头），框选层由后端立即关闭
    const { data, w, h } = compose();
    const headers: Record<string, string> = { "x-width": String(w), "x-height": String(h) };
    if (op === "done") {
      await invoke("copy_region", data, { headers });
    } else if (op === "save") {
      await invoke("capture_region", data, { headers });
    } else if (op === "pin") {
      await invoke("pin_region", data, {
        headers: {
          ...headers,
          "x-css-x": String(sel.x),
          "x-css-y": String(sel.y),
          "x-css-w": String(sel.w),
          "x-css-h": String(sel.h),
        },
      });
    }
  } catch (err) {
    console.error("操作失败", err);
    tipEl.textContent = t("overlay.failed", { err: String(err) });
  }
}

// 微信习惯：双击选区内部 = 完成（复制到剪贴板并关闭）
window.addEventListener("dblclick", async (e) => {
  const target = e.target as HTMLElement;
  if (target.closest("#toolbar") || target.closest("#palette") || target.closest(".text-input")) return;
  if (mode !== "screenshot") return;
  if (state === "selected" && inSelection(e.clientX, e.clientY)) {
    await doAction("done");
  }
});

// 颜色选择
paletteEl.addEventListener("click", (e) => {
  const colorEl = (e.target as HTMLElement).closest(".color") as HTMLElement | null;
  if (!colorEl) return;
  currentColor = colorEl.dataset.color!;
  paletteEl.querySelectorAll(".color").forEach((c) => c.classList.remove("active"));
  colorEl.classList.add("active");
});

// ============ 合成最终图片 ============

/** 合成最终图并直接返回 RGBA 原始像素（不经 PNG 编码，秒出） */
function compose(): { data: Uint8ClampedArray; w: number; h: number } {
  const px = Math.round(sel.x * ratio);
  const py = Math.round(sel.y * ratio);
  const pw = Math.round(sel.w * ratio);
  const ph = Math.round(sel.h * ratio);

  const c = document.createElement("canvas");
  c.width = pw;
  c.height = ph;
  const ctx = c.getContext("2d")!;

  // 背景：从快照图裁剪选区
  ctx.drawImage(bgEl, px, py, pw, ph, 0, 0, pw, ph);

  // 标注：变换到「选区为原点」的 CSS 坐标系
  ctx.save();
  ctx.beginPath();
  ctx.rect(0, 0, pw, ph);
  ctx.clip();
  ctx.setTransform(ratio, 0, 0, ratio, -sel.x * ratio, -sel.y * ratio);
  for (const s of shapes) drawShape(ctx, s);
  ctx.restore();

  const imageData = ctx.getImageData(0, 0, pw, ph);
  return { data: imageData.data, w: pw, h: ph };
}

// ============ 录制模式 ============

async function startRecording() {
  tipEl.textContent = t("overlay.recordStarting");
  try {
    await invoke("start_record", {
      kind: mode,
      x: sel.x,
      y: sel.y,
      width: sel.w,
      height: sel.h,
      scale: window.devicePixelRatio,
    });
  } catch (err) {
    console.error("启动录制失败", err);
    tipEl.textContent = t("overlay.recordFailed", { err: String(err) });
    resetToIdle();
  }
}

// ============ 键盘 / 取消 ============

async function cancel() {
  await invoke("cancel_capture");
}

// 另存为对话框被取消 → 恢复编辑状态
listen("save-cancelled", () => {
  if (state === "selected") tipEl.textContent = t("overlay.selected");
});

// Esc 兜底在 Rust 全局热键里也有一份，这里保留一份前端快速响应
window.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    // 有工具激活时先退出工具，再按一次才取消
    if (currentTool) {
      currentTool = null;
      toolbarEl.querySelectorAll("[data-tool]").forEach((b) => b.classList.remove("active"));
      paletteEl.classList.remove("show");
      return;
    }
    cancel();
  }
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z") {
    e.preventDefault();
    shapes.pop();
    updateUndoButton();
    redrawCanvas();
  }
});

window.addEventListener("contextmenu", (e) => {
  e.preventDefault();
  cancel();
});

// 启动：先加载语言包（含工具条 tooltip），再加载快照背景
(async () => {
  await initI18n();
  tipEl.textContent = tipFor(mode);
  if (isColor) {
    dimEl.style.display = "none";
    magEl.classList.add("show");
  }
  loadBackground();
})();
