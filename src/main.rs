use clap::Parser;
use cli::{Cli, Command};
use maya_core::Result;
use presenter::Presenter;
use std::sync::Arc;

mod cli;
mod presenter;

mod modules {
    pub mod clean_ops;
    pub mod git_ops;
    pub mod optimize_ops;
    pub mod pack_ops;
    pub mod transform_ops;
}

#[tokio::main]
async fn main() {
    let result = run(Cli::parse()).await;
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(i32::from(error.exit_code()));
    }
}

async fn run(cli: Cli) -> Result<()> {
    let presenter = Arc::new(Presenter::new(cli.quiet, cli.no_progress));

    match cli.command {
        Command::Clean { types, path } => {
            modules::clean_ops::handle_clean_ops(&types, &path, presenter.as_ref())?;
        }
        Command::Git { ops, path, message } => {
            modules::git_ops::handle_git_ops(ops, &path, &message, presenter.as_ref())?;
        }
        Command::Pack {
            pack_type,
            path,
            out_dir,
        } => {
            modules::pack_ops::handle_pack_ops(pack_type, &path, out_dir, presenter.as_ref())?;
        }
        Command::Optimize {
            types,
            path,
            new_file,
            jpeg_quality,
            failure_policy,
        } => {
            modules::optimize_ops::handle_optimize_ops(
                &types,
                &path,
                new_file,
                jpeg_quality,
                failure_policy,
                presenter.as_ref(),
            )?;
        }
        Command::Transform {
            types,
            path,
            failure_policy,
        } => {
            modules::transform_ops::handle_transform_ops(
                &types,
                &path,
                failure_policy,
                Arc::clone(&presenter),
            )
            .await?;
        }
    }
    Ok(())
}
