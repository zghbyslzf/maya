# Maya Rust 架构审查与演进方案

> 审查日期：2026-08-31  
> 审查范围：根 CLI、8 个 workspace crate、公共错误与文件工具、测试、Cargo/NPM 发布流程  
> 目标：提高项目的可理解性、可扩展性、复用性、鲁棒性和可测试性，同时避免与当前规模不匹配的过度设计

## 1. 总体结论

当前项目**有明确的架构优化空间**，但不需要推倒重写。

项目已经形成“根 CLI 负责分发、各 crate 承担具体能力、`maya_common` 提供公共功能”的基本结构；`cargo test` 和严格 Clippy 也能通过。这些是良好的演进基础。当前的主要问题不是 Rust 语法或性能，而是几个边界没有被清晰表达：

1. 文件系统和外部进程的失败可能被静默忽略或误判为成功；
2. 业务库同时负责执行、打印和进度条，难以复用和测试；
3. CLI 参数、业务结果和错误仍大量使用字符串、布尔值或裸元组表达；
4. crate 按命令名称拆得较细，而公共 crate 的职责又过宽；
5. 发布链路缺少事务性与统一版本源，Cargo 与 NPM 包存在漂移风险。

建议先修复会造成数据损坏、漏处理或错误退出码的鲁棒性问题，再逐步明确 CLI、用例和基础设施边界，最后整理 workspace 与发布流程。整个过程应保持现有 CLI 行为尽可能兼容。

## 2. 当前优势与基线

### 2.1 已有优势

- 根程序通过 `src/modules/*.rs` 分发命令，业务逻辑大部分已经下沉到 crate；
- workspace 依赖已有统一管理意识；
- `maya_common` 提供统一的 `Error` 和 `Result<T>`，避免了完全割裂的错误体系；
- 图片压缩已经使用并行处理，视频转换封装了 FFmpeg 自动获取；
- 已有单元测试和 CLI 集成测试，具备继续补充回归测试的基础；
- release profile 已针对二进制体积优化。

### 2.2 本次验证基线

| 检查项 | 结果 | 说明 |
| --- | --- | --- |
| `cargo test --workspace` | 通过 | 当前共 29 个测试通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 通过 | 当前无 Clippy 警告 |
| `cargo fmt --all -- --check` | 未通过 | 存在已有格式差异，应纳入后续质量门禁 |
| `cargo package -p maya --allow-dirty --no-verify --registry crates-io` | 未通过 | 内部 crate 无法从 registry 解析，根包元数据也不完整 |
| `npm pack --dry-run --json` | 通过 | 压缩包约 66 MB，解压约 179 MB，主要来自 FFmpeg/FFprobe |

P0 完成后的验证基线：`cargo test --workspace` 共 47 个测试通过，严格 Clippy 与 `cargo build --release --locked` 均通过；本次修改涉及的 Rust 文件已通过 `rustfmt --check`。全仓库 `cargo fmt --all -- --check` 仍受 P0 范围外的历史格式差异影响，留待阶段 3 统一清理。

这里的“测试通过”只代表已有测试覆盖的行为正确，不代表关键失败路径已经得到验证。后文列出的嵌套目录删除问题已可在当前实现中稳定复现。

## 3. 架构诊断总览

