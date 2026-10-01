mod capture;
mod db;
mod recorder;
mod settings;

use db::{Db, HistoryItem};
use settings::Settings;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_opener::OpenerExt;

#[derive(Clone)]
pub struct TrayItems {
    pub snap: MenuItem<tauri::Wry>,
    pub video: MenuItem<tauri::Wry>,
    pub gif: MenuItem<tauri::Wry>,
    pub color: MenuItem<tauri::Wry>,
    pub stop: MenuItem<tauri::Wry>,
    pub autostart: CheckMenuItem<tauri::Wry>,
    pub show: MenuItem<tauri::Wry>,
    pub quit: MenuItem<tauri::Wry>,
}

pub struct AppState {
    pub db: Mutex<Db>,
    pub recorder: Mutex<Option<recorder::Recorder>>,
    pub settings: Mutex<Settings>,
    pub settings_path: PathBuf,
    pub overlay_snapshot: Mutex<Option<capture::OverlaySnapshot>>,
    /// 托盘菜单项引用，用于语言/热键/自启勾选同步刷新
    pub tray_items: Mutex<Option<TrayItems>>,
}

fn esc_shortcut() -> Shortcut {
    Shortcut::new(None, Code::Escape)
}

/// 异步打开框选层：截屏+编码耗时操作放到后台线程，避免阻塞 UI 主线程导致「无响应」
fn open_overlay(app: &tauri::AppHandle, mode: &str) {
    let app = app.clone();
    let mode = mode.to_string();
    std::thread::spawn(move || {
        if let Err(e) = open_overlay_inner(&app, &mode) {
            eprintln!("打开框选层失败: {e}");
            let _ = app.emit("record-error", format!("打开框选层失败：{e}"));
        }
    });
}

/// 打开框选窗口（先抓屏快照作为背景），mode: "screenshot" | "gif" | "video"
fn open_overlay_inner(app: &tauri::AppHandle, mode: &str) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.close();
    }
    // 先截屏做快照：框选层显示这张冻结画面，截图也直接从它裁剪，
    // 避免「关窗后再抢拍」的竞态把框选层自己截进去
    let (_bg_path, rect) = capture::make_snapshot(app)?;

    // 无边框不透明窗口手动铺满屏幕。不用 fullscreen（独占全屏与透明窗口冲突），
    // 也不用 transparent（全屏透明 WebView 在 Windows 上走软件渲染，会严重卡顿）
    let win = WebviewWindowBuilder::new(
        app,
        "overlay",
        WebviewUrl::App(format!("overlay.html?mode={mode}").into()),
    )
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .shadow(false)
    .inner_size(rect.logical_w, rect.logical_h)
    .position(rect.logical_x, rect.logical_y)
    .build()
    .map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;

    // 全局 Esc 兜底关闭：即使 WebView 卡死也能退出框选
    let gs = app.global_shortcut();
    let _ = gs.unregister(esc_shortcut());
    gs.on_shortcut(esc_shortcut(), |app, _s, event| {
        if event.state() == ShortcutState::Pressed {
            let _ = close_overlay(app);
        }
    })
    .map_err(|e| e.to_string())?;

    Ok(())
}

