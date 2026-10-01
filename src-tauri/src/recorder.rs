use crate::capture::{physical_rect, save_dir};
use crate::db::HistoryItem;
use crate::AppState;
use chrono::Local;
use image::RgbaImage;
use screenshots::Screen;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};

const MAX_SECS: u64 = 60;

const VIDEO_MAX_SECS: u64 = 600;

pub struct Recorder {
    pub stop: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
}

pub fn is_recording(app: &tauri::AppHandle) -> bool {
    app.state::<AppState>()
        .recorder
        .lock()
        .map(|r| r.is_some())
        .unwrap_or(false)
}

/// 切换暂停/继续，返回切换后的暂停状态（true = 已暂停）
pub fn toggle_pause(app: &tauri::AppHandle) -> Option<bool> {
    let state = app.state::<AppState>();
    let rec = state.recorder.lock().ok()?;
    rec.as_ref().map(|r| {
        let new = !r.paused.load(Ordering::SeqCst);
        r.paused.store(new, Ordering::SeqCst);
        new
    })
}

/// 在后台线程开始录制。kind 目前支持 "gif"。
pub fn start(
    app: &tauri::AppHandle,
    kind: String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    scale: f64,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    {
        let rec = state.recorder.lock().map_err(|e| e.to_string())?;
        if rec.is_some() {
            return Err("已有录制任务进行中".to_string());
        }
    }

    let rect = physical_rect(x, y, w, h, scale)?;
    let stop = Arc::new(AtomicBool::new(false));
    let paused = Arc::new(AtomicBool::new(false));
    let stop_flag = stop.clone();
    let pause_flag = paused.clone();
    let app_handle = app.clone();

    std::thread::spawn(move || {
        let result = match kind.as_str() {
            "gif" => record_gif(&app_handle, rect, &stop_flag, &pause_flag),
            "video" => record_video(&app_handle, rect, &stop_flag, &pause_flag),
            _ => Err(format!("暂不支持的录制类型：{kind}")),
        };
        // 清理录制状态、关闭悬浮控制条和录制框
        if let Ok(mut rec) = app_handle.state::<AppState>().recorder.lock() {
            *rec = None;
        }
        if let Some(w) = app_handle.get_webview_window("recordbar") {
            let _ = w.close();
        }
        if let Some(w) = app_handle.get_webview_window("recframe") {
            let _ = w.close();
        }
        match result {
            Ok(item) => {
                let _ = app_handle.emit("history-updated", item.kind);
            }
            Err(e) => {
                let _ = app_handle.emit("record-error", e);
            }
        }
    });

    let mut rec = state.recorder.lock().map_err(|e| e.to_string())?;
    *rec = Some(Recorder { stop, paused });
    Ok(())
}

pub fn stop(app: &tauri::AppHandle) {
    if let Ok(rec) = app.state::<AppState>().recorder.lock() {
        if let Some(r) = rec.as_ref() {
            r.stop.store(true, Ordering::SeqCst);
        }
    }
}