| 优先级 | 维度 | 现状 | 主要风险 | 建议方向 |
| --- | --- | --- | --- | --- |
| P0 | 文件遍历 | 多处使用 `filter_map(...ok())` 或 `flatten()` | 权限、路径和 I/O 错误被静默忽略，命令可能漏处理却报告成功 | 明确严格/尽力而为策略并返回警告 |
| P0 | 删除逻辑 | 先收集全部 `node_modules` 再逐个删除 | 父目录先删后，子目录删除报 `NotFound` | 遍历时剪枝或按深度处理，补回归测试 |
| P0 | 写入安全 | 图片、ZIP、视频输出直接覆盖或预先删除 | 中途失败可能破坏原文件或留下半成品 | 临时同级文件/目录写入，成功后原子替换 |
| P0 | 外部进程 | FFmpeg 主要依赖日志 EOF 和输出文件存在判断 | 非零退出、旧文件或部分输出可能被误判成功 | 检查退出状态并验证完整产物 |
| P1 | 职责边界 | 业务 crate 内直接打印和创建进度条 | 难复用、难测试、难增加 JSON/quiet 模式 | 业务层返回报告/事件，CLI 统一展示 |
| P1 | 类型建模 | 参数使用字符串，结果使用裸元组或布尔值 | 合法组合不清晰，扩展需要多点同步 | `ValueEnum`、Options、Report、Outcome |
| P1 | 错误模型 | 多数领域错误仅保存 `String` | 丢失操作、路径、退出码及错误来源 | 使用带上下文的结构化错误 |
| P1 | 隐式环境 | 部分打包 API 内部读取 `current_dir()` | 无法显式控制作用域，测试和复用困难 | 所有业务入口接收 `&Path` |
| P1 | 失败语义 | 批处理部分失败或找不到目标时仍可能返回成功 | 自动化脚本无法可靠判断结果 | 统一失败策略、报告和退出码 |
| P2 | workspace 边界 | 小 crate 较多，`maya_common` 职责较杂 | 新增命令改动面大，公共模块持续膨胀 | 按稳定能力收敛为 core/fs/media/git |
| P2 | 测试设计 | 正常路径较多，失败和中断路径不足 | 鲁棒性重构缺少安全网 | 文件系统、假进程、CLI、发布分层测试 |
| P2 | 发布流程 | 构建前改版本、Cargo/NPM 分别递增 | 构建失败后版本脏化，版本长期漂移 | 单一版本源、先验证后发布、失败可恢复 |

## 4. P0：先处理数据安全与真实成功语义（已完成）

> 完成状态：**已完成（2026-08-31）**
> 实施摘要：文件遍历改为严格错误传播；`node_modules` 搜索命中后剪枝；图片与 ZIP 使用同目录临时文件提交；视频使用同级暂存目录和旧目录回滚；FFmpeg 增加超时、真实退出状态、stderr 尾部及 HLS 产物校验。相关失败路径均已补充回归测试。

### 4.1 文件遍历不能静默吞错（已完成）

**现状**

以下位置会将 `walkdir` 返回的错误直接丢弃：

- `crates/maya_common/src/file_utils.rs` 中多处 `filter_map(|e| e.ok())`；
- `crates/gitignore_add_zip/src/lib.rs` 中的 `walker.flatten()`。

权限不足、目录在遍历中被删除、路径过长等错误不会出现在最终结果里。因此，“成功处理 100 个文件”可能实际只是“看到了 100 个文件”，调用者无法知道还有多少文件未被扫描。

**建议**

为扫描行为定义明确策略，而不是通过迭代器组合隐式决定：

```rust
pub enum ScanPolicy {
    Strict,
    BestEffort,
}

pub struct ScanReport<T> {
    pub entries: Vec<T>,
    pub warnings: Vec<ScanWarning>,
}
```

- `Strict`：遇到遍历错误立即返回带根路径和具体路径的错误；
- `BestEffort`：继续处理，但必须把错误收集为 warning，并影响最终摘要或退出码；
- 删除、覆盖、归档等破坏性操作默认使用 `Strict`；
- 纯查询类命令才考虑显式选择 `BestEffort`。

**验收标准**

- [x] 代码中不再以 `ok()`/`flatten()` 静默丢弃遍历错误；
- [x] 遍历错误保留具体路径和底层错误，并有目录在遍历中消失的回归测试；
- [x] 当前所有破坏性扫描统一采用严格模式，失败直接返回非零结果；`BestEffort` warning/CLI 展示属于 P1 的部分失败报告能力，不在本次 P0 引入。

### 4.2 修复嵌套 `node_modules` 删除错误（已完成）

**现状**

`clear_node_modules` 先用 `find_by_name` 收集所有同名目录，再按遍历顺序删除。对于以下结构：

```text
node_modules/
└── dep/
    └── node_modules/
```

父目录通常先被删除，之后再次删除已随父目录消失的子目录，会返回 `NotFound`。当前行为已实测复现：先打印根 `node_modules` 删除成功，随后整个命令失败。

