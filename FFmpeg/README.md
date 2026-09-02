# FFmpeg 分发清单

Maya 的 Windows NPM 包固定携带以下工具，不在运行时下载：

- FFmpeg / FFprobe：`7.1.1-essentials_build-www.gyan.dev`
- 上游来源：<https://www.gyan.dev/ffmpeg/builds/>
- 架构：Windows x86_64

`checksums.sha256` 是构建、打包和运行时共用的完整性清单，十六进制哈希不区分大小写。更新二进制时必须同步更新版本说明和 SHA-256。

- `cargo build --release` 会在构建期间校验两个源文件，并复制到 Cargo 实际生成的 `maya.exe` 同级目录；支持 `CARGO_TARGET_DIR` 和 `CARGO_BUILD_TARGET`；
- `cargo make package` 从 Cargo JSON 消息记录本次实际生成的 executable 路径和哈希，拒绝已改变或版本不匹配的 `maya.exe`，再从其同级目录复制经过校验的 sidecar；
- Maya 在视频转换前再次校验与自身同目录的两个二进制。
