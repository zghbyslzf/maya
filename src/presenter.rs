use maya_core::{ArchiveReport, ProgressEvent, ProgressSink, RemovalReport};
use maya_git::GitOutcome;
use maya_media::image::{CompressionOutcome, CompressionReport};
use maya_media::video::{ConversionOutcome, ConversionReport};
use std::sync::Mutex;

#[derive(Debug, Default)]
struct ProgressState {
    operation: String,
    completed: u64,
}

#[derive(Debug)]
pub struct Presenter {
    quiet: bool,
    no_progress: bool,
    progress: Mutex<ProgressState>,
}

impl Presenter {
    pub fn new(quiet: bool, no_progress: bool) -> Self {
        Self {
            quiet,
            no_progress,
            progress: Mutex::new(ProgressState::default()),
        }
    }

    pub fn removal(&self, label: &str, report: &RemovalReport) {
        if !self.quiet {
            println!("已清理 {} 个{}", report.removed_count(), label);
            if !report.skipped.is_empty() {
                println!("跳过 {} 个已不存在的目标", report.skipped.len());
            }
        }
    }

    pub fn archive(&self, report: &ArchiveReport) {
        if !self.quiet {
            println!(
                "已创建归档 {}（{} 个文件，{} 个空目录，{} 字节 -> {} 字节）",
                report.archive_path.display(),
                report.files_added,
                report.directories_added,
                report.source_bytes,
                report.archive_bytes
            );
        }
    }

    pub fn compression(&self, report: &CompressionReport) {
        if self.quiet {
            return;
        }
        for item in &report.items {
            if let CompressionOutcome::Failed { path, error } = item {
                eprintln!("压缩失败 {}: {error}", path.display());
            }
        }
        println!(
            "图片处理完成：扫描 {}，成功 {}，跳过 {}，失败 {}，压缩率 {:.1}%",
            report.scanned,
            report.succeeded,
            report.skipped,
            report.failed,
            report.compression_ratio() * 100.0
        );
    }

    pub fn git(&self, outcome: &GitOutcome) {
        if self.quiet {
            return;
        }
        match outcome {
            GitOutcome::NothingToCommit => println!("没有变更，无需提交"),
            GitOutcome::CommittedAndPushed { commit_summary } => {
                if !commit_summary.is_empty() {
                    println!("{commit_summary}");
                }
                println!("已完成 git add/commit/push 操作");
            }
        }
    }

    pub fn conversion(&self, report: &ConversionReport) {
        if self.quiet {
            return;
        }
        for item in &report.items {
            match item {
                ConversionOutcome::Failed { input, error } => {
                    eprintln!("转换失败 {}: {error}", input.display());
                }
                ConversionOutcome::Converted { warnings, .. } => {
                    for warning in warnings {
                        match &warning.path {
                            Some(path) => eprintln!("警告 {}: {}", path.display(), warning.message),
                            None => eprintln!("警告: {}", warning.message),
                        }
                    }
                }
            }
        }
        println!(
            "视频转换完成：扫描 {}，成功 {}，失败 {}，警告 {}",
            report.scanned, report.succeeded, report.failed, report.warning_count
        );
    }
}

impl ProgressSink for Presenter {
    fn emit(&self, event: ProgressEvent) {
        if self.quiet || self.no_progress {
            return;
        }
        let mut state = self
            .progress
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        match event {
            ProgressEvent::Started { operation, total } => {
                state.operation = operation;
                state.completed = 0;
                match total {
                    Some(total) => println!("开始{}（共 {total} 项）", state.operation),
                    None => println!("开始{}…", state.operation),
                }
            }
            ProgressEvent::Advanced {
                increment,
                total,
                message,
            } => {
                state.completed += increment;
                if let Some(message) = message {
                    match total {
                        Some(total) => println!(
                            "{} {}/{}：{}",
                            state.operation, state.completed, total, message
                        ),
                        None => println!("{}：{}", state.operation, message),
                    }
                }
            }
            ProgressEvent::Message(message) => println!("{message}"),
            ProgressEvent::Finished => {}
        }
    }
}