**建议**

优先使用 `WalkDir::filter_entry`：发现目标目录后记录并阻止继续进入该目录。这样既避免重复目标，也减少无意义扫描。备选方案是按深度降序删除，但仍需处理目录在执行期间变化的情况。

**验收标准**

- [x] 嵌套 `node_modules` 只生成一个有效删除目标；
- [x] 删除结果只报告实际删除数量；
- [x] 同时存在多个同级 `node_modules` 时全部删除；
- [x] 已补充嵌套目录、同级目录和目标在删除前消失的回归测试。

### 4.3 所有破坏性写入改为临时产物后替换（已完成）

**现状**

- 图片覆盖模式直接对原路径执行 `File::create`，编码或写入中途失败会截断原图片；
- ZIP 直接创建最终文件，失败时会留下损坏或不完整归档；
- 视频转换在开始前删除已有输出目录，转换失败后旧结果不可恢复。

**建议**

在 `maya-fs` 或现有公共模块中提供统一的原子写入能力：

1. 在目标同级目录创建唯一临时文件或目录；
2. 完成全部写入和必要的 `flush`/`sync`；
3. 验证临时产物；
4. 成功后再替换最终目标；
5. 失败时清理临时产物并保留旧目标。

同级临时文件很重要，因为跨文件系统 `rename` 不能保证原子性。Windows 上替换已存在文件需封装为平台明确的实现和测试，不能假设所有平台的 `rename` 语义一致。

**验收标准**

- [x] 模拟图片编码、ZIP 写入或 FFmpeg 中途失败时，原产物保持不变；
- [x] 成功或失败后均不遗留 `.maya-tmp-*`；
- [x] 图片和 ZIP 共用 `atomic_write`，视频共用同模块中的目录暂存、提交和回滚能力。

### 4.4 以进程退出状态和产物契约判断 FFmpeg 成功（已完成）

**现状**

`mp4_to_m3u8` 收到 `FfmpegEvent::LogEOF` 后把进度设为 100%，随后主要检查 `index.m3u8` 是否存在。日志结束不等于进程成功；旧文件、空 playlist 或只生成部分 segment 都可能造成误判。

**建议**

- 等待子进程结束并要求退出状态为成功；
- 非零退出时保留退出码和 stderr 尾部，避免错误信息过大；
- 再验证 `index.m3u8` 非空、可读取且引用的基本 segment 已存在；
- 输出始终先写临时目录，通过验证后再提交；
- 为转换增加可配置超时和取消策略；
- 如果 sidecar 的迭代接口是阻塞式，将它放入 `spawn_blocking`，或把转换用例整体设计为同步，仅把下载保持异步。

**验收标准**

- [x] 假 FFmpeg 返回非零退出码但创建了 playlist 时，命令仍判定失败；
- [x] 退出码为 0 但 playlist 为空、未引用分片或分片缺失/为空时，命令判定失败；
- [x] 错误包含命令、工作目录、退出码和最大 8 KiB 的 stderr 尾部；
- [x] FFmpeg 阻塞工作已移入 `spawn_blocking`，每个转换默认有 2 小时超时并可通过库 API 配置；
- [x] 非零退出、超时或产物不完整都不会删除旧的成功产物。

## 5. P1：让边界、输入、输出和失败可被理解（已完成）

**完成摘要（2026-08-31）**

