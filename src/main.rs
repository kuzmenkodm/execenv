#[cfg(not(unix))]
compile_error!("execenv is intended for Unix-like systems!");

use anyhow::Result;
use args::{ExecCommand, FilesMapping};
use cli::parse_cli_args;
use envsubst::{envsubst, exclude_env};
use run_process::exec_proc;
use std::collections::HashSet;

mod args;
mod cli;
mod envsubst;
mod run_process;

fn main() -> Result<()> {
    let cli_args = parse_cli_args();
    let mut excluded_envs = HashSet::new();
    for name in cli_args.excluded_envs {
        exclude_env(&mut excluded_envs, &name)?;
    }
    for file_arg in cli_args.files {
        let file_mapping = FilesMapping::parse(&file_arg)?;
        envsubst(
            file_mapping.input(),
            file_mapping.output(),
            &cli_args.missing_env,
            cli_args.unset_rendered_envs.then_some(&mut excluded_envs),
        )?;
    }
    if cli_args.exec {
        let command = ExecCommand::parse(cli_args.cmd)?;
        exec_proc(command.prog(), command.args(), &excluded_envs)?;
    }
    Ok(())
}
