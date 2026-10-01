// 生成 1024x1024 应用图标源图：蓝色圆角方块 + 白色取景框
import { PNG } from "pngjs";
import { writeFileSync } from "fs";

const SIZE = 1024;
const png = new PNG({ width: SIZE, height: SIZE });

const BG = [59, 130, 246]; // #3b82f6
const FG = [255, 255, 255];
const RADIUS = 200; // 圆角半径

// 取景框参数
const INSET = 240; // 框离边缘距离
const ARM = 200; // 角臂长
const THICK = 56; // 线条粗细

function inRoundedRect(x, y) {
  const r = RADIUS;
  const cx = x < r ? r : x > SIZE - r ? SIZE - r : x;
  const cy = y < r ? r : y > SIZE - r ? SIZE - r : y;
  const dx = x - cx;
  const dy = y - cy;
  return dx * dx + dy * dy <= r * r;
}

// 四个角臂（取景框四个角）
function inBracket(x, y) {
  const corners = [
    [INSET, INSET, 1, 1],
    [SIZE - INSET, INSET, -1, 1],
    [INSET, SIZE - INSET, 1, -1],
    [SIZE - INSET, SIZE - INSET, -1, -1],
  ];
  for (const [cx, cy, sx, sy] of corners) {
    // 横臂
    const hx0 = sx > 0 ? cx : cx - ARM;
    const hx1 = sx > 0 ? cx + ARM : cx;
    if (x >= hx0 && x <= hx1 && Math.abs(y - cy) <= THICK / 2) return true;
    // 竖臂
    const vy0 = sy > 0 ? cy : cy - ARM;
    const vy1 = sy > 0 ? cy + ARM : cy;
    if (y >= vy0 && y <= vy1 && Math.abs(x - cx) <= THICK / 2) return true;
  }
  return false;
}

for (let y = 0; y < SIZE; y++) {
  for (let x = 0; x < SIZE; x++) {
    const idx = (y * SIZE + x) << 2;
    if (!inRoundedRect(x, y)) {
      png.data[idx + 3] = 0; // 透明
      continue;
    }
    const c = inBracket(x, y) ? FG : BG;
    png.data[idx] = c[0];
    png.data[idx + 1] = c[1];
    png.data[idx + 2] = c[2];
    png.data[idx + 3] = 255;
  }
}

writeFileSync("icon-src.png", PNG.sync.write(png));
console.log("已生成 icon-src.png");