fn close_overlay(app: &tauri::AppHandle) -> Result<(), String> {
    // 取消 Esc 兜底热键，避免影响其他程序
    let _ = app.global_shortcut().unregister(esc_shortcut());
    // 丢弃快照数据
    if let Ok(mut snap) = app.state::<AppState>().overlay_snapshot.lock() {
        *snap = None;
    }
    if let Some(w) = app.get_webview_window("overlay") {
        w.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// 托盘菜单文案（按语言）
fn tray_texts(lang: &str) -> [&'static str; 8] {
    match lang {
        "zh-TW" => ["區域截圖", "錄影 MP4", "GIF 錄製", "取色", "停止錄製", "開機自動啟動", "顯示主視窗", "退出"],
        "en" => ["Region Screenshot", "Record MP4", "Record GIF", "Pick Color", "Stop Recording", "Launch at login", "Show Window", "Quit"],
        _ => ["区域截图", "录屏 MP4", "GIF 录制", "取色", "停止录制", "开机自启", "显示主窗口", "退出"],
    }
}

/// 按当前设置刷新托盘菜单文案（语言/热键变化后调用）
fn refresh_tray_texts(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let settings = match state.settings.lock() {
        Ok(s) => s.clone(),
        Err(_) => return,
    };
    let items = match state.tray_items.lock() {
        Ok(i) => i.clone(),
        Err(_) => return,
    };
    let Some(items) = items else { return };
    let [snap, video, gif, color, stop, autostart, show, quit] = tray_texts(&settings.language);
    let _ = items.snap.set_text(format!("{snap} ({})", settings.hotkey_screenshot));
    let _ = items.video.set_text(format!("{video} ({})", settings.hotkey_video));
    let _ = items.gif.set_text(format!("{gif} ({})", settings.hotkey_gif));
    let _ = items.color.set_text(format!("{color} ({})", settings.hotkey_color));
    let _ = items.stop.set_text(stop);
    let _ = items.autostart.set_text(autostart);
    let _ = items.show.set_text(show);
    let _ = items.quit.set_text(quit);
}

/// 按当前设置重新注册全部全局热键。
/// 容错设计：单个热键被占用只跳过并提示，绝不让整个程序起不来
fn register_hotkeys(app: &tauri::AppHandle) -> Result<(), String> {
    let settings = app
        .state::<AppState>()
        .settings
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let gs = app.global_shortcut();
    gs.unregister_all().map_err(|e| e.to_string())?;

    let mut failed: Vec<String> = Vec::new();

    match settings::parse_shortcut(&settings.hotkey_screenshot) {
        Some(sc) => {
            if let Err(e) = gs.on_shortcut(sc, |app, _s, event| {
                if event.state() == ShortcutState::Pressed {
                    open_overlay(app, "screenshot");
                }
            }) {
                failed.push(format!("截图热键 {} 被占用（{e}）", settings.hotkey_screenshot));
            }
        }
        None => failed.push(format!("截图热键无效：{}", settings.hotkey_screenshot)),
    }

    match settings::parse_shortcut(&settings.hotkey_video) {
        Some(sc) => {
            if let Err(e) = gs.on_shortcut(sc, |app, _s, event| {
                if event.state() == ShortcutState::Pressed {
                    if recorder::is_recording(app) {
                        recorder::stop(app);
                    } else {
                        open_overlay(app, "video");
                    }
                }
            }) {
                failed.push(format!("录屏热键 {} 被占用（{e}）", settings.hotkey_video));
            }
        }
        None => failed.push(format!("录屏热键无效：{}", settings.hotkey_video)),
    }

    match settings::parse_shortcut(&settings.hotkey_gif) {
        Some(sc) => {
            if let Err(e) = gs.on_shortcut(sc, |app, _s, event| {
                if event.state() == ShortcutState::Pressed {
                    if recorder::is_recording(app) {
                        recorder::stop(app);
                    } else {
                        open_overlay(app, "gif");
                    }
                }
            }) {
                failed.push(format!("GIF 热键 {} 被占用（{e}）", settings.hotkey_gif));
            }
        }
        None => failed.push(format!("GIF 热键无效：{}", settings.hotkey_gif)),
    }

    match settings::parse_shortcut(&settings.hotkey_color) {
        Some(sc) => {
            if let Err(e) = gs.on_shortcut(sc, |app, _s, event| {
                if event.state() == ShortcutState::Pressed {
                    open_overlay(app, "color");
                }
            }) {
                failed.push(format!("取色热键 {} 被占用（{e}）", settings.hotkey_color));
            }
        }
        None => failed.push(format!("取色热键无效：{}", settings.hotkey_color)),
    }

    if !failed.is_empty() {
        let msg = format!("{}，请到设置页更换组合键", failed.join("；"));
        eprintln!("{msg}");
        let _ = app.emit("record-error", msg);
    }

    Ok(())
}

/// 让窗口对屏幕捕获隐身（WDA_EXCLUDEFROMCAPTURE）：
/// 窗口正常显示，但抓屏/录屏拍不到它。老系统退化为 WDA_MONITOR（录屏中显示黑块）。
#[cfg(windows)]
fn exclude_from_capture(win: &tauri::WebviewWindow) {
    const WDA_MONITOR: u32 = 0x1;
    const WDA_EXCLUDEFROMCAPTURE: u32 = 0x11;
    if let Ok(hwnd) = win.hwnd() {
        unsafe {
            let h = hwnd.0 as _;
            let ok = windows_sys::Win32::UI::WindowsAndMessaging::SetWindowDisplayAffinity(
                h,
                WDA_EXCLUDEFROMCAPTURE,
            );
            if ok == 0 {
                windows_sys::Win32::UI::WindowsAndMessaging::SetWindowDisplayAffinity(h, WDA_MONITOR);
            }
        }
    }
}

/// 打开录制悬浮控制条（主屏底部居中）
fn open_record_bar(app: &tauri::AppHandle) -> Result<(), String> {
    if app.get_webview_window("recordbar").is_some() {
        return Ok(());
    }
    let bar = WebviewWindowBuilder::new(app, "recordbar", WebviewUrl::App("recordbar.html".into()))
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .resizable(false)
        .skip_taskbar(true)
        .inner_size(300.0, 56.0)
        .build()
        .map_err(|e| e.to_string())?;
    // 录制条不能被录进画面
    exclude_from_capture(&bar);
    if let Ok(Some(m)) = app.primary_monitor() {
        let sf = m.scale_factor();
        let lw = m.size().width as f64 / sf;
        let lh = m.size().height as f64 / sf;
        let lx = m.position().x as f64 / sf;
        let ly = m.position().y as f64 / sf;
        let _ = bar.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(
            lx + (lw - 300.0) / 2.0,
            ly + lh - 56.0 - 90.0,
        )));
    }
    Ok(())
}