- 根 CLI 已迁移到 Clap `ValueEnum` 和参数结构体，保留 `n/m/g/a` 等旧值别名，并新增 `--new-file`、`--jpeg-quality`、`--failure-policy`、`--quiet`、`--no-progress`、Pack 路径和 Vite `--out-dir`；
- 清理、归档、图片和视频均返回类型化 report/outcome，视频降级信息通过结构化 warning 返回；长任务通过 `ProgressSink` 上报，由 CLI presenter 统一展示；
- 所有业务入口显式接收路径，业务 crate 已移除终端输出、具体进度条和全局当前目录读取；
- 新增带操作/路径的 I/O 错误、外部命令错误、配置边界错误、部分失败错误和稳定退出码；参数错误为 2、部分失败为 3、路径/配置错误为 4、外部命令错误为 5；
- Vite 打包明确只支持静态字符串 `build.outDir`，动态配置会被拒绝并可用 `--out-dir` 覆盖；Git 使用暂存区 diff 的退出状态判断是否有变更，不再读取英文错误文案；
- 已通过 `cargo test --workspace`（63 个测试）、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo build --release --locked` 和 `git diff --check`；业务 crate 副作用扫描无命中。

### 5.1 业务库不直接负责终端展示（已完成）

**现状**

清理、压缩、视频和 Git 等 crate 中存在大量 `println!`、`eprintln!` 和 `ProgressBar`。这使业务函数同时承担：执行操作、汇总状态、决定文案、选择终端交互方式。

直接后果包括：

- 作为库调用时无法安静运行；
- 单元测试需要处理全局 stdout；
- 难以增加 `--quiet`、`--json`、`--no-progress`；
- 部分失败既被打印又被转成弱上下文错误，调用者难以二次处理。

**建议**

业务用例返回类型化报告；CLI presenter 是唯一负责用户输出的层。长任务通过事件回调或很小的 `ProgressSink` 边界上报进度，不让领域层依赖具体进度条实现。

```rust
pub struct OperationReport<S> {
    pub summary: S,
    pub items: Vec<ItemOutcome>,
    pub warnings: Vec<OperationWarning>,
}
```

不要一开始引入事件总线。只有图片/视频这类确实需要流式进度的任务才使用回调或 trait，短任务直接返回报告即可。

### 5.2 使用类型表达命令参数和领域选项（已完成）

**现状**

根 CLI 使用 `Vec<String>` 或 `String` 接收 `types`、`ops`、`pack_type`，dispatcher 再手工匹配 `"n"`、`"lock"`、`"m"`、`"g"`、`"a"` 等协议。新增选项时需要同步修改帮助文本和多个匹配分支，非法组合只能在运行期发现。

“`n` 表示输出新文件”还与图片类型混在同一个列表中，属于两个不同维度。

**建议**

利用 Clap 的 `ValueEnum` 和参数结构体：

```rust
enum CleanTarget { NodeModules, LockFiles }
enum PackMode { Gitignore, Vite }
enum ImageFormat { Png, Jpeg, All }
enum OutputMode { Overwrite, NewFile }
```

- 短别名通过 Clap alias 保持兼容；
- `--new-file` 或 `--output-mode` 独立表达输出策略；
- 互斥、必填和默认值尽量在参数解析阶段验证；
- JPEG/JPG 作为同一格式的两个输入别名，不在领域层保留两个等价枚举分支。

### 5.3 用 Options 和 Report 替代长参数、布尔值和裸元组（已完成）

**现状**

- 图片压缩返回 `(u32, u32, f64)`；
- 视频转换返回 `(u32, u32)`；
- 覆盖/新文件等行为使用布尔值表达；
- 压缩质量等关键策略硬编码在实现内。

调用点必须记住每个位置的含义，后续增加 skipped、字节数、失败原因会破坏函数签名。

**建议**

```rust
pub struct CompressionOptions {
    pub formats: Vec<ImageFormat>,
    pub output: OutputMode,
    pub jpeg_quality: u8,
    pub failure_policy: FailurePolicy,
}

