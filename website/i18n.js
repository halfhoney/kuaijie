// 官网国际化：根据浏览器语言自动切换（简/繁/英），手动选择优先并记忆
const SITE_I18N = {
  "zh-CN": {
    "nav.features": "功能", "nav.personas": "适合谁", "nav.specs": "特性", "nav.showcase": "预览",
    "nav.download": "免费下载",
    "hero.badge": "免费 · 本地运行 · 多平台支持",
    "hero.title1": "屏幕上的灵感，", "hero.title2": "一键定格",
    "hero.sub": "截图 · 标注 · 贴图 · 录屏 · GIF · 取色——快截把屏幕工具箱装进一个热键里",
    "hero.download": "免费下载",
    "hero.more": "了解功能 ↓",
    "hero.meta": "v0.1.0 · 约 43 MB · 无需注册 · 本地运行不上传",
    "hk.screenshot": "区域截图", "hk.video": "录屏", "hk.gif": "GIF", "hk.color": "取色",
    "features.title": "一个热键，六种能力",
    "features.sub": "冻结屏幕、框选、标注、保存——整套动作行云流水",
    "f1.t": "截图与标注", "f1.d": "冻结式框选，选区可拖动可微调；矩形、圆形、箭头、画笔、马赛克、文字六种标注，双击选区即复制。",
    "f2.t": "贴图钉住", "f2.d": "把截图钉在屏幕最上层，对照文档写代码、比稿对色都顺手。拖动移动，双击关闭，可同时钉多张。",
    "f3.t": "高清录屏", "f3.d": "区域录制输出 MP4（H.264），红框实时标示录制范围且不入镜，支持暂停/继续，帧率码率可调。",
    "f4.t": "GIF 动图", "f4.d": "框选即录，自动优化调色板与体积，演示 Bug、记录操作步骤，一张动图胜过千言万语。",
    "f5.t": "屏幕取色器", "f5.d": "放大镜十倍像素级取景，读取真实屏幕像素（无压缩色差），左键复制 HEX，设计对色好帮手。",
    "f6.t": "历史记录", "f6.d": "每张截图、每段录屏自动归档，支持搜索、网格/列表视图，随时回看、打开或清理。",
    "showcase.title": "真实界面，所见即所得",
    "showcase.sub": "以下截图均来自软件实机运行画面",
    "showcase.home": "首页", "showcase.history": "历史记录", "showcase.settings": "设置", "showcase.about": "关于",
    "personas.title": "谁在用快截", "personas.sub": "不同岗位，同一个趁手工具",
    "p1.t": "设计师", "p1.d": "取色器读真实像素，贴图对照参考稿，标注精确到像素。",
    "p2.t": "开发者", "p2.d": "报错即录即钉，日志截图钉在边上对照改码，GIF 复现 Bug。",
    "p3.t": "产品经理", "p3.d": "标注 + 序号思路清晰，录屏带操作轨迹，需求评审更直观。",
    "p4.t": "办公与客服", "p4.d": "马赛克敏感信息，一键复制进微信，问题描述一步到位。",
    "specs.title": "为效率而生的细节", "specs.sub": "轻量、干净、可定制",
    "s1": "全局热键均可自定义，冲突自动检测提示",
    "s2": "界面支持简体中文 / 繁體中文 / English",
    "s3": "托盘常驻，开机自启可选",
    "s4": "录制帧率、码率、GIF 尺寸可调",
    "s5": "纯本地运行，零上传零追踪",
    "s6": "基于 Rust + Tauri，内存占用低",
    "s7": "视频编码由 FFmpeg（LGPL）驱动",
    "s8": "离线可用，无需账号",
    "download.title": "现在就开始",
    "download.sub": "下载安装包，按下热键，截下第一张图",
    "download.btn": "免费下载 Windows 版",
    "download.note": "安装包约 43 MB · 支持 Windows 10 / 11（64 位）",
    "download.soon": "敬请期待",
    "download.notYet": "{plat} 版正在适配中，后续更新提供下载",
    "footer.right": "快截 KuaiJie · 由 Rust 与 Tauri 强力驱动",
  },
  "zh-TW": {
    "nav.features": "功能", "nav.personas": "適合誰", "nav.specs": "特性", "nav.showcase": "預覽",
    "nav.download": "免費下載",
    "hero.badge": "免費 · 本機執行 · 多平台支援",
    "hero.title1": "螢幕上的靈感，", "hero.title2": "一鍵定格",
    "hero.sub": "截圖 · 標註 · 貼圖 · 錄影 · GIF · 取色——快截把螢幕工具箱裝進一個熱鍵裡",
    "hero.download": "免費下載",
    "hero.more": "了解功能 ↓",
    "hero.meta": "v0.1.0 · 約 43 MB · 無需註冊 · 本機執行不上傳",
    "hk.screenshot": "區域截圖", "hk.video": "錄影", "hk.gif": "GIF", "hk.color": "取色",
    "features.title": "一個熱鍵，六種能力",
    "features.sub": "凍結螢幕、框選、標註、儲存——整套動作行雲流水",
    "f1.t": "截圖與標註", "f1.d": "凍結式框選，選區可拖曳可微調；矩形、圓形、箭頭、畫筆、馬賽克、文字六種標註，雙擊選區即複製。",
    "f2.t": "貼圖釘住", "f2.d": "把截圖釘在螢幕最上層，對照文件寫程式、比稿對色都順手。拖曳移動，雙擊關閉，可同時釘多張。",
    "f3.t": "高清錄影", "f3.d": "區域錄製輸出 MP4（H.264），紅框即時標示錄製範圍且不入鏡，支援暫停/繼續，幀率位元速率可調。",
    "f4.t": "GIF 動圖", "f4.d": "框選即錄，自動最佳化調色盤與體積，演示 Bug、記錄操作步驟，一張動圖勝過千言萬語。",
    "f5.t": "螢幕取色器", "f5.d": "放大鏡十倍像素級取景，讀取真實螢幕像素（無壓縮色差），左鍵複製 HEX，設計對色好幫手。",
    "f6.t": "歷史記錄", "f6.d": "每張截圖、每段錄影自動歸檔，支援搜尋、網格/清單檢視，隨時回看、開啟或清理。",
    "showcase.title": "真實介面，所見即所得",
    "showcase.sub": "以下截圖均來自軟體實機執行畫面",
    "showcase.home": "首頁", "showcase.history": "歷史記錄", "showcase.settings": "設定", "showcase.about": "關於",
    "personas.title": "誰在用快截", "personas.sub": "不同崗位，同一個趁手工具",
    "p1.t": "設計師", "p1.d": "取色器讀真實像素，貼圖對照參考稿，標註精確到像素。",
    "p2.t": "開發者", "p2.d": "報錯即錄即釘，日誌截圖釘在邊上對照改碼，GIF 復現 Bug。",
    "p3.t": "產品經理", "p3.d": "標註 + 序號思路清晰，錄影帶操作軌跡，需求評審更直觀。",
    "p4.t": "辦公與客服", "p4.d": "馬賽克敏感資訊，一鍵複製進微信，問題描述一步到位。",
    "specs.title": "為效率而生的細節", "specs.sub": "輕量、乾淨、可客製",
    "s1": "全域熱鍵均可自訂，衝突自動偵測提示",
    "s2": "介面支援简体中文 / 繁體中文 / English",
    "s3": "系統匣常駐，開機自動啟動可選",
    "s4": "錄製幀率、位元速率、GIF 尺寸可調",
    "s5": "純本機執行，零上傳零追蹤",
    "s6": "基於 Rust + Tauri，記憶體佔用低",
    "s7": "影片編碼由 FFmpeg（LGPL）驅動",
    "s8": "離線可用，無需帳號",
    "download.title": "現在就開始",
    "download.sub": "下載安裝包，按下熱鍵，截下第一張圖",
    "download.btn": "免費下載 Windows 版",
    "download.note": "安裝包約 43 MB · 支援 Windows 10 / 11（64 位）",
    "download.soon": "敬請期待",
    "download.notYet": "{plat} 版正在適配中，後續更新提供下載",
    "footer.right": "快截 KuaiJie · 由 Rust 與 Tauri 強力驅動",
  },
  "en": {
    "nav.features": "Features", "nav.personas": "Who", "nav.specs": "Details", "nav.showcase": "Preview",
    "nav.download": "Free Download",
    "hero.badge": "Free · Runs locally · Cross-platform",
    "hero.title1": "Capture anything on screen, ", "hero.title2": "in one keystroke",
    "hero.sub": "Screenshot · Annotate · Pin · Record · GIF · Color picker — KuaiJie puts a screen toolbox behind one hotkey",
    "hero.download": "Free Download",
    "hero.more": "See features ↓",
    "hero.meta": "v0.1.0 · ~43 MB · No account · Runs locally, nothing uploaded",
    "hk.screenshot": "Region shot", "hk.video": "Record", "hk.gif": "GIF", "hk.color": "Pick color",
    "features.title": "One hotkey, six powers",
    "features.sub": "Freeze, select, annotate, save — one fluid motion",
    "f1.t": "Screenshot & Annotate", "f1.d": "Frozen-frame selection you can drag and fine-tune. Rectangles, ellipses, arrows, pen, mosaic and text. Double-click to copy.",
    "f2.t": "Pin on Screen", "f2.d": "Pin screenshots above everything. Perfect for references while coding or comparing colors. Drag to move, double-click to close.",
    "f3.t": "HD Recording", "f3.d": "Record any region to MP4 (H.264). A red frame marks the area but stays out of the footage. Pause/resume, adjustable fps & bitrate.",
    "f4.t": "GIF Recording", "f4.d": "Select and record — palette and size auto-optimized. Demo a bug or a workflow in one looping image.",
    "f5.t": "Color Picker", "f5.d": "A 10x magnifier reads real screen pixels (zero compression drift). Left-click copies the HEX value.",
    "f6.t": "History", "f6.d": "Every capture is archived automatically. Search, grid/list views, reopen or clean up anytime.",
    "showcase.title": "The real interface, as it is",
    "showcase.sub": "All screenshots below are taken from the actual app",
    "showcase.home": "Home", "showcase.history": "History", "showcase.settings": "Settings", "showcase.about": "About",
    "personas.title": "Who uses KuaiJie", "personas.sub": "Different roles, same handy tool",
    "p1.t": "Designers", "p1.d": "Pick true pixel colors, pin references, annotate down to the pixel.",
    "p2.t": "Developers", "p2.d": "Record errors, pin logs beside your editor, reproduce bugs in GIFs.",
    "p3.t": "Product Managers", "p3.d": "Clear numbered annotations, recordings with trails, sharper reviews.",
    "p4.t": "Office & Support", "p4.d": "Mosaic sensitive info, paste straight into chat, explain issues in one shot.",
    "specs.title": "Details built for speed", "specs.sub": "Light, clean, customizable",
    "s1": "Every hotkey is customizable, with conflict detection",
    "s2": "UI in 简体中文 / 繁體中文 / English",
    "s3": "Lives in the tray, optional launch at login",
    "s4": "Adjustable fps, bitrate and GIF width",
    "s5": "Fully local. Zero upload, zero tracking",
    "s6": "Built with Rust + Tauri, low memory footprint",
    "s7": "Video encoding powered by FFmpeg (LGPL)",
    "s8": "Works offline, no account needed",
    "download.title": "Get started now",
    "download.sub": "Download, press the hotkey, take your first shot",
    "download.btn": "Free Download for Windows",
    "download.note": "~43 MB installer · Windows 10 / 11 (64-bit)",
    "download.soon": "coming soon",
    "download.notYet": "The {plat} build is in the works — check back soon",
    "footer.right": "KuaiJie · Powered by Rust & Tauri",
  },
};

