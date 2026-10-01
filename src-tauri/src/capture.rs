use crate::db::HistoryItem;
use crate::AppState;
use arboard::Clipboard;
use chrono::Local;
use image::RgbaImage;
use screenshots::Screen;
use std::borrow::Cow;
use std::path::PathBuf;
use tauri::{Emitter, Manager};

/// 框选层快照的元数据（快照图本身写到缓存目录 overlay_bg.jpg，由前端展示和合成）
pub struct OverlaySnapshot {
    pub scale: f64,
    pub logical_x: f64,
    pub logical_y: f64,
    /// 内存中的原始像素（取色器用，避免 JPEG 压缩色差）
    pub img: RgbaImage,
}

/// 主屏信息（供框选层窗口定位铺满）
pub struct PrimaryScreenInfo {
    /// 逻辑坐标/尺寸（Tauri 窗口用）
    pub logical_x: f64,
    pub logical_y: f64,
    pub logical_w: f64,
    pub logical_h: f64,
}

/// 截图保存目录：优先用户自定义目录，否则 图片/快截，取不到则退回应用数据目录
pub fn save_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let custom = app
        .state::<AppState>()
        .settings
        .lock()
        .ok()
        .and_then(|s| s.save_dir.clone())
        .filter(|p| !p.trim().is_empty());
    let dir = match custom {
        Some(p) => PathBuf::from(p),
        None => {
            let base = dirs::picture_dir()
                .or_else(|| app.path().app_data_dir().ok())
                .ok_or_else(|| "无法确定保存目录".to_string())?;
            base.join("快截")
        }
    };
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn primary_screen() -> Result<Screen, String> {
    let screens = Screen::all().map_err(|e| e.to_string())?;
    screens
        .into_iter()
        .find(|s| s.display_info.is_primary)
        .or_else(|| Screen::from_point(0, 0).ok())
        .ok_or_else(|| "未找到显示器".to_string())
}

/// 截取主屏并编码为 JPEG 写入缓存目录（框选层背景），同时记录屏幕元数据。
/// 注意：本函数耗时（截屏 + 编码），必须在后台线程调用，不能阻塞 UI 主线程。
pub fn make_snapshot(app: &tauri::AppHandle) -> Result<(PathBuf, PrimaryScreenInfo), String> {
    let screen = primary_screen()?;
    let info = screen.display_info;
    let img: RgbaImage = screen.capture().map_err(|e| e.to_string())?;

    let temp = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("overlay_bg.jpg");
    if let Some(parent) = temp.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // JPEG 编码比 PNG 快一个数量级，框选层背景不需要无损
    let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
    let mut file = std::io::BufWriter::new(std::fs::File::create(&temp).map_err(|e| e.to_string())?);
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, 85);
    encoder
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ColorType::Rgb8.into(),
        )
        .map_err(|e| e.to_string())?;

    let sf = info.scale_factor.max(0.1) as f64;
    let rect = PrimaryScreenInfo {
        logical_x: info.x as f64 / sf,
        logical_y: info.y as f64 / sf,
        logical_w: info.width as f64 / sf,
        logical_h: info.height as f64 / sf,
    };

    let state = app.state::<AppState>();
    *state.overlay_snapshot.lock().map_err(|e| e.to_string())? = Some(OverlaySnapshot {
        scale: sf,
        logical_x: rect.logical_x,
        logical_y: rect.logical_y,
        img,
    });

    Ok((temp, rect))
}

/// 读取快照中某个物理像素的颜色（#RRGGBB），取色器用
pub fn color_at(app: &tauri::AppHandle, x: u32, y: u32) -> Result<String, String> {
    let state = app.state::<AppState>();
    let snap = state.overlay_snapshot.lock().map_err(|e| e.to_string())?;
    let snap = snap.as_ref().ok_or_else(|| "快照数据不存在".to_string())?;
    let px = x.min(snap.img.width().saturating_sub(1));
    let py = y.min(snap.img.height().saturating_sub(1));
    let p = snap.img.get_pixel(px, py);
    Ok(format!("#{:02X}{:02X}{:02X}", p[0], p[1], p[2]))
}