pub struct CompressionReport {
    pub scanned: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub bytes_before: u64,
    pub bytes_after: u64,
    pub items: Vec<CompressionOutcome>,
}
```

视频、清理和归档采用同一设计风格，但不必强制共用所有字段。统一的是“Options 输入、Report 输出”的约定，而不是制造一个容纳所有业务的巨型结构体。

### 5.4 错误必须保留操作上下文和机器可判定信息（已完成）

**现状**

`maya_common::Error` 中多个变体只保存 `String`，例如 `Path(String)`、`Compression(String)`、`VideoConversion(String)`、`Git(String)`。`Io(#[from] std::io::Error)` 虽保留 source，却不知道在读、写或删除哪个路径。Git 命令失败也没有稳定保存参数、cwd、退出码和 stderr。

**建议**

按外部边界建立结构化错误：

```rust
Io {
    operation: IoOperation,
    path: PathBuf,
    source: std::io::Error,
}

CommandFailed {
    program: String,
    args: Vec<String>,
    cwd: PathBuf,
    status: Option<i32>,
    stderr: String,
}
```

- 领域错误保留稳定变体，展示文案只在 CLI 顶层生成；
- 对 stderr 设置长度上限，但保留原始 cause chain；
- 定义一致退出语义，例如参数错误、输入错误、外部工具失败、部分失败使用稳定退出码；
- 避免库先打印一次，CLI 再打印一次。

### 5.5 所有业务 API 显式接收根路径（已完成）

**现状**

Gitignore 和 Vite 打包在库内部调用 `current_dir()`，而清理、Git、图片和视频命令又接收路径，API 风格不一致。隐式当前目录还会受测试进程的全局状态影响。

**建议**

- 所有用例入口接收 `root: &Path` 或具体输入路径；
- CLI 的 Pack 子命令增加默认值为 `.` 的路径参数；
- 只允许 CLI 入口读取当前目录，领域和基础设施代码不读取进程全局 cwd；
- 路径规范化和“目标是否允许位于根目录外”等策略集中处理。

### 5.6 明确 Vite 打包能力的支持边界（已完成）

**现状**

Vite 配置查找/解析使用 `Option` 和正则，将“配置不存在、读取失败、无法解析”折叠为同一结果。JS/TS 配置实际上可以包含函数、环境分支、插件等动态逻辑，正则无法可靠模拟 Vite。当前找不到配置或输出目录时还可能打印提示后返回 `Ok(())`。

**建议**

```rust
fn pack_vite(
    project_root: &Path,
    options: &VitePackOptions,
) -> Result<ArchiveReport, VitePackError>;
```

- 区分 `ConfigNotFound`、`ConfigUnreadable`、`UnsupportedDynamicConfig`、`OutputDirNotFound`；
- 明确只支持静态字符串形式的 `build.outDir`，不要宣称能够完整解析 JS/TS；
- 提供 `--out-dir` 作为可靠覆盖方式；
- 找不到必需输入应返回非零退出，而不是仅打印消息；
- 删除解析分支中不必要的 `unwrap()`。

### 5.7 不依赖 Git 的英文错误文案（已完成）

**现状**

Git 提交逻辑通过 stderr 是否包含 `"nothing to commit"` 判断无变更。Git 的本地化输出、版本变化或不同执行环境都会破坏该判断。

**建议**

- `git add` 后使用 `git diff --cached --quiet --exit-code` 判断暂存区是否为空；
- 将结果建模为 `GitOutcome::CommittedAndPushed` 或 `GitOutcome::NothingToCommit`；
- 通过小型 `ProcessRunner` 边界封装外部命令，便于用假程序或本地临时仓库测试；
- 不要用通用 shell 字符串拼接 Git 命令，始终以参数数组执行。

### 5.8 统一批处理的部分失败策略（已完成）

**现状**

图片或视频逐项执行时，部分失败有时只计数后继续；Vite 找不到目标也可能返回成功。用户看到错误文本，但脚本拿到的退出码仍可能是 0。

**建议**

定义显式策略：

```rust
pub enum FailurePolicy {
    FailFast,
    Continue,
}
```

`Continue` 不等于忽略失败：它应完成其余项目、返回完整报告，并由 CLI 根据 failed 数量输出非零退出码。后续可在此基础上增加 `--json`，让 CI 能读取每个项目的结果。

## 6. P2：收敛 workspace，而不是继续增加微型 crate（已完成）

**完成摘要（2026-08-31）**

- workspace 已从 8 个命令型 crate 收敛为 4 个稳定能力 crate：`maya-core`、`maya-fs`、`maya-media`、`maya-git`；根 CLI 的命令、参数、别名、输出与退出语义保持兼容；
- `maya-core` 只保留错误、执行策略、进度边界、报告和 warning 等稳定领域类型，直接依赖仅有 `thiserror`，不再通过 feature 引入 Tokio、Anyhow 或 Rayon；
- 扫描、原子文件/目录替换、清理、Gitignore/Vite 归档已统一进入 `maya-fs`；Git 工作流与 `ProcessRunner` 进入 `maya-git`；图片与视频能力统一进入 `maya-media`；
- 媒体内部已按职责拆分：图片采用 `model / codec / orchestration`，视频采用 `model / probe / ffmpeg / orchestration`，外部仍通过稳定的 `image`、`video` 模块调用；
- 已删除 `maya_common`、6 个命令型微型 crate，以及未使用的并行扫描、通用名称扫描、空目录删除、可选 feature 和 Anyhow 转换；
- `cargo tree` 与 `cargo metadata` 已确认依赖方向为 `CLI → media/git/fs → core`，其中 `media → fs → core`，不存在反向依赖或旧 workspace 成员；
- 已通过 `cargo test --workspace`（57 个测试）、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo build --release --locked`、`cargo fmt --all` 和 `git diff --check`。

### 6.1 当前边界问题（已完成）

当前 8 个 crate 多数按命令名拆分，其中 `clear_node_modules`、`clear_lock`、`gitignore_add_zip`、`git_add_commit_push` 的实现较小。每增加一个相似命令，通常要同步增加 crate、manifest、workspace dependency、根依赖和 dispatcher。

另一边，`maya_common` 同时容纳错误、扫描、查找、删除空目录和 ZIP 等能力，名称无法说明依赖方向，容易逐渐成为“什么都能放”的公共 crate。当前还存在未被实际调用的公共 API/feature，例如并行查找、空目录删除以及部分错误转换。

### 6.2 建议目标结构（已完成）

```text
maya-cli
├── typed clap args
├── presenter（human / quiet / json / progress）
└── use cases
    ├── maya-core
    │   └── options / report / policy / domain error
    ├── maya-fs
    │   └── scan / clean / archive / atomic_io
    ├── maya-media
    │   └── image / video / ffmpeg
    └── maya-git（仅在 Git 能力继续增长时独立）
        └── repository operation / process boundary
```

依赖方向保持单向：

```text
CLI 展示层  →  用例/领域层  →  文件系统与外部进程边界
```

领域层不直接打印、不读取 `current_dir()`，基础设施层不决定面向用户的文案。

### 6.3 渐进迁移原则（已完成）

- 先调整 API 和测试，再移动文件或合并 crate；
- 合并过程中保留现有 CLI 命令和别名；
- `maya_common` 中真正通用的类型迁移到 `maya-core`，文件能力迁移到 `maya-fs`；
- 删除未使用 API，或先降为 crate 私有，避免无意形成长期兼容承诺；
- `compress_pictures` 可按 `options/scanner/png/jpeg/report` 拆内部模块；
- `mp4_to_m3u8` 可按 `ffmpeg/probe/converter/progress` 拆内部模块；
- 模块化优先于创建新 crate：只有需要独立依赖、独立测试边界或明确复用时才拆 crate。

## 7. 文件扫描、归档与压缩的复用优化

### 7.1 避免重复扫描和 TOCTOU

图片命令在 dispatcher 中先扫描统计，压缩 crate 内又扫描一次；Gitignore 打包先构建完整 `HashSet<PathBuf>`，归档函数再遍历一次。除了性能浪费，文件可能在两次扫描之间发生变化，造成统计和实际操作不一致。

建议让业务用例拥有一次完整扫描，并把 entry stream 直接交给执行器；最终由 Report 提供 scanned/succeeded/skipped 数据。归档构建器也应接收 entry iterator 或明确的归档计划，避免再次遍历。

### 7.2 归档 API 应表达策略与结果

当前 ZIP 实现还存在以下隐患：

- 非 UTF-8 路径在 `to_str()` 失败后可能被跳过；
- `strip_prefix` 失败未形成明确错误；
- 只按文件名比较输出 ZIP，可能误跳过嵌套同名文件；
- 目标 ZIP 直接覆盖；
- 空目录及 metadata 的保留策略不明确。

建议引入 `ArchiveOptions` 和 `ArchiveReport`，明确路径编码、符号链接、空目录、metadata、忽略规则及失败策略。排除输出文件时应比较规范化后的完整路径，而不是仅比较文件名。

### 7.3 进度必须来自真实工作量

FFmpeg 下载目前使用固定时间推进到 100% 的模拟进度。网络慢时进度会在 100% 停留，网络快时又会延迟完成，反而降低可信度。

- 无法获得总字节数时使用 spinner；
- 能获得 Content-Length 和下载字节时才使用确定进度；
- 进度显示属于 CLI presenter，不改变下载或转换结果。

## 8. 测试与可观测性方案

### 8.1 建议的分层测试

| 层级 | 关注点 | 代表用例 |
| --- | --- | --- |
| 纯逻辑单元测试 | 参数归一化、Vite 静态配置解析、报告汇总、退出码映射 | JPG/JPEG alias、失败策略、动态配置拒绝 |
| 文件系统组件测试 | 使用 `tempfile` 验证真实目录和文件行为 | 嵌套 `node_modules`、原子替换、ZIP 写失败、目标消失 |
| 外部进程契约测试 | 假可执行文件或本地临时仓库 | FFmpeg 非零退出/部分输出/stderr，Git 无提交/提交失败 |
| CLI 集成测试 | 参数、输出模式和进程退出码 | 非法组合、部分失败、`--quiet`、未来的 `--json` |
| 发布冒烟测试 | 构建产物是否可执行、文档是否匹配 | `maya --help`、典型子命令、NPM 包内容 |

优先补充以下回归测试：

1. 嵌套及同级 `node_modules`；
2. 遍历错误不被吞掉；
3. 图片覆盖中途失败保留原文件；
4. ZIP 中途失败不替换旧归档；
5. FFmpeg 非零退出但已产生部分文件；
6. Vite 配置不存在、不可读、动态表达式和特殊路径；
7. Git 使用本地 fake remote 或假 runner，不访问公网；
8. 批处理中部分失败时的报告与退出码。

### 8.2 最小质量门禁

项目目前没有 CI/CD。无论使用 CI 还是 `cargo-make`，都应先提供一条本地可重复的 verify 任务：

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release --locked
```

当前格式检查尚未通过，应先一次性整理历史格式，再启用门禁，避免把无关格式变化混入功能提交。

## 9. 发布流程与分发策略

### 9.1 单一版本源

当前根 `Cargo.toml` 为 `0.1.55`，`pkg/package.json` 为 `0.1.51`；构建流程在构建前修改 Cargo 版本，NPM 阶段又单独递增版本。构建失败也可能留下已修改文件。

建议：

- 以 workspace package version 作为唯一版本源；
- 成员使用 `version.workspace = true`；
- `package.json` 由发布脚本同步，不自行计算下一版本；
- 先完成 verify，再计算并一次性写入版本；
- 发布失败可回滚，或只允许从明确的 release commit/tag 发布。

### 9.2 明确 Cargo 发布意图

根包执行 `cargo package` 时，内部 crate 无法从 crates.io 解析，同时根包缺少 description、license、repository 等 package metadata。

如果项目只通过 NPM 分发：

- 根 binary 和内部 crate 均设置 `publish = false`；
- 发布流程不再把 `cargo package` 当作目标。

如果未来需要 crates.io：

- 为可发布 crate 完整填写 metadata；
- 明确内部 crate 的独立版本与发布顺序；
- 不能只依赖本地 path 解析。

### 9.3 统一 FFmpeg 分发策略

当前 NPM 包同时携带约 87 MB 的 `ffmpeg.exe` 和约 87 MB 的 `ffprobe.exe`，而运行时又能通过 `ffmpeg-sidecar` 自动下载，存在重复策略。应明确选择：

- **随包携带**：离线可用，但包很大；建议按平台和架构拆分可选包；
- **运行时下载**：包小，但需网络、缓存、校验、代理和下载失败处理。

两种策略都合理，但不应在没有明确优先级和回退规则时同时存在。若保留下载，应校验来源、版本和文件哈希，并明确缓存位置。

### 9.4 文档与 CLI 同步

`pkg/README.md` 仍包含旧的单字母 flag 示例，与当前子命令接口不一致。建议由根 README 的受控片段生成包内 README，或在发布检查中执行示例快照测试，避免两个文档长期漂移。

## 10. 分阶段实施路线

### 阶段 0：鲁棒性止血（已完成）

- [x] 修复嵌套 `node_modules`；
- [x] 消除文件遍历的静默错误；
- [x] 检查 FFmpeg 真实退出状态、超时和产物；
- [x] 为图片、ZIP、视频引入临时产物后替换；
- [x] 为上述问题补回归测试。

这一阶段尽量不改变公共 CLI，只修复“看似成功但实际失败”和“失败破坏旧数据”。

### 阶段 1：明确输入输出边界（已完成）

- [x] CLI 参数改为类型化枚举和 options；
- [x] 所有业务入口显式接收路径；
- [x] 用 typed report 替代裸元组；
- [x] 输出和进度条迁移到 CLI presenter；
- [x] 结构化错误、失败策略和退出码；
- [x] 增加 `--quiet`、`--no-progress`，JSON 可随后增量加入。

### 阶段 2：整理 workspace（已完成）

- [x] 将小型文件操作 crate 收敛到 `maya-fs`；
- [x] 建立轻量 `maya-core`，只保留稳定领域类型；
- [x] 图片与视频归入 `maya-media` 并统一 API 风格；
- [x] 删除未使用公共 API 和 feature；
- [x] 拆分大文件内部模块，保持外部命令兼容。

### 阶段 3：发布可靠性（预计 1～3 天）

- 增加统一 verify 任务；
- 修复格式基线并启用自动门禁；
- 使用单一版本源；
- 明确 `publish = false` 或补齐 crates.io 发布模型；
- 选择 FFmpeg 分发策略；
- 同步 NPM README，并增加 release smoke test。

## 11. 不建议引入的复杂度

当前项目源码规模约为数千行，最重要的是让副作用和结果可见，而不是套用大型企业架构。现阶段不建议：

- 为每个函数创建 trait 或 repository 接口；
- 引入依赖注入容器、全局 service locator 或事件总线；
- 为了“纯净架构”把每个命令拆成更多微型 crate；
- 在没有多实现或测试替身需求时抽象所有文件 API；
- 一次性重写整个 workspace。

trait 应优先用于真正的外部边界，例如 `ProcessRunner`、需要流式更新的 `ProgressSink`，以及只有在原子写入测试确有需要时的 `FileOps`。其余代码优先使用普通结构体、枚举和纯函数。

## 12. 最终验收清单

完成本方案的核心部分后，项目应满足：

- [x] 所有公共业务 API 都显式接收输入路径，不读取全局当前目录；
- [x] 业务 crate 不直接 `println!`/`eprintln!`，终端展示集中在 CLI；
- [x] CLI 参数使用类型表达，非法组合尽量在解析阶段拒绝；
- [x] 批处理返回 typed report，能区分成功、失败、跳过和警告；
- [x] 遍历错误不会被 `ok()` 或 `flatten()` 静默丢弃；
- [x] 所有覆盖式写入使用临时产物验证后替换；
- [x] FFmpeg 以退出状态和完整产物契约判断成功；
- [x] 嵌套 `node_modules` 回归测试通过；
- [x] Git 逻辑不依赖英文 stderr；
- [x] 部分失败有稳定的报告和非零退出语义；
- [ ] `fmt`、Clippy、测试和 release build 可通过一条 verify 命令完成；
- [ ] Cargo 与 NPM 版本一致且来自单一版本源；
- [ ] NPM 文档中的示例与实际 `maya --help` 一致；
- [ ] FFmpeg 采用一种清晰、有校验、有失败处理的分发策略。

## 13. 推荐决策顺序

若资源有限，建议严格按以下顺序投入：

1. **先保证不会漏处理、误报成功或破坏旧文件**；
2. **再用类型化参数、报告和错误把行为说清楚**；
3. **随后把展示从业务层移出，提高复用和测试能力**；
4. **等边界稳定后再合并 crate，避免先搬目录后改 API**；
5. **最后收敛发布和分发策略，形成持续质量门禁**。

这条路线能够以较小改动持续获得收益，也为后续增加新清理规则、新媒体格式、机器可读输出或其他打包方式留下稳定扩展点。
