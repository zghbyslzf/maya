# FFmpeg 分发清单

Maya 的 Windows NPM 包固定携带以下工具，不在运行时下载：

- FFmpeg / FFprobe：`7.1.1-essentials_build-www.gyan.dev`
- 上游来源：<https://www.gyan.dev/ffmpeg/builds/>
- 架构：Windows x86_64

`checksums.sha256` 是打包和运行时共用的完整性清单。更新二进制时必须同步更新版本说明和 SHA-256；`cargo make package` 会在复制前后校验清单，Maya 在视频转换前也会校验与自身同目录的两个二进制。
