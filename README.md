# Maya CLI

Maya 是仅支持 Windows 的项目维护工具，提供目录清理、Git 操作、归档、图片压缩和 MP4 转 M3U8 能力。

## 安装

```powershell
npm install --global maya-cli-rs
```

NPM 包固定携带经过 SHA-256 校验的 FFmpeg 7.1.1 与 FFprobe，因此视频转换无需联网下载或手动安装。若二进制缺失或校验失败，命令会以非零退出码失败并提示重新安装。

## 使用

```powershell
# 清理；n 是 node_modules 的兼容值别名
maya clean . --types n
maya clean . --types lock

# Git add、commit、push；m 是 add-commit-push 的兼容值别名
maya git . --ops m --message "feat: update"

# 按 .gitignore 规则归档，或归档 Vite 输出目录
maya pack . --type g
maya pack . --type a
maya pack C:\project --type a --out-dir dist

# 图片压缩
maya optimize . --types all
maya optimize . --types png jpeg --new-file --jpeg-quality 85
maya optimize . --types all --failure-policy fail-fast

# 视频转换
maya transform . --types mp4 m3u8
maya transform C:\videos --types mp4 m3u8 --failure-policy continue
```

路径参数均可省略，默认值为当前目录 `.`。命令名仍支持 `c/g/p/o/t` 别名，值别名仍支持 `n/m/g/a`；推荐文档中的完整子命令形式。

## 全局输出选项

```powershell
maya --quiet clean . --types lock
maya --no-progress optimize . --types all
```

- `--quiet`：不输出非错误信息；
- `--no-progress`：保留结果摘要，但禁用长任务进度输出。

使用 `maya --help` 或 `maya <子命令> --help` 查看完整参数。

## 开发与发布

```powershell
cargo make verify   # fmt、Clippy、测试、release build、发布配置检查
cargo make package  # verify、同步 NPM 版本、组装并冒烟测试 NPM 包
cargo make publish  # 显式发布到 npm
```

Cargo workspace 的版本是唯一版本源；普通 `cargo make build` 和 `cargo make verify` 不会修改版本或发布。
