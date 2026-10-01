use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    /// 自定义保存目录，None 表示默认（图片/快截）
    pub save_dir: Option<String>,
    /// 开机自启
    pub autostart: bool,
    /// 界面语言：zh-CN / zh-TW / en
    pub language: String,
    pub hotkey_color: String,
    pub gif_fps: u32,
    pub gif_max_width: u32,
    pub video_fps: u32,
    /// 视频码率，单位 kbps
    pub video_bitrate_k: u32,
    pub hotkey_screenshot: String,
    pub hotkey_video: String,
    pub hotkey_gif: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            save_dir: None,
            autostart: false,
            language: "zh-CN".into(),
            hotkey_color: "Ctrl+Shift+C".into(),
            gif_fps: 10,
            gif_max_width: 480,
            video_fps: 15,
            video_bitrate_k: 2500,
            hotkey_screenshot: "Ctrl+Shift+A".into(),
            hotkey_video: "Ctrl+Shift+R".into(),
            hotkey_gif: "Ctrl+Shift+G".into(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let s = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, s).map_err(|e| e.to_string())
    }

    pub fn validate(&self) -> Result<(), String> {
        if !(1..=30).contains(&self.gif_fps) {
            return Err("GIF 帧率需在 1-30 之间".into());
        }
        if !(100..=3840).contains(&self.gif_max_width) {
            return Err("GIF 最大宽度需在 100-3840 之间".into());
        }
        if !(5..=60).contains(&self.video_fps) {
            return Err("录屏帧率需在 5-60 之间".into());
        }
        if !(500..=20000).contains(&self.video_bitrate_k) {
            return Err("录屏码率需在 500-20000 kbps 之间".into());
        }
        for (name, hk) in [
            ("截图", &self.hotkey_screenshot),
            ("录屏", &self.hotkey_video),
            ("GIF", &self.hotkey_gif),
            ("取色", &self.hotkey_color),
        ] {
            if parse_shortcut(hk).is_none() {
                return Err(format!("{name}热键无效：{hk}（需要 修饰键+字母/数字，如 Ctrl+Shift+A）"));
            }
        }
        Ok(())
    }
}

/// 解析 "Ctrl+Shift+A" 形式的热键字符串
pub fn parse_shortcut(s: &str) -> Option<Shortcut> {
    let mut mods = Modifiers::empty();
    let mut key_code: Option<Code> = None;
    for part in s.split('+').map(|p| p.trim()) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "shift" => mods |= Modifiers::SHIFT,
            "alt" => mods |= Modifiers::ALT,
            "win" | "super" | "meta" => mods |= Modifiers::SUPER,
            "" => {}
            k => {
                if key_code.is_some() {
                    return None;
                }
                key_code = Some(parse_key(k)?);
            }
        }
    }
    if mods.is_empty() {
        return None;
    }
    Some(Shortcut::new(Some(mods), key_code?))
}

fn parse_key(k: &str) -> Option<Code> {
    let upper = k.to_ascii_uppercase();
    let code = match upper.as_str() {
        "A" => Code::KeyA, "B" => Code::KeyB, "C" => Code::KeyC, "D" => Code::KeyD,
        "E" => Code::KeyE, "F" => Code::KeyF, "G" => Code::KeyG, "H" => Code::KeyH,
        "I" => Code::KeyI, "J" => Code::KeyJ, "K" => Code::KeyK, "L" => Code::KeyL,
        "M" => Code::KeyM, "N" => Code::KeyN, "O" => Code::KeyO, "P" => Code::KeyP,
        "Q" => Code::KeyQ, "R" => Code::KeyR, "S" => Code::KeyS, "T" => Code::KeyT,
        "U" => Code::KeyU, "V" => Code::KeyV, "W" => Code::KeyW, "X" => Code::KeyX,
        "Y" => Code::KeyY, "Z" => Code::KeyZ,
        "0" => Code::Digit0, "1" => Code::Digit1, "2" => Code::Digit2, "3" => Code::Digit3,
        "4" => Code::Digit4, "5" => Code::Digit5, "6" => Code::Digit6, "7" => Code::Digit7,
        "8" => Code::Digit8, "9" => Code::Digit9,
        "F1" => Code::F1, "F2" => Code::F2, "F3" => Code::F3, "F4" => Code::F4,
        "F5" => Code::F5, "F6" => Code::F6, "F7" => Code::F7, "F8" => Code::F8,
        "F9" => Code::F9, "F10" => Code::F10, "F11" => Code::F11, "F12" => Code::F12,
        "SPACE" => Code::Space,
        "TAB" => Code::Tab,
        "ENTER" => Code::Enter,
        _ => return None,
    };
    Some(code)
}
