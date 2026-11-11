use anyhow::{Context, Result};
use std::{collections::HashSet, os::unix::process::CommandExt, process::Command};

pub fn exec_proc(cmd: &str, args: &[String], excluded_envs: &HashSet<String>) -> Result<()> {
    let mut command = Command::new(cmd);
    command.args(args);

    for key in excluded_envs {
        command.env_remove(key);
    }

    Err(command.exec())
        .with_context(|| format!("failed to execute command `{} {}`", cmd, args.join(" ")))
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn failed_to_execute() {
        let cmd = String::from("no_command");
        let args = Vec::new();
        let result = exec_proc(&cmd, &args, &HashSet::new());
        assert!(result.is_err_and(|err| err.to_string().contains("failed to execute command")))
    }
}