/// 打开录制区域红框：透明、穿透鼠标，只画一圈边框标出正在录制的范围
fn open_record_frame(app: &tauri::AppHandle, x: f64, y: f64, w: f64, h: f64) -> Result<(), String> {
    if app.get_webview_window("recframe").is_some() {
        return Ok(());
    }
    // 框选层坐标是相对于屏幕逻辑原点的，加上原点偏移
    let (_sf, ox, oy) = capture::snapshot_meta(app).unwrap_or((1.0, 0.0, 0.0));
    let border = 3.0;
    let win = WebviewWindowBuilder::new(app, "recframe", WebviewUrl::App("recframe.html".into()))
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .inner_size(w + border * 2.0, h + border * 2.0)
        .position(ox + x - border, oy + y - border)
        .build()
        .map_err(|e| e.to_string())?;
    // 红框不能被录进画面
    exclude_from_capture(&win);
    // 鼠标事件穿透，不影响录制区域内的操作
    win.set_ignore_cursor_events(true).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn start_region_capture(app: tauri::AppHandle) -> Result<(), String> {
    open_overlay(&app, "screenshot");
    Ok(())
}

#[tauri::command]
fn start_gif_capture(app: tauri::AppHandle) -> Result<(), String> {
    if recorder::is_recording(&app) {
        return Err("已有录制任务进行中".to_string());
    }
    open_overlay(&app, "gif");
    Ok(())
}

#[tauri::command]
fn start_color_capture(app: tauri::AppHandle) -> Result<(), String> {
    open_overlay(&app, "color");
    Ok(())
}

/// 取色：读取快照真实像素颜色（前端鼠标移动时实时查询）
#[tauri::command]
fn overlay_color_at(app: tauri::AppHandle, x: f64, y: f64) -> Result<String, String> {
    capture::color_at(&app, x as u32, y as u32)
}

/// 取色提交：复制 HEX 到剪贴板，关闭取色层，通知主窗口
#[tauri::command]
async fn pick_color_commit(app: tauri::AppHandle, x: f64, y: f64) -> Result<String, String> {
    let hex = capture::color_at(&app, x as u32, y as u32)?;
    capture::copy_text(&hex)?;
    close_overlay(&app)?;
    let _ = app.emit("color-copied", hex.clone());
    Ok(hex)
}

#[tauri::command]
fn start_video_capture(app: tauri::AppHandle) -> Result<(), String> {
    if recorder::is_recording(&app) {
        return Err("已有录制任务进行中".to_string());
    }
    open_overlay(&app, "video");
    Ok(())
}

#[tauri::command]
fn cancel_capture(app: tauri::AppHandle) -> Result<(), String> {
    close_overlay(&app)
}

/// 从原始像素请求解析出图像（前端 getImageData 直传 RGBA，不经 PNG 编解码）
fn parse_pixels(req: &tauri::ipc::Request<'_>) -> Result<image::RgbaImage, String> {
    let header_u32 = |name: &str| -> Result<u32, String> {
        req.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| format!("缺少或无效的请求头：{name}"))
    };
    let w = header_u32("x-width")?;
    let h = header_u32("x-height")?;
    let tauri::ipc::InvokeBody::Raw(bytes) = req.body() else {
        return Err("像素数据必须是原始字节".to_string());
    };
    image::RgbaImage::from_raw(w, h, bytes.clone()).ok_or_else(|| "像素数据尺寸不匹配".to_string())
}