/// 录制循环开头调用：返回 true 表示应结束；暂停时挂起等待
fn should_stop(stop: &AtomicBool, paused: &AtomicBool) -> bool {
    loop {
        if stop.load(Ordering::SeqCst) {
            return true;
        }
        if !paused.load(Ordering::SeqCst) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// GIF 录制主循环：按固定帧率抓帧 → 裁剪 → 缩放 → 流式写入 GIF 编码器。
/// 边录边编码到磁盘，避免长录制把内存吃光。
fn record_gif(
    app: &tauri::AppHandle,
    rect: (i32, i32, u32, u32, u32, u32),
    stop: &AtomicBool,
    paused: &AtomicBool,
) -> Result<HistoryItem, String> {
    let settings = app
        .state::<AppState>()
        .settings
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let fps = settings.gif_fps.max(1) as u64;
    let max_width = settings.gif_max_width;

    let (anchor_x, anchor_y, rx, ry, pw, ph) = rect;
    let screen = Screen::from_point(anchor_x, anchor_y).map_err(|e| e.to_string())?;

    // 超宽区域按比例缩小，控制 GIF 体积
    let ratio = if pw > max_width {
        max_width as f32 / pw as f32
    } else {
        1.0
    };
    let out_w = ((pw as f32) * ratio).max(1.0) as u16;
    let out_h = ((ph as f32) * ratio).max(1.0) as u16;

    let dir = save_dir(app)?;
    let path = dir.join(format!("快截_{}.gif", Local::now().format("%Y%m%d_%H%M%S")));
    let file = File::create(&path).map_err(|e| e.to_string())?;

    let mut encoder = gif::Encoder::new(file, out_w, out_h, &[]).map_err(|e| e.to_string())?;
    encoder.set_repeat(gif::Repeat::Infinite).map_err(|e| e.to_string())?;

    let frame_dur = Duration::from_millis(1000 / fps);
    let started = Instant::now();
    let mut last_frame_at: Option<Instant> = None;

    loop {
        if should_stop(stop, paused) {
            break;
        }
        if started.elapsed() > Duration::from_secs(MAX_SECS) {
            break;
        }
        let t0 = Instant::now();

        let img: RgbaImage = screen.capture().map_err(|e| e.to_string())?;
        let cropped = image::imageops::crop_imm(&img, rx, ry, pw, ph).to_image();
        let resized = image::imageops::resize(
            &cropped,
            out_w as u32,
            out_h as u32,
            image::imageops::FilterType::Triangle,
        );
        let mut rgba = resized.into_raw();
        // speed=10：量化质量与速度的折中
        let mut frame = gif::Frame::from_rgba_speed(out_w, out_h, &mut rgba, 10);
        // 按真实帧间隔设置延迟（单位 10ms），抓帧慢了也不会快进
        let now = Instant::now();
        let delay_ms = last_frame_at
            .map(|t| now.duration_since(t).as_millis() as u64)
            .unwrap_or(1000 / fps);
        last_frame_at = Some(now);
        frame.delay = (delay_ms / 10).clamp(2, 200) as u16;
        encoder.write_frame(&frame).map_err(|e| e.to_string())?;

        let elapsed = t0.elapsed();
        if elapsed < frame_dur {
            std::thread::sleep(frame_dur - elapsed);
        }
    }
    drop(encoder); // flush 并收尾 GIF 文件

    let state = app.state::<AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.insert("gif", &path, out_w as u32, out_h as u32)
}

/// 定位 sidecar ffmpeg：生产环境在 exe 同目录，开发环境在 src-tauri/binaries
fn ffmpeg_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("ffmpeg.exe");
            if p.exists() {
                return p;
            }
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries/ffmpeg-x86_64-pc-windows-msvc.exe")
}

/// MP4 录制：抓帧（RGBA 原始数据）→ 管道喂给 ffmpeg → libopenh264 编码。
/// 使用 BtbN 的 LGPL 静态构建，独立进程调用，可商用分发。
fn record_video(
    app: &tauri::AppHandle,
    rect: (i32, i32, u32, u32, u32, u32),
    stop: &AtomicBool,
    paused: &AtomicBool,
) -> Result<HistoryItem, String> {
    let settings = app
        .state::<AppState>()
        .settings
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let video_fps = settings.video_fps.max(5) as u64;
    let bitrate = format!("{}k", settings.video_bitrate_k);

    let (anchor_x, anchor_y, rx, ry, pw, ph) = rect;
    // H.264 的 yuv420p 要求宽高为偶数
    let pw = pw & !1;
    let ph = ph & !1;
    if pw == 0 || ph == 0 {
        return Err("选区太小".to_string());
    }
    let screen = Screen::from_point(anchor_x, anchor_y).map_err(|e| e.to_string())?;

    let dir = save_dir(app)?;
    let path = dir.join(format!("快截_{}.mp4", Local::now().format("%Y%m%d_%H%M%S")));

    let ffmpeg = ffmpeg_path();
    if !ffmpeg.exists() {
        return Err(format!("未找到 ffmpeg：{}", ffmpeg.display()));
    }

    let mut cmd = Command::new(&ffmpeg);
    cmd.args([
        "-y",
        "-f", "rawvideo",
        "-pix_fmt", "rgba",
        "-s", &format!("{pw}x{ph}"),
        // 用帧到达的真实时间打时间戳，播放速度与现实一致（不再快进）；
        // 抓帧慢了只是掉帧，不会压缩时间
        "-use_wallclock_as_timestamps", "1",
        "-i", "-",
        "-c:v", "libopenh264",
        "-b:v", &bitrate,
        "-pix_fmt", "yuv420p",
        // 输出统一帧率（不足的帧由 ffmpeg 补帧）
        "-r", &video_fps.to_string(),
        &path.to_string_lossy(),
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::null())
    .stderr(Stdio::null());

    // 避免弹出黑色控制台窗口
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let mut child = cmd.spawn().map_err(|e| format!("启动 ffmpeg 失败：{e}"))?;
    let mut stdin = child.stdin.take().ok_or("无法打开 ffmpeg 标准输入")?;

    let frame_dur = Duration::from_millis(1000 / video_fps);
    let started = Instant::now();
    let mut write_err: Option<String> = None;

    loop {
        if should_stop(stop, paused) {
            break;
        }
        if started.elapsed() > Duration::from_secs(VIDEO_MAX_SECS) {
            break;
        }
        let t0 = Instant::now();

        let img: RgbaImage = screen.capture().map_err(|e| e.to_string())?;
        let cropped = image::imageops::crop_imm(&img, rx, ry, pw, ph).to_image();
        if let Err(e) = stdin.write_all(cropped.as_raw()) {
            write_err = Some(format!("写入 ffmpeg 失败：{e}"));
            break;
        }

        let elapsed = t0.elapsed();
        if elapsed < frame_dur {
            std::thread::sleep(frame_dur - elapsed);
        }
    }

    drop(stdin); // 关闭管道 → ffmpeg 收到 EOF，正常收尾 mp4（写入 moov）
    let status = child.wait().map_err(|e| e.to_string())?;
    if let Some(e) = write_err {
        let _ = std::fs::remove_file(&path);
        return Err(e);
    }
    if !status.success() {
        let _ = std::fs::remove_file(&path);
        return Err(format!("ffmpeg 异常退出：{status}"));
    }

    let state = app.state::<AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.insert("video", &path, pw, ph)
}
