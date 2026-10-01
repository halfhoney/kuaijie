# 第三方组件许可证说明（THIRD-PARTY LICENSES）

「快截」使用了以下开源组件。按各许可证要求，本分发中附带的组件均为**未修改的原版**，
相关许可证文本见各组件官方仓库。发布安装包时请将此文件一并分发。

## Rust 依赖（宽松许可证，允许闭源商用）

| 组件 | 许可证 | 仓库 |
|---|---|---|
| Tauri (tauri, tauri-plugin-*) | MIT / Apache-2.0 | https://github.com/tauri-apps/tauri |
| screenshots | MIT | https://github.com/nashaofu/screenshots-rs |
| image | MIT | https://github.com/image-rs/image |
| gif | MIT / Apache-2.0 | https://github.com/image-rs/image-gif |
| arboard | Apache-2.0 / MIT | https://github.com/1Password/arboard |
| rusqlite | MIT | https://github.com/rusqlite/rusqlite |
| serde / serde_json | MIT / Apache-2.0 | https://github.com/serde-rs/serde |
| chrono | MIT / Apache-2.0 | https://github.com/chronotope/chrono |
| dirs | MIT / Apache-2.0 | https://github.com/dirs-dev/dirs-rs |

## FFmpeg（视频编码，独立子进程调用）

- **许可证：LGPL v2.1 或更高版本**
- 构建来源（未修改的官方自动构建）：https://github.com/BtbN/FFmpeg-Builds
- FFmpeg 源代码：https://github.com/FFmpeg/FFmpeg
- 合规方式：本软件以**独立可执行文件（sidecar 子进程）**方式调用 FFmpeg，
  未链接、未修改 FFmpeg。用户可自由替换同目录下的 `ffmpeg.exe`。
- 注意：本软件特意选用 **LGPL 构建**（不含 libx264 等 GPL 组件）。
  如需自行替换 FFmpeg，请勿使用 GPL 构建，否则将改变分发义务。

### libopenh264（H.264 编码器，随 FFmpeg 提供）

- **许可证：BSD 2-Clause**
- 版权所有：Cisco Systems
- 仓库：https://github.com/cisco/openh264

## WebView2 运行时

- 本软件界面由系统自带的 Microsoft Edge WebView2 渲染，属操作系统组件，
  不随本软件分发。
