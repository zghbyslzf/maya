# Maya CLI Rust 开发技术文档

> 面向 Rust 初学者和第一次接手 Maya 项目的开发者
>
> 文档版本：2026-09-02
>
> 适用仓库版本：Cargo workspace 0.1.55

## 这份文档解决什么问题

这不是一份只告诉你“如何运行命令”的用户手册，而是一份接手项目时可以从头读到尾的开发说明。你可以通过它回答下面这些问题：

- 项目为什么有多个 crate？每个 crate 应该放什么代码？
- 用户输入一条 maya optimize 命令后，代码具体经过哪些层？
- Result、Error、trait、Arc、async 在本项目里分别解决什么问题？
- 新增一个命令、参数或业务能力时，应该修改哪些文件？
- 为什么图片、ZIP、视频都要使用临时文件或临时目录？
- 如何运行单元测试、集成测试、Clippy 和发布冒烟测试？
- Release 构建为什么要复制 FFmpeg？如何处理自定义 Cargo 目标目录？
- 遇到构建失败、参数错误、FFmpeg 校验失败时，应该从哪里开始排查？

如果你是 Rust 新手，建议按章节顺序阅读；如果你只是要修改某一块功能，可以先阅读“目录结构”与对应 crate 的章节，再回到“如何开发一个新功能”。

## 目录