fn header_f64(req: &tauri::ipc::Request<'_>, name: &str) -> f64 {
    req.headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0)
}

/// 完成（复制到剪贴板）：立即关闭框选层（秒关），剪贴板写入放后台线程
#[tauri::command]
fn copy_region(app: tauri::AppHandle, request: tauri::ipc::Request<'_>) -> Result<(), String> {
    let img = parse_pixels(&request)?;
    close_overlay(&app)?;
    std::thread::spawn(move || {
        if let Err(e) = capture::copy_to_clipboard(&img) {
            let _ = app.emit("record-error", e);
        }
    });
    Ok(())
}

/// 保存：后台线程弹「另存为」对话框，确定后关框选层并快速编码写盘
#[tauri::command]
fn capture_region(app: tauri::AppHandle, request: tauri::ipc::Request<'_>) -> Result<(), String> {
    let img = parse_pixels(&request)?;
    std::thread::spawn(move || {
        // 对话框挂到框选层窗口下（它是置顶窗口，对话框随之置顶）
        let overlay = app.get_webview_window("overlay");
        let default_name = format!("快截_{}.png", chrono::Local::now().format("%Y%m%d_%H%M%S"));
        let mut dlg = app
            .dialog()
            .file()
            .set_file_name(&default_name)
            .set_title("保存截图");
        if let Ok(dir) = capture::save_dir(&app) {
            dlg = dlg.set_directory(dir);
        }
        if let Some(w) = &overlay {
            dlg = dlg.set_parent(w);
        }
        let Some(target) = dlg.blocking_save_file() else {
            // 用户取消：通知前端恢复编辑状态
            let _ = app.emit("save-cancelled", ());
            return;
        };
        let path = match target.into_path() {
            Ok(p) => p,
            Err(e) => {
                let _ = app.emit("record-error", e.to_string());
                return;
            }
        };
        if let Err(e) = close_overlay(&app) {
            eprintln!("关闭框选层失败: {e}");
        }
        if let Err(e) = capture::write_png_fast(&img, &path) {
            let _ = app.emit("record-error", format!("保存失败：{e}"));
            return;
        }
        let state = app.state::<AppState>();
        if let Ok(db) = state.db.lock() {
            let _ = db.insert("screenshot", &path, img.width(), img.height());
        }
        let _ = app.emit("history-updated", "screenshot");
    });
    Ok(())
}

/// 钉住：立即关闭框选层，后台写缓存并开钉住窗口
#[tauri::command]
fn pin_region(app: tauri::AppHandle, request: tauri::ipc::Request<'_>) -> Result<(), String> {
    let img = parse_pixels(&request)?;
    let css_x = header_f64(&request, "x-css-x");
    let css_y = header_f64(&request, "x-css-y");
    let css_w = header_f64(&request, "x-css-w");
    let css_h = header_f64(&request, "x-css-h");
    let (_scale, origin_x, origin_y) = capture::snapshot_meta(&app).unwrap_or((1.0, 0.0, 0.0));
    close_overlay(&app)?;
    std::thread::spawn(move || {
        let dir = match app.path().app_cache_dir() {
            Ok(d) => d,
            Err(_) => return,
        };
        let _ = std::fs::create_dir_all(&dir);
        let name = format!("pin_{}.png", chrono::Local::now().timestamp_millis());
        if let Err(e) = capture::write_png_fast(&img, &dir.join(&name)) {
            let _ = app.emit("record-error", format!("钉住失败：{e}"));
            return;
        }
        let label = format!("pin-{}", chrono::Local::now().timestamp_millis());
        let _ = WebviewWindowBuilder::new(
            &app,
            &label,
            WebviewUrl::App(format!("pin.html?img={name}").into()),
        )
        .decorations(false)
        .always_on_top(true)
        .resizable(false)
        .skip_taskbar(true)
        .inner_size(css_w.max(50.0), css_h.max(50.0))
        .position(origin_x + css_x, origin_y + css_y)
        .build();
    });
    Ok(())
}