(function () {
  const SAVED = localStorage.getItem("kuaijie-lang");
  let lang = SAVED;
  if (!lang) {
    const nav = (navigator.language || "zh-CN").toLowerCase();
    if (nav.includes("tw") || nav.includes("hk") || nav.includes("hant")) lang = "zh-TW";
    else if (nav.startsWith("zh")) lang = "zh-CN";
    else lang = "en";
  }

  function apply(l) {
    lang = l;
    localStorage.setItem("kuaijie-lang", l);
    const dict = SITE_I18N[l] || SITE_I18N["zh-CN"];
    document.querySelectorAll("[data-i18n]").forEach((el) => {
      const k = el.getAttribute("data-i18n");
      if (dict[k]) el.textContent = dict[k];
    });
    document.documentElement.lang = l;
    document.querySelectorAll(".lang-btn").forEach((b) => {
      b.classList.toggle("active", b.dataset.lang === l);
    });
    const titles = {
      "zh-CN": "快截 KuaiJie - 截图 · 录屏 · GIF · 取色，一站式屏幕工具",
      "zh-TW": "快截 KuaiJie - 截圖 · 錄影 · GIF · 取色，一站式螢幕工具",
      "en": "KuaiJie - Screenshot · Record · GIF · Color Picker",
    };
    document.title = titles[l];
  }

  window.__applyLang = apply;
  document.addEventListener("DOMContentLoaded", () => apply(lang));
})();