/// 复制文本到剪贴板
pub fn copy_text(text: &str) -> Result<(), String> {
    let mut cb = Clipboard::new().map_err(|e| format!("无法访问剪贴板：{e}"))?;
    cb.set_text(text).map_err(|e| format!("复制失败：{e}"))
}

/// 快速 PNG 编码写盘（Fast 压缩 + Sub 滤镜，大图编码比默认快数倍）
pub fn write_png_fast(img: &RgbaImage, path: &std::path::Path) -> Result<(), String> {
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    use image::ImageEncoder;
    let file = std::io::BufWriter::new(std::fs::File::create(path).map_err(|e| e.to_string())?);
    let encoder = PngEncoder::new_with_quality(file, CompressionType::Fast, FilterType::Sub);
    encoder
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ColorType::Rgba8,
        )
        .map_err(|e| e.to_string())
}

/// 当前快照元数据（钉住窗口定位用）
pub fn snapshot_meta(app: &tauri::AppHandle) -> Option<(f64, f64, f64)> {
    app.state::<AppState>()
        .overlay_snapshot
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(|m| (m.scale, m.logical_x, m.logical_y)))
}

/// 框选层传来的逻辑坐标 + DPI 缩放 → 物理像素矩形。
/// 返回 (定位屏幕用的锚点 x, 锚点 y, 屏幕内相对 x, 相对 y, 宽, 高)。
pub fn physical_rect(
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    scale: f64,
) -> Result<(i32, i32, u32, u32, u32, u32), String> {
    let px = (x * scale) as i32;
    let py = (y * scale) as i32;
    let screen = Screen::from_point(px, py).map_err(|e| e.to_string())?;
    let info = screen.display_info;
    let rx = (px - info.x).max(0) as u32;
    let ry = (py - info.y).max(0) as u32;
    let pw = ((w * scale) as u32).min(info.width.saturating_sub(rx));
    let ph = ((h * scale) as u32).min(info.height.saturating_sub(ry));
    if pw == 0 || ph == 0 {
        return Err("选区超出屏幕范围".to_string());
    }
    Ok((px, py, rx, ry, pw, ph))
}

/// 解码前端合成好的 PNG 字节
pub fn decode_image(bytes: &[u8]) -> Result<RgbaImage, String> {
    let img = image::load_from_memory(bytes).map_err(|e| format!("图片数据无效：{e}"))?;
    Ok(img.to_rgba8())
}

/// 复制图片到剪贴板
pub fn copy_to_clipboard(img: &RgbaImage) -> Result<(), String> {
    let mut cb = Clipboard::new().map_err(|e| format!("无法访问剪贴板：{e}"))?;
    cb.set_image(arboard::ImageData {
        width: img.width() as usize,
        height: img.height() as usize,
        bytes: Cow::Borrowed(img.as_raw()),
    })
    .map_err(|e| format!("复制到剪贴板失败：{e}"))
}

/// 公共收尾：保存 PNG + 复制剪贴板 + 写历史 + 广播事件
pub fn finish_save(app: &tauri::AppHandle, img: RgbaImage) -> Result<HistoryItem, String> {
    let dir = save_dir(app)?;
    let name = format!("快截_{}.png", Local::now().format("%Y%m%d_%H%M%S"));
    let path = dir.join(&name);
    img.save(&path).map_err(|e| e.to_string())?;

    // 复制到剪贴板（失败不阻断保存流程）
    let _ = copy_to_clipboard(&img);

    let state = app.state::<AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let item = db.insert("screenshot", &path, img.width(), img.height())?;

    // 通知主窗口刷新历史列表
    let _ = app.emit("history-updated", "screenshot");

    Ok(item)
}

/// 主屏全屏截图
pub fn capture_fullscreen_and_save(app: &tauri::AppHandle) -> Result<HistoryItem, String> {
    let screen = primary_screen()?;
    let img: RgbaImage = screen.capture().map_err(|e| e.to_string())?;
    finish_save(app, img)
}