#[tauri::command]
async fn capture_fullscreen(app: tauri::AppHandle) -> Result<HistoryItem, String> {
    capture::capture_fullscreen_and_save(&app)
}

#[tauri::command]
async fn start_record(
    app: tauri::AppHandle,
    kind: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    scale: f64,
) -> Result<(), String> {
    close_overlay(&app)?;
    // 短暂等待框选层消失，避免第一帧录到残影（async 命令在后台线程池执行，不阻塞主线程）
    std::thread::sleep(std::time::Duration::from_millis(150));
    recorder::start(&app, kind, x, y, width, height, scale)?;
    if let Err(e) = open_record_frame(&app, x, y, width, height) {
        eprintln!("打开录制框失败: {e}");
    }
    open_record_bar(&app)?;
    Ok(())
}

#[tauri::command]
fn stop_record(app: tauri::AppHandle) {
    recorder::stop(&app);
}

#[tauri::command]
fn toggle_pause(app: tauri::AppHandle) -> bool {
    recorder::toggle_pause(&app).unwrap_or(false)
}

#[tauri::command]
fn list_history(app: tauri::AppHandle) -> Result<Vec<HistoryItem>, String> {
    let state = app.state::<AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list()
}

#[tauri::command]
fn delete_history(app: tauri::AppHandle, id: i64) -> Result<(), String> {
    let state = app.state::<AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    if let Some(path) = db.delete(id)? {
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}

#[tauri::command]
fn open_path(app: tauri::AppHandle, path: String) -> Result<(), String> {
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_save_dir(app: tauri::AppHandle) -> Result<String, String> {
    let dir = capture::save_dir(&app)?;
    Ok(dir.to_string_lossy().to_string())
}

/// 读取框选层快照图（原始字节返回，前端转 Blob URL 使用，
/// 避免 asset:// 跨域加载导致 canvas 被污染无法导出）
#[tauri::command]
fn read_snapshot(app: tauri::AppHandle) -> Result<tauri::ipc::Response, String> {
    let path = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("overlay_bg.jpg");
    let bytes = std::fs::read(path).map_err(|e| format!("读取快照失败：{e}"))?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> Result<Settings, String> {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

/// 统一应用设置的入口：校验 → 自启注册表 → 存盘 → 托盘勾选同步 → 热键重注册 → 通知前端
fn apply_settings(app: &tauri::AppHandle, settings: Settings) -> Result<(), String> {
    settings.validate()?;
    // 应用开机自启（写/删注册表 Run 键）
    let autolaunch = app.autolaunch();
    if settings.autostart {
        autolaunch.enable().map_err(|e| format!("开启开机自启失败：{e}"))?;
    } else {
        autolaunch.disable().map_err(|e| format!("关闭开机自启失败：{e}"))?;
    }
    let state = app.state::<AppState>();
    settings.save_to(&state.settings_path)?;
    // 托盘菜单勾选项同步
    if let Ok(items) = state.tray_items.lock() {
        if let Some(items) = items.as_ref() {
            let _ = items.autostart.set_checked(settings.autostart);
        }
    }
    {
        let mut current = state.settings.lock().map_err(|e| e.to_string())?;
        *current = settings;
    }
    // 托盘菜单文案（语言/热键可能变了）
    refresh_tray_texts(app);
    // 热键可能变了，重新注册
    register_hotkeys(app)?;
    // 通知设置页刷新（双向同步）
    let _ = app.emit("settings-changed", ());
    Ok(())
}

#[tauri::command]
async fn save_settings(app: tauri::AppHandle, settings: Settings) -> Result<(), String> {
    apply_settings(&app, settings)
}

#[tauri::command]
async fn pick_save_dir(app: tauri::AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .blocking_pick_folder()
        .map(|p| p.to_string())
}

/// 打开第三方许可证说明文件（安装版在安装目录 licenses/ 下，开发版在项目根目录）
#[tauri::command]
async fn open_licenses(app: tauri::AppHandle) -> Result<(), String> {
    let candidates = [
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("licenses").join("THIRD_PARTY_LICENSES.md"))),
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../THIRD_PARTY_LICENSES.md")),
    ];
    for path in candidates.into_iter().flatten() {
        if path.exists() {
            return app
                .opener()
                .open_path(path.to_string_lossy().to_string(), None::<&str>)
                .map_err(|e| e.to_string());
        }
    }
    Err("未找到许可证文件".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 崩溃日志：任何 panic 都写入数据目录 crash.log，便于排查用户现场
    std::panic::set_hook(Box::new(|info| {
        if let Some(mut dir) = dirs::data_dir() {
            dir.push("com.kuaijie.app");
            let _ = std::fs::create_dir_all(&dir);
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(dir.join("crash.log"))
            {
                let _ = writeln!(f, "[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), info);
            }
        }
        eprintln!("panic: {info}");
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        .setup(|app| {
            // 初始化数据库和设置
            let data_dir = app.path().app_data_dir().expect("无法获取应用数据目录");
            let db = Db::new(&data_dir.join("kuaijie.db"))?;
            let settings_path = data_dir.join("settings.json");
            let settings = Settings::load(&settings_path);
            let autostart_now = settings.autostart;
            app.manage(AppState {
                db: Mutex::new(db),
                recorder: Mutex::new(None),
                settings: Mutex::new(settings),
                settings_path,
                overlay_snapshot: Mutex::new(None),
                tray_items: Mutex::new(None),
            });

            // 注册全局热键（录制中再按对应热键则停止）
            register_hotkeys(app.handle())?;

            // 同步开机自启状态
            let autolaunch = app.autolaunch();
            let want_autostart = app
                .state::<AppState>()
                .settings
                .lock()
                .map(|s| s.autostart)
                .unwrap_or(false);
            let _ = if want_autostart {
                autolaunch.enable()
            } else {
                autolaunch.disable()
            };

            // 系统托盘（文案由 refresh_tray_texts 按语言统一设置）
            let snap = MenuItem::with_id(app, "snap", "-", true, None::<&str>)?;
            let video = MenuItem::with_id(app, "video", "-", true, None::<&str>)?;
            let gif = MenuItem::with_id(app, "gif", "-", true, None::<&str>)?;
            let color = MenuItem::with_id(app, "color", "-", true, None::<&str>)?;
            let stop = MenuItem::with_id(app, "stop", "-", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            // 开机自启勾选项（与设置页双向同步）
            let autostart_item =
                CheckMenuItem::with_id(app, "autostart", "-", true, autostart_now, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "-", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "-", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[&snap, &video, &gif, &color, &stop, &sep, &autostart_item, &show, &quit],
            )?;
            *app.state::<AppState>()
                .tray_items
                .lock()
                .map_err(|e| e.to_string())? = Some(TrayItems {
                snap,
                video,
                gif,
                color,
                stop,
                autostart: autostart_item,
                show,
                quit,
            });
            refresh_tray_texts(app.handle());

            let mut tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("快截")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "snap" => {
                        open_overlay(app, "screenshot");
                    }
                    "video" => {
                        open_overlay(app, "video");
                    }
                    "gif" => {
                        open_overlay(app, "gif");
                    }
                    "color" => {
                        open_overlay(app, "color");
                    }
                    "stop" => recorder::stop(app),
                    "autostart" => {
                        // 托盘点击直接翻转自启开关，走统一入口保持全链路同步
                        let mut s = match app.state::<AppState>().settings.lock() {
                            Ok(s) => s.clone(),
                            Err(_) => return,
                        };
                        s.autostart = !s.autostart;
                        let _ = apply_settings(app, s);
                    }
                    "show" => show_main_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;

            // 主窗口点关闭时最小化到托盘，而不是退出
            if let Some(main_win) = app.get_webview_window("main") {
                let win = main_win.clone();
                main_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win.hide();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_region_capture,
            start_gif_capture,
            start_video_capture,
            start_color_capture,
            overlay_color_at,
            pick_color_commit,
            cancel_capture,
            capture_region,
            copy_region,
            pin_region,
            capture_fullscreen,
            start_record,
            stop_record,
            toggle_pause,
            list_history,
            delete_history,
            open_path,
            get_save_dir,
            read_snapshot,
            get_settings,
            save_settings,
            pick_save_dir,
            open_licenses,
        ])
        .run(tauri::generate_context!())
        .expect("运行快截时发生错误");
}