1. [项目定位与重要约定](#1-项目定位与重要约定)
2. [第一次准备开发环境](#2-第一次准备开发环境)
3. [目录结构与文件职责](#3-目录结构与文件职责)
4. [从命令行到业务代码](#4-从命令行到业务代码)
5. [CLI 命令详解](#5-cli-命令详解)
6. [四个能力 crate 详解](#6-四个能力-crate-详解)
7. [Rust 新手必须理解的项目概念](#7-rust-新手必须理解的项目概念)
8. [如何开发一个新功能](#8-如何开发一个新功能)
9. [测试策略与验证方法](#9-测试策略与验证方法)
10. [构建、打包与发布](#10-构建打包与发布)
11. [常见问题与排查顺序](#11-常见问题与排查顺序)
12. [提交代码前检查清单](#12-提交代码前检查清单)
13. [术语表与延伸阅读](#13-术语表与延伸阅读)

---

## 1. 项目定位与重要约定

### 1.1 Maya 是什么

Maya 是一个仅支持 Windows 的命令行工具，把日常项目维护中比较重复的工作统一起来：

| 能力 | 命令 | 作用 |
| --- | --- | --- |
| 清理 | maya clean | 删除 node_modules 和常见前端锁文件 |
| Git | maya git | 执行 git add、检查暂存区、commit、push |
| 归档 | maya pack | 按 .gitignore 或 Vite 输出目录创建 ZIP |
| 图片 | maya optimize | 压缩 PNG/JPEG，可覆盖原文件或生成带 _c 后缀的新文件 |
| 视频 | maya transform | 将 MP4 转换为 HLS（M3U8 加分片） |

### 1.2 必须遵守的项目约定

1. **只支持 Windows。** 源码、FFmpeg 二进制和 PowerShell 发布脚本都以 Windows 为目标，不要为了“看起来跨平台”随意加入未验证的分支。
2. **业务库不直接打印终端内容。** maya-fs、maya-git、maya-media 返回报告、结果或进度事件；终端展示统一由根程序的 Presenter 完成。
3. **错误使用 maya_core::Error。** 不要在库内部随意 unwrap，或返回没有上下文的字符串错误。
4. **破坏性写入必须可恢复。** 图片和 ZIP 使用同目录临时文件，视频使用同级临时目录，业务成功后才替换旧结果。
5. **扫描错误不能静默丢失。** 不要使用 filter_map 后丢弃错误，也不要用 flatten 把遍历错误吞掉。
6. **所有路径显式传递。** 业务 API 使用 &Path，不要在库代码里隐式读取 current_dir。
7. **版本以根 Cargo.toml 的 workspace 版本为准。** NPM 版本由脚本同步，不要手动制造两个长期不一致的版本源。
8. **发布不是普通构建。** cargo make package 会执行完整检查、组装 NPM 包和冒烟测试；真正的 npm publish 只能通过显式发布任务触发。

### 1.3 先建立安全意识

下面几条命令会修改或删除文件：

- maya clean 会删除目录或锁文件；
- maya optimize 默认可能覆盖原图片；
- maya git 会提交并推送远端；
- maya transform 会替换同名的视频输出目录；
- cargo make package 会清理并重建 pkg/release。

学习项目时优先使用临时目录。第一次尝试删除、覆盖或 Git 操作前，先执行 --help，确认路径和参数确实指向预期位置。

---

## 2. 第一次准备开发环境

### 2.1 必需软件

| 软件 | 用途 | 检查命令 |
| --- | --- | --- |
| Windows 10/11 | 项目目标平台 | winver |
| Git | 获取代码、Git 功能和版本管理 | git --version |
| Rustup + Rust MSVC | 编译 Rust | rustc --version、cargo --version |
| Visual Studio Build Tools | Windows MSVC 链接器 | 安装“使用 C++ 的桌面开发”工作负载 |
| PowerShell | 构建和发布脚本 | $PSVersionTable.PSVersion |
| Node.js/npm | NPM 打包和发布验证 | node --version、npm --version |
| cargo-make | 执行 Makefile.toml 任务 | cargo make --version |

Rust 建议使用 MSVC 工具链：

~~~powershell
rustup default stable-x86_64-pc-windows-msvc
rustup show
~~~

如果 cargo build 报找不到 link.exe，通常不是 Rust 源码问题，而是 Visual Studio C++ Build Tools 没装好，或者当前终端没有加载对应开发环境。

### 2.2 获取代码并进入仓库

~~~powershell
git clone https://github.com/zghbyslzf/maya.git
Set-Location .\maya
~~~

确认自己位于包含 Cargo.toml、src 和 crates 的仓库根目录：

~~~powershell
Get-ChildItem
Test-Path .\Cargo.toml
~~~

### 2.3 第一次编译

开发编译：

~~~powershell
cargo build
~~~

运行帮助：

~~~powershell
cargo run -- --help
~~~

这里的双横线很重要：它把前面的参数交给 Cargo，把后面的参数交给 Maya。例如：

~~~powershell
cargo run -- clean . --types node_modules
~~~

Release 编译：

~~~powershell
cargo build --release
~~~

Release 编译完成后，根目录 build.rs 会校验 FFmpeg/checksums.sha256，并把 ffmpeg.exe、ffprobe.exe 复制到实际 maya.exe 同级目录。因此视频转换应使用 Release 产物或 cargo-make 任务。

### 2.4 第一次验证

按由快到慢的顺序执行：

~~~powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
~~~

如果三条命令都成功，说明格式、静态检查和现有测试没有问题。完整发布前检查使用：

~~~powershell
cargo make verify
~~~

---

## 3. 目录结构与文件职责

### 3.1 总体结构

~~~text
maya/
├─ Cargo.toml                         # workspace、版本、共享依赖和根 CLI 包配置
├─ Cargo.lock                         # 锁定所有依赖的精确版本
├─ Makefile.toml                      # cargo-make 任务编排
├─ build.rs                           # Release 构建时校验并部署 FFmpeg sidecar
├─ src/                               # 根 maya 二进制：CLI、分发和终端展示
│  ├─ main.rs                         # 程序入口、错误转退出码
│  ├─ cli.rs                          # clap 参数和类型定义
│  ├─ presenter.rs                    # 终端输出、摘要、进度展示
│  └─ modules/                        # 很薄的命令适配层
│     ├─ clean_ops.rs
│     ├─ git_ops.rs
│     ├─ pack_ops.rs
│     ├─ optimize_ops.rs
│     └─ transform_ops.rs
├─ crates/
│  ├─ maya-core/                      # 共享 Error、Result、Report、进度和失败策略
│  ├─ maya-fs/                        # 文件扫描、清理、归档、原子写入
│  ├─ maya-git/                       # Git 外部进程边界和 Git 工作流
│  └─ maya-media/                     # 图片压缩和视频转换
├─ tests/
│  └─ integration_tests.rs            # 从用户角度启动 maya 的 CLI 集成测试
├─ FFmpeg/
│  ├─ ffmpeg.exe                      # 固定版本的 FFmpeg sidecar
│  ├─ ffprobe.exe                     # 固定版本的 FFprobe sidecar
│  ├─ checksums.sha256                # 构建、打包、运行时共用的 SHA-256 清单
│  └─ README.md                       # sidecar 来源和分发说明
├─ scripts/
│  ├─ build-release.ps1               # 记录本次实际 maya.exe 路径和哈希
│  ├─ assemble-package.ps1            # 组装 pkg/release
│  ├─ package-smoke-test.ps1          # 检查 NPM 包、CLI、sidecar 和版本
│  ├─ verify-release-config.ps1       # 发布配置和 sidecar 清单检查
│  └─ sync-package-version.ps1        # 将 Cargo 版本同步到 package.json
├─ pkg/                               # NPM 包元数据和用户说明
├─ docs/                              # 设计、优化和架构审查记录
└─ 开发技术文档/                      # 当前这份新手开发手册
~~~

### 3.2 为什么根目录既是 workspace 又是 package

根 Cargo.toml 同时包含：

- [workspace]：把 crates/maya-* 组织成一个统一构建、测试和依赖解析单元；
- [package]：根包本身叫 maya，产出最终 CLI 二进制。

这样公共能力可以独立成库并单元测试，最终 CLI 又能直接依赖这些库。执行 cargo test --workspace 时，Cargo 会测试根包和所有 workspace 成员。

### 3.3 应该把代码放在哪一层

| 你要做的事情 | 应放位置 |
| --- | --- |
| 增加命令名称、参数、枚举值 | src/cli.rs |
| 把 CLI 参数转换成业务 Options | src/modules 下对应文件 |
| 终端打印中文摘要或进度 | src/presenter.rs |
| 可被多个入口复用的文件操作 | crates/maya-fs |
| 图片、视频领域逻辑 | crates/maya-media |
| Git 命令和外部进程适配 | crates/maya-git |
| 公共错误、报告、进度接口 | crates/maya-core |
| Release 时复制二进制 | 根目录 build.rs |
| NPM 组装和发布检查 | scripts 下的 PowerShell 脚本 |

不要把真正的文件处理代码写进 src/modules，也不要让库层为了显示进度直接 println。模块层的职责是“翻译”，不是“承载全部业务”。

---

## 4. 从命令行到业务代码

### 4.1 总体调用链

~~~text
用户输入命令
    │
    ▼
src/main.rs
    ├─ Cli::parse 解析参数
    ├─ 创建 Presenter
    ├─ match Command 分发
    ▼
src/modules 下的命令适配层
    ├─ 校验 CLI 特有组合
    ├─ 构造业务 Options
    └─ 调用某个能力 crate
    ▼
crates/maya-*
    ├─ 扫描、处理、调用外部工具
    ├─ 返回 Result、Report、Outcome
    └─ 通过 ProgressSink 发出事件
    ▼
src/presenter.rs
    ├─ 把 Report 转成中文摘要
    ├─ 打印 warning/error
    └─ main.rs 将 Error 映射为进程退出码
~~~

### 4.2 以 optimize 为例

执行：

~~~powershell
maya optimize . --types png --new-file --jpeg-quality 85
~~~

代码流程如下：

1. clap 将 png 解析为 OptimizeType::Png，将 --new-file 解析为 bool，将质量解析为 u8。
2. src/modules/optimize_ops.rs 确认图片类型只有一种，把 CLI 类型转换为 maya_media::image::ImageType。
3. 模块层构造 CompressionOptions：输出模式为 NewFile，JPEG 质量为 85，失败策略使用 Continue 或用户指定值。
4. maya_media::image::compress_images_with_progress 调用 maya_fs::find_files_by_extension 扫描文件。
5. 每张图片由 codec.rs 编码；输出写入同目录临时文件，成功后原子替换或生成带 _c 后缀的文件。
6. 业务层累计 CompressionReport，其中包含扫描、成功、跳过、失败和字节数。
7. Presenter::compression 打印摘要；如果有失败项，模块层返回 Error::PartialFailure，主程序最终以退出码 3 结束。

### 4.3 以 transform 为例

执行：

~~~powershell
maya transform . --types mp4 m3u8 --failure-policy continue
~~~

流程如下：

1. CLI 只接受两个媒体类型，并且当前业务只允许 mp4 转 m3u8。
2. maya-media 的 video 模块扫描所有 mp4 文件。
3. 转换前校验程序同目录的 ffmpeg.exe 和 ffprobe.exe，并计算 SHA-256。
4. ffprobe 尝试获取视频时长，用于进度展示；获取失败时记录 warning，而不是直接把转换判为失败。
5. FFmpeg 输出写入视频同级的临时目录。
6. 监听 FFmpeg 事件，保存有上限的 stderr 尾部，并检查超时、进程退出码、播放列表和每个分片。
7. 只有所有检查成功，临时目录才会替换原来的同名输出目录。
8. 最终通过 ConversionReport 返回成功、失败和 warning 数量。

### 4.4 错误如何到达用户

main.rs 的入口不会直接 unwrap：

~~~rust
let result = run(Cli::parse()).await;
if let Err(error) = result {
    eprintln!("{error}");
    std::process::exit(i32::from(error.exit_code()));
}
~~~

Error::exit_code 将错误分成稳定类别：

| 错误类别 | 退出码 | 典型场景 |
| --- | ---: | --- |
| InvalidArgument | 2 | 参数组合不合法、JPEG 质量超出范围 |
| PartialFailure | 3 | 批量处理时部分图片或视频失败 |
| Path、Config | 4 | 路径、.gitignore、Vite 配置问题 |
| CommandFailed | 5 | Git 或外部命令返回异常退出码 |
| 其他 | 1 | I/O、压缩、视频和未知错误 |

自动化脚本应依赖退出码，不要解析中文提示来判断成功或失败。

---

## 5. CLI 命令详解

先查看当前版本实际支持的参数：

~~~powershell
maya --help
maya clean --help
maya git --help
maya pack --help
maya optimize --help
maya transform --help
~~~

路径参数都可以省略，默认是当前目录。命令名支持短别名 c/g/p/o/t，但新文档和脚本推荐使用完整命令名。

### 5.1 全局选项

~~~powershell
maya --quiet clean . --types lock
maya --no-progress optimize . --types all
~~~

- --quiet：不输出成功摘要和普通提示，但错误仍写入 stderr。
- --no-progress：保留最终摘要，只隐藏长任务的开始、逐项和 FFmpeg 进度事件。

### 5.2 clean：清理前端项目残留

~~~powershell
maya clean C:\project --types node_modules
maya clean C:\project --types lock
maya clean C:\project --types node_modules lock
~~~

支持的类型：

- node_modules，兼容别名 node-modules 和 n；
- lock，匹配 package-lock.json、yarn.lock、pnpm-lock.yaml。

实现特点：命中一个 node_modules 目录后会剪枝，不再重复遍历内部嵌套目录；删除过程中目标如果已经消失，会记录为 skipped，而不是把整个操作误报为失败。由于这是删除操作，请在执行前确认路径。

### 5.3 git：添加、提交并推送

~~~powershell
maya git C:\project --ops m --message "feat: update"
~~~

m 是当前唯一操作值，也可写成 add-commit-push。内部顺序是：

1. git add；
2. git diff --cached --quiet --exit-code 判断暂存区是否有变化；
3. 有变化时执行 git commit；
4. commit 成功后执行 git push。

判断“没有变更”依赖 Git 的退出码 0/1，不依赖 Git 的中文或英文输出，因此不会因为系统语言不同而误判。真实错误会保留程序、参数、工作目录、退出码和 stderr 尾部。

### 5.4 pack：创建 ZIP

按 .gitignore 打包：

~~~powershell
maya pack C:\project --type g
~~~

按 Vite 输出目录打包：

~~~powershell
maya pack C:\project --type a
maya pack C:\project --type a --out-dir build-output
~~~

规则：

- g/gitignore：要求项目根有 .gitignore，遵守忽略规则，并排除 .git 和符号链接；
- a/vite：优先使用 --out-dir，否则读取 vite.config.js 或 vite.config.ts 中静态的 build.outDir，没有配置时默认 dist；
- 动态 outDir 不会被猜测，会返回 UnsupportedConfig，此时使用 --out-dir；
- --out-dir 只能和 Vite 打包方式一起使用；
- ZIP 先写到临时文件，写入失败不会破坏旧 ZIP。

### 5.5 optimize：压缩图片

覆盖原文件：

~~~powershell
maya optimize C:\images --types png
maya optimize C:\images --types jpeg --jpeg-quality 85
maya optimize C:\images --types all
~~~

生成新文件：

~~~powershell
maya optimize C:\images --types png --new-file
maya optimize C:\images --types png n
~~~

规则：

- png 使用 oxipng 优化；
- jpeg 同时匹配 jpg 和 jpeg，使用 image 解码并按质量重新编码；
- all 匹配三种扩展名；
- --jpeg-quality 范围是 1 到 100，默认 80；
- 覆盖模式下，如果压缩后没有变小，会跳过，不替换原文件；
- 新文件模式生成同目录的“原文件名_c.扩展名”；
- 默认失败策略 continue：继续处理其他图片，最后以退出码 3 表示部分失败；
- fail-fast：遇到第一项错误立即返回。

### 5.6 transform：MP4 转 M3U8

~~~powershell
maya transform C:\videos --types mp4 m3u8
maya transform C:\videos --types mp4 m3u8 --failure-policy fail-fast
~~~

每个 demo.mp4 会产生同目录的 demo/index.m3u8 和若干分片。转换需要 ffmpeg.exe、ffprobe.exe 与 maya.exe 位于同一目录，并且 sidecar 的 SHA-256 必须与 FFmpeg/checksums.sha256 一致。

Debug 构建不会执行 Release sidecar 部署；需要测试视频时建议：

~~~powershell
cargo build --release
target\release\maya.exe transform C:\videos --types mp4 m3u8
~~~

视频转换默认超时时间为 2 小时。失败时会检查真实退出码、stderr 尾部和完整 HLS 产物，避免“进程失败但留下旧文件”或“只有空播放列表却被当作成功”。

---

## 6. 四个能力 crate 详解

### 6.1 maya-core：稳定的公共契约

文件：

- crates/maya-core/src/error.rs
- crates/maya-core/src/report.rs

公开内容：

- Error：统一错误枚举；
- Result：统一结果别名；
- FailurePolicy：Continue 或 FailFast；
- ProgressEvent：Started、Advanced、Message、Finished；
- ProgressSink：进度事件接收 trait；
- RemovalReport、ArchiveReport、OperationWarning：业务报告模型；
- NoopProgress：库调用方不关心进度时使用的空实现。

这个 crate 不应该依赖具体 CLI、终端库或 FFmpeg。它定义的是跨能力复用的“协议”。如果一个类型只服务于图片编码，就放在 maya-media，不要把所有类型都塞入 core。

### 6.2 maya-fs：文件系统能力

文件：

- scan.rs：严格的目录扫描和扩展名匹配；
- clean.rs：删除 node_modules 和锁文件；
- archive.rs：.gitignore/Vite 归档；
- atomic_io.rs：原子文件写入和原子目录替换；
- lib.rs：对外 re-export。

关键 API：

~~~rust
find_files(&Path, filter)
find_files_by_extension(&Path, &[&str])
find_directories_by_name_pruned(&Path, name)
clear_node_modules(&Path)
clear_lock_files(&Path)
pack_with_gitignore(&Path)
pack_vite(&Path, &VitePackOptions)
atomic_write(&Path, closure)
atomic_replace_directory(&Path, closure)
~~~

扫描函数返回 Result<Vec<PathBuf>>。遇到遍历错误会返回 Error::Traversal，不会悄悄跳过。atomic_write 的闭包只负责向临时文件写入；只有闭包成功、flush 和 sync 成功后，临时文件才会替换目标。

### 6.3 maya-git：外部进程边界

文件只有 crates/maya-git/src/lib.rs，但职责很明确：

- GitOutcome 表示“已提交并推送”或“没有需要提交的变化”；
- ProcessOutput 统一保存 success、退出码、stdout、stderr；
- ProcessRunner trait 把真实进程和测试替身隔离；
- SystemProcessRunner 使用 std::process::Command 执行 Git；
- git_add_commit_push_with_runner 是可注入 runner 的测试入口。

以后如果要增加 Git 操作，优先复用现有的命令检查、错误转换和 ProcessRunner，不要在业务函数里散落多个 Command::new。

### 6.4 maya-media：图片与视频

#### 图片模块

文件：

- image.rs：批量处理、并行调度、报告汇总；
- image/model.rs：ImageType、OutputMode、CompressionOptions、Outcome、Report；
- image/codec.rs：单文件 PNG/JPEG 编码和原子写入。

Continue 策略使用 Rayon 并行处理；FailFast 为了确定的“遇到第一项错误即返回”语义，使用顺序处理。不要在 codec 层打印信息，因为同一个库 API 也可能被非 CLI 调用。

#### 视频模块

文件：

- video.rs：批量转换、异步调度、报告；
- video/model.rs：转换 Options、Outcome、Report；
- video/binaries.rs：定位和 SHA-256 校验 sidecar；
- video/probe.rs：调用 FFprobe 获取时长、解析时间字符串；
- video/ffmpeg.rs：运行 FFmpeg、解析进度、处理超时、验证输出。

视频的单个转换在 spawn_blocking 中运行，因为 FFmpeg 进程和文件 I/O 是阻塞操作；外层 API 保持 async，便于 CLI 将来与其他异步任务组合。

---

## 7. Rust 新手必须理解的项目概念

### 7.1 `&Path`：借用一个路径，不取得所有权

```rust
pub fn clear_lock_files(root: &Path) -> Result<RemovalReport>
```

调用方拥有 `PathBuf`，函数只临时借用它。这样函数不会复制路径，也不会把调用方的变量“拿走”。

常见转换：

```rust
let owned: PathBuf = PathBuf::from("images");
let borrowed: &Path = owned.as_path();
```

如果需要把路径放入返回值或跨线程移动，再使用 `to_path_buf()` 创建拥有所有权的副本。

### 7.2 `Result`、`?` 与错误传播

```rust
let files = find_files_by_extension(path, &["png"])?;
```

`?` 的含义是：成功时取出值，失败时立刻从当前函数返回错误。它不是“忽略错误”，而是把错误交给上层统一处理。

不要这样写业务代码：

```rust
let files = find_files(path, filter).unwrap();
```

`unwrap()` 在目录权限、路径消失或磁盘故障时会直接 panic，用户只能看到程序异常退出，无法得到结构化上下文。

### 7.3 `Option`：值可能不存在

例如文件可能没有扩展名：

```rust
let extension = path.extension().and_then(|value| value.to_str());
```

不要把不存在的值强行当成空字符串；应该在业务允许时提供默认值，或转换成带上下文的 `Error::Path`。

### 7.4 `enum` 和 `match`：把有限状态写清楚

`FailurePolicy`、`ImageType`、`Command` 都是枚举。枚举比字符串更能表达“允许哪些值”，`match` 还能让编译器提醒你是否漏处理了新分支。

CLI 层使用 `clap::ValueEnum`，因此非法值会在进入业务代码前被拒绝。业务层仍需检查组合关系，例如 `optimize` 必须且只能有一个图片格式。

### 7.5 trait 与依赖注入

`ProcessRunner` 是一个 trait：它只描述“可以运行一个进程”，不规定具体实现。生产环境使用 `SystemProcessRunner`，测试使用 Fake runner。

这让测试不需要真的执行 Git，也不会改动开发者的仓库。新增外部系统时，可以沿用“trait + 真实实现 + fake 实现”的模式，但只在确实需要替换外部副作用时使用，不要为了抽象而抽象。

### 7.6 `Arc`、`Mutex` 与线程安全

- `Arc<T>`：多个线程或异步任务共享同一个拥有所有权的对象；
- `Mutex<T>`：同一时间只允许一个线程修改内部状态；
- `dyn ProgressSink`：运行时使用实现了 trait 的任意进度接收器。

`Presenter` 被多个 Rayon/FFmpeg 任务共享，因此用 `Arc<Presenter>`；它内部的进度状态用 `Mutex` 保护。锁的作用域应尽量小，不要持锁执行文件 I/O 或外部命令。

### 7.7 async 与阻塞任务

`main` 使用 Tokio runtime，`transform` 的公开入口是 async。但 FFmpeg 子进程本身是阻塞式的，所以真正的单文件转换放进 `tokio::task::spawn_blocking`。原则是：

- 网络、定时器、异步通道等适合 async；
- 大量文件读写、压缩编码、外部进程等待等阻塞操作放入 blocking 线程；
- 不要在 async 函数中直接执行长时间阻塞循环，否则会卡住 Tokio worker。

### 7.8 `pub`、`pub(crate)` 与模块边界

默认私有是 Rust 的重要安全机制。当前很多实现函数保持私有或 `pub(super)`，只有稳定 API 从 `lib.rs` re-export。新增函数前先问自己：

1. 是否真的需要被 crate 外调用？
2. 是否需要被父模块调用但不需要公开给外部？
3. 能否保持私有，减少未来兼容负担？

### 7.9 Cargo 依赖应该怎么理解

根 `Cargo.toml` 的 `[workspace.dependencies]` 是共享依赖的“版本登记处”。例如 `clap`、`tokio`、`sha2` 只在这里写版本，成员 crate 使用：

```toml
[dependencies]
clap = { workspace = true }
```

这样可以避免多个 crate 各自选择不同版本。某个 crate 需要额外 feature 时，可以在本 crate 补充：

```toml
tokio = { workspace = true, features = ["time"] }
```

新增依赖时建议按下面顺序：

1. 先确认标准库或已有依赖是否已经能解决问题；
2. 在根 `[workspace.dependencies]` 登记版本；
3. 在实际使用它的 crate 的 `Cargo.toml` 中通过 `workspace = true` 引入；
4. 运行 `cargo check` 或 `cargo test`，确认 `Cargo.lock` 发生的变化符合预期；
5. 在代码中只引入真正使用到的模块，并查看 `cargo clippy` 是否有警告。

不要为了一个很小的字符串处理引入大型依赖，也不要手动编辑 `Cargo.lock`。Cargo 会根据 `Cargo.toml` 自动解析并更新锁文件。

---

## 8. 如何开发一个新功能

下面以“新增一个文件清理类型”为例说明推荐步骤。新增图片格式、Git 操作或新命令时，流程类似。

### 8.1 先写清楚行为契约

在写代码前明确：

- 输入是什么？路径是否允许文件？
- 哪些文件会被处理？大小写是否敏感？
- 是否会删除或覆盖数据？失败时保留什么？
- 批量处理中是继续还是立即停止？
- 成功和失败应该返回什么报告？退出码是什么？
- 如何写一个不依赖真实外部环境的测试？

如果这些问题无法回答，先不要急着加参数。模糊的行为最终会变成隐含的字符串和难以维护的分支。

### 8.2 新增已有命令的参数

以给 `optimize` 增加参数为例：

1. 在 `src/cli.rs` 的 `Command::Optimize` 增加字段，并使用明确类型（布尔、整数、`ValueEnum` 或 `PathBuf`）。
2. 在 `src/main.rs` 的 match 分支中把字段传给 `handle_optimize_ops`。
3. 在 `src/modules/optimize_ops.rs` 把 CLI 类型转换成业务 Options，并做 CLI 组合校验。
4. 在 `maya-media::image::CompressionOptions` 增加业务真正需要的字段和默认值。
5. 在业务实现中使用该字段，不要让 codec 直接读取 CLI 全局状态。
6. 增加至少一个集成测试验证参数可解析和一个单元测试验证业务行为。
7. 更新根 `README.md` 和本开发文档中的示例。

### 8.3 新增一个完整子命令

推荐顺序：

1. 在合适的 crate 中先实现可复用业务 API，例如 `maya-fs::scan_xxx(&Path, &Options) -> Result<Report>`。
2. 为业务 API 写单元测试，先覆盖成功、空目录、路径错误和中途失败。
3. 在 `src/cli.rs` 增加 `Command` 枚举分支和输入类型。
4. 新建 `src/modules/xxx_ops.rs`，只做参数转换、调用和失败语义映射。
5. 在 `src/main.rs` 增加 dispatch 分支。
6. 在 `Presenter` 增加报告展示；如果是长任务，实现 `ProgressSink` 事件而不是在库层打印。
7. 在 `tests/integration_tests.rs` 增加 help、参数错误、真实临时目录执行测试。
8. 更新 README、NPM README 和本开发文档。

### 8.4 新增文件写入逻辑

必须遵循以下模板：

```rust
atomic_write(output_path, |file, _temporary_path| {
    // 向临时文件写入完整内容
    file.write_all(bytes)?;
    Ok(())
})?;
```

不要先 `remove_file(output)` 再创建新文件，也不要直接 `File::create(output)` 后在原文件上写很久。程序被终止、磁盘写满或编码失败时，原子写入能保留旧文件。

目录输出使用：

```rust
atomic_replace_directory(output_dir, |staging_dir| {
    // 在 staging_dir 中生成全部文件
    Ok(())
})?;
```

只有完整成功的临时目录才会替换旧目录；失败会清理临时目录并尽力恢复旧结果。

### 8.5 新增外部进程调用

需要外部命令时：

1. 先定义可测试的 runner trait，或复用 `maya-git::ProcessRunner` 的设计；
2. 保存程序名、参数、工作目录、退出码和 stderr；
3. 明确哪些退出码代表业务上的正常分支；
4. 对 stdout/stderr 设置大小上限，避免异常工具输出撑爆内存或错误消息；
5. 检查真实退出状态和预期产物，不能只看某个文件“存在”；
6. 为启动失败、非零退出、超时、部分产物和成功路径分别写测试。

### 8.6 新增错误类型

先判断现有 `maya_core::Error` 是否能表达。如果只是补充上下文，优先使用已有构造函数，例如：

```rust
return Err(Error::path(format!("输出目录不存在: {}", path.display())));
```

只有错误类别会影响退出码、调用方分支或长期诊断时，才新增枚举变体。新增变体时同步：

- 添加 `thiserror` 展示信息；
- 在 `exit_code()` 中决定退出码；
- 添加单元测试；
- 更新本开发文档的错误表。

### 8.7 修改代码时的推荐节奏

对于不熟悉的模块，建议每次只做一个小改动，并按下面节奏循环：

1. **先读调用方。** 从公开函数的参数和返回值开始，确认它被谁调用、调用方期待什么。
2. **再读数据模型。** 先看 `Options`、`Outcome`、`Report` 和 `Error`，它们通常比实现细节更能说明业务契约。
3. **画出失败路径。** 写出“路径不存在、权限失败、外部进程异常、输出不完整”时应该发生什么。
4. **先补测试再改实现。** 测试先描述期望行为，改错时更容易定位。
5. **运行最小验证。** 先运行目标 crate 的测试，再运行 workspace 测试和 Clippy。
6. **最后更新文档。** 参数、输出、退出码和发布流程变化都要同步到 README 与本手册。

调试时可以使用：

```powershell
cargo check -p maya-media
cargo test -p maya-media -- --nocapture
cargo run -- transform --help
```

当前项目没有统一的日志框架。临时调试信息可以在本地使用 `dbg!` 或 `println!`，但提交前应删除；正式用户可见信息必须通过 `Presenter` 设计，不能把调试输出留在能力 crate 中。

---

## 9. 测试策略与验证方法

### 9.1 三层测试

#### 库单元测试

放在实现文件底部的 `#[cfg(test)] mod tests` 中，适合测试：

- 路径和扩展名解析；
- 错误映射；
- ZIP 条目规划；
- 原子写入失败时旧文件是否保留；
- 图片报告统计；
- FFmpeg 时间解析和 HLS 输出校验；
- Fake process runner 的调用顺序。

#### CLI 集成测试

放在 `tests/integration_tests.rs`，使用 `assert_cmd::Command::cargo_bin("maya")` 从用户角度启动二进制，适合测试：

- `--help`、`--version`；
- 子命令和别名；
- 缺参数和非法值；
- 临时目录中的真实清理、图片、归档行为；
- 退出码和 stdout/stderr。

#### 发布冒烟测试

由 `scripts/package-smoke-test.ps1` 执行，检查：

- `pkg/release` 只有预期的三个 exe；
- `maya.exe --help` 和 `--version` 可执行；
- 五个子命令都出现在帮助中；
- FFmpeg/FFprobe 可执行且哈希匹配；
- NPM dry-run 只包含预期文件；
- Cargo 与 NPM 版本一致。

### 9.2 推荐验证命令

日常修改后：

```powershell
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

提交前：

```powershell
cargo fmt --all -- --check
cargo make verify
git diff --check
```

只运行某个 crate：

```powershell
cargo test -p maya-core
cargo test -p maya-fs
cargo test -p maya-git
cargo test -p maya-media
```

只运行一个测试并显示输出：

```powershell
cargo test -p maya-media verifies_expected_hash -- --nocapture
```

### 9.3 测试失败路径的思路

不要只测试“正常图片能压缩”。更有价值的测试包括：

- 路径不存在或不是目录；
- 文件在扫描过程中消失；
- 图片损坏；
- 压缩结果没有变小；
- 写入中途失败，原文件必须保持不变；
- ZIP 源文件在规划后消失；
- Git diff 返回 1、128 等不同退出码；
- FFmpeg 非零退出但留下部分输出；
- 播放列表为空或引用不存在分片；
- 超时后旧视频目录仍存在；
- sidecar 缺失、哈希错误或清单大小写不同。

测试应尽量使用 `tempfile::tempdir()`，测试结束后自动清理，不要依赖开发者本机真实项目目录。

---

## 10. 构建、打包与发布

### 10.1 Cargo 直接构建

开发构建：

```powershell
cargo build
```

Release 构建：

```powershell
cargo build --release
```

根 `build.rs` 只有在 `PROFILE=release` 且目标操作系统是 Windows 时部署 sidecar。它会：

1. 读取 `FFmpeg/checksums.sha256`；
2. 检查清单恰好包含一个 `ffmpeg.exe` 和一个 `ffprobe.exe`；
3. 校验仓库 `FFmpeg/` 下的源文件；
4. 根据 Cargo 的 `OUT_DIR` 推导本次 Release 产物目录；
5. 复制到 `maya.exe` 同级目录；
6. 复制后再次校验。

哈希比较使用 ASCII 大小写不敏感匹配，因此清单写大写或小写都可以。

### 10.2 cargo-make 任务关系

`Makefile.toml` 中最重要的任务：

| 任务 | 作用 |
| --- | --- |
| `fmt-check` | 检查 Rust 格式 |
| `clippy` | workspace 全目标严格 Clippy |
| `test` | workspace 全部测试 |
| `release-build` | 调用 `scripts/build-release.ps1` |
| `release-config-check` | 校验版本、publish 配置和 FFmpeg 清单 |
| `verify` | 以上质量检查和 Release 构建的组合 |
| `sync-package-version` | 将 Cargo 版本同步到 `pkg/package.json` |
| `assemble-package` | 组装 NPM release 目录 |
| `package-smoke-test` | 对组装目录执行冒烟测试 |
| `package` | 完整打包链路，不发布 npm |
| `publish` | 显式触发 npm 发布 |

常用命令：

```powershell
cargo make verify
cargo make package
cargo make publish       # 会真正执行 npm publish，请确认后再运行
```

### 10.3 自定义 Cargo 目标目录和 target triple

发布脚本不会猜测固定的默认产物路径，而是读取 Cargo 的 JSON 编译消息，找到本次真正生成的 executable，并记录到被忽略的：

```text
target/maya-release-artifact.json
```

因此以下组合可以工作：

```powershell
$env:CARGO_TARGET_DIR = 'D:\build\maya-target'
$env:CARGO_BUILD_TARGET = 'x86_64-pc-windows-msvc'
cargo make package
```

组装脚本会验证记录中的路径仍存在、哈希未变化、`maya --version` 与 workspace 版本一致，然后从这个实际二进制的同目录复制两个 sidecar。这样可以避免自定义目标目录下构建了新二进制，却误把默认 target/release 中的旧文件打进包。

完成测试后如需清除变量：

```powershell
Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
Remove-Item Env:CARGO_BUILD_TARGET -ErrorAction SilentlyContinue
```

### 10.4 NPM 包最终包含什么

`pkg/package.json` 的 `files` 只包含 `release`，冒烟测试要求最终包恰好有：

```text
release/maya.exe
release/ffmpeg.exe
release/ffprobe.exe
README.md
package.json
```

不要把 `target/`、源码、Cargo 文档或临时构建记录打入 NPM 包。

### 10.5 更新 FFmpeg 的流程

如果要替换 FFmpeg 版本：

1. 下载并确认 Windows x86_64 版本的 `ffmpeg.exe`、`ffprobe.exe` 来源；
2. 替换 `FFmpeg/` 中的两个文件；
3. 重新计算 SHA-256 并更新 `FFmpeg/checksums.sha256`；
4. 更新 `FFmpeg/README.md` 的上游版本说明；
5. 执行 `cargo make verify`，确认 Release 构建部署成功；
6. 执行 `cargo make package`，确认 NPM dry-run 和 sidecar 校验通过。

不要只替换一个文件，也不要在未更新清单时提交二进制。清单是构建、运行时和发布三处共用的完整性契约。

---

## 11. 常见问题与排查顺序

### 11.1 `cargo` 或 `rustc` 找不到

检查 Rustup 是否安装、PATH 是否刷新：

```powershell
rustup show
Get-Command cargo
```

重新打开终端通常可以刷新 PATH。不要通过手动复制 `rustc.exe` 解决依赖问题。

### 11.2 找不到 `link.exe`

安装 Visual Studio Build Tools 的“使用 C++ 的桌面开发”工作负载，确认使用 MSVC 工具链：

```powershell
rustup default stable-x86_64-pc-windows-msvc
```

### 11.3 `cargo run -- transform` 找不到 FFmpeg

Debug 构建不会自动把仓库 `FFmpeg/` 复制到 `target/debug`。使用 Release：

```powershell
cargo build --release
target\release\maya.exe transform . --types mp4 m3u8
```

同时确认 `target\release` 中存在三个 exe：

```powershell
Get-ChildItem target\release\maya.exe, target\release\ffmpeg.exe, target\release\ffprobe.exe
```

### 11.4 FFmpeg 完整性校验失败

按顺序检查：

1. `ffmpeg.exe` 和 `ffprobe.exe` 是否与 `maya.exe` 同目录；
2. 是否误复制了其他版本的二进制；
3. `FFmpeg/checksums.sha256` 是否是对应版本；
4. 是否在复制后修改或覆盖了 sidecar；
5. 运行 `powershell -File scripts/verify-release-config.ps1`。

哈希大小写不是问题，运行时和脚本都不区分 ASCII 大小写。

### 11.5 Vite 打包提示动态配置不支持

项目无法安全执行 JavaScript 配置来推断动态值。显式指定输出目录：

```powershell
maya pack C:\project --type a --out-dir dist-prod
```

如果输出目录仍不存在，会返回路径错误；先确认前端构建已经完成。

### 11.6 Git 命令返回退出码 5

退出码 5 表示外部命令失败，不等于“没有变更”。检查错误信息中的：

- 工作目录是否是 Git 仓库；
- Git 用户名、邮箱和远端是否配置；
- 是否有未解决冲突；
- 是否有权限 push；
- stderr 尾部是否提示 hook 或认证失败。

### 11.7 `cargo make package` 失败

从第一条失败的任务开始处理，不要只看最后的总失败信息：

1. `fmt-check`：运行 `cargo fmt --all`；
2. `clippy`：修复所有警告；
3. `test`：单独运行失败 crate 的测试；
4. `release-build`：检查 MSVC、sidecar 和自定义 target 变量；
5. `release-config-check`：检查版本和清单；
6. `assemble-package`：检查 artifact record 和实际 exe；
7. `package-smoke-test`：检查包内容、版本和 NPM dry-run。

### 11.8 版本不一致

版本唯一源是根 `Cargo.toml` 的 `[workspace.package] version`。修改版本后执行：

```powershell
powershell -File scripts/sync-package-version.ps1
```

不要直接只改 `pkg/package.json`。完整的 `cargo make package` 也会自动同步并验证。

---

## 12. 提交代码前检查清单

### 12.1 代码检查

- [ ] 修改放在正确的 crate 或层级，没有把业务逻辑塞进 CLI dispatcher。
- [ ] 所有可恢复错误都通过 `maya_core::Error` 返回。
- [ ] 没有新增无理由的 `unwrap()`、`expect()`、`filter_map(|x| x.ok())` 或 `flatten()`。
- [ ] 所有破坏性写入都有临时文件/目录和失败恢复语义。
- [ ] 外部进程检查真实退出码，并限制 stderr/日志大小。
- [ ] 公共 API 接收显式 `&Path`，不依赖隐式当前目录。
- [ ] 新增字段、枚举分支和错误类型都有默认值、验证和文档。

### 12.2 测试检查

- [ ] 成功路径测试通过。
- [ ] 空输入、非法输入、路径不存在测试通过。
- [ ] 中途失败不会破坏旧文件或旧目录。
- [ ] 批量部分失败的报告和退出码正确。
- [ ] CLI help、参数错误和真实临时目录集成测试已补充。
- [ ] `cargo fmt --all -- --check` 通过。
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` 通过。
- [ ] `cargo test --workspace` 通过。

### 12.3 发布检查

- [ ] `cargo make verify` 通过。
- [ ] Release `maya.exe` 与两个 sidecar 位于同一目录。
- [ ] sidecar SHA-256 与清单一致。
- [ ] `cargo make package` 通过。
- [ ] NPM 包中没有多余文件。
- [ ] 未意外执行 `npm publish`。
- [ ] 文档、README 和命令示例与实际 CLI 一致。

---

## 13. 术语表与延伸阅读

### 13.1 术语表

| 术语 | 含义 |
| --- | --- |
| crate | Rust 的一个编译单元，可以是库或二进制 |
| workspace | 多个 crate 的统一项目和依赖管理方式 |
| sidecar | 主程序旁边随包分发、由主程序调用的外部二进制；本项目是 FFmpeg/FFprobe |
| report | 一次批处理的汇总结果，包含数量、大小、warning 等 |
| outcome | 单个输入项的结果，例如 Compressed、Skipped、Failed |
| progress sink | 接收业务进度事件的 trait 实现 |
| 原子写入 | 先写临时文件，成功后一次性替换目标，避免半成品 |
| HLS | HTTP Live Streaming；本项目输出 `index.m3u8` 和媒体分片 |
| Cargo target 目录 | Cargo 存放编译中间文件和最终二进制的目录；可由 `CARGO_TARGET_DIR` 改变 |
| NPM dry-run | 只检查将要打包的文件，不上传到 npm |

### 13.2 仓库内相关文档

- [项目文件结构设计](../docs/001、项目文件结构设计.md)
- [项目优化方案](../docs/002、项目优化方案.md)
- [代码深度优化方案](../docs/003、代码深度优化方案.md)
- [最新依赖与最佳实践优化方案](../docs/004、最新依赖与最佳实践优化方案.md)
- [Rust 架构审查与演进方案](../docs/005、Rust架构审查与演进方案.md)
- [FFmpeg 分发说明](../FFmpeg/README.md)
- [面向用户的 README](../README.md)

### 13.3 推荐 Rust 学习顺序

如果你刚开始学 Rust，可以按这个顺序补基础：

1. 变量、函数、结构体、枚举和 `match`；
2. 所有权、借用、生命周期和 `Path`/`PathBuf`；
3. `Result`、`Option`、错误传播和 `thiserror`；
4. trait、泛型和依赖注入；
5. 模块、crate、Cargo workspace 和 feature；
6. 迭代器、闭包、并行迭代和 Rayon；
7. Tokio async、`spawn_blocking`、`Arc` 和 `Mutex`；
8. 测试、Clippy、格式化和发布自动化。

学习时不要试图一次读懂所有实现。建议先运行一条命令，再沿着“CLI → module → crate → report → presenter”的链路逐层跟踪；每次只弄清一层的输入、输出和错误边界。
