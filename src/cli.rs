use clap::{Parser, ValueEnum};

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum EnvErrorMode {
    Empty,
    Error,
    Keep,
}

#[derive(Parser, Debug)]
#[command(version)]
#[command(arg_required_else_help = true)]
#[command(about = "Render environment variables into files, then optionally exec a command")]
#[command(
    long_about = r#"execenv renders Spring Boot-style `${VAR}` placeholders in one or more files using values from the current environment.

Supported forms:
  ${VAR}
  ${VAR:default}

By default, each file is rewritten in place. To keep the source file unchanged and write the rendered output somewhere else, pass a mapping in the `<input>=<output>` form.

Rendering is performed before the command is executed. If any file fails to render, the command is not started.

Missing variables in `${VAR}` placeholders are controlled by `--missing-env`:
  empty  Replace missing variables with an empty string. This is the default.
  keep   Leave unresolved `${VAR}` placeholders unchanged.
  error  Fail immediately when a referenced variable is not set.

When a default value is specified in `${VAR:default}`, it is used if the variable is unset and `--missing-env` is ignored for that placeholder.

When `--exec` is passed, execenv replaces itself with the requested command using exec(2). Everything after `--exec` is treated as the command and its arguments, including values that look like flags.

The command inherits the current environment by default. Use `--unset-rendered-envs` to remove variables referenced in the templates, or `--excluded-envs VAR1,VAR2` to remove specific variables. Both options can be combined and only affect the command's environment after rendering.

You can also use only the rendering step by omitting `--exec`."#
)]
pub struct Cli {
    /// How to handle missing environment variables.
    ///
    /// Values:
    ///     empty: replace missing variables with an empty string
    ///     error: fail if a referenced variable is not set
    ///     keep:  leave unresolved placeholders unchanged
    #[arg(short, long, default_value = "empty", verbatim_doc_comment)]
    pub missing_env: EnvErrorMode,

    /// Files to render.
    ///
    /// Use a bare path to rewrite a file in place:
    ///     /app/config.yaml
    ///
    /// Use <input>=<output> to render one file into another:
    ///     /templates/config.yaml=/app/config.yaml
    #[arg(short, long, num_args = 1.., verbatim_doc_comment)]
    pub files: Vec<String>,

    /// Remove variables referenced in templates from the command's environment.
    ///
    /// Variables remain available during rendering and are removed before execution.
    /// Other variables are inherited. Referenced variables such as HOME or PATH
    /// are also removed, even if the command needs them.
    #[arg(short, long, verbatim_doc_comment)]
    pub unset_rendered_envs: bool,

    /// Environment variables to exclude from the command's environment.
    ///
    /// Comma-separated variable names:
    ///     VAR1,VAR2,VAR3
    #[arg(short = 'x', long, value_delimiter = ',', verbatim_doc_comment)]
    pub excluded_envs: Vec<String>,

    /// Execute a command after all files have been rendered successfully.
    ///
    /// Everything after --exec is treated as the command and its arguments,
    /// including values that start with a hyphen.
    #[arg(short = 'e', long, verbatim_doc_comment)]
    pub exec: bool,

    #[arg(requires = "exec", trailing_var_arg = true, allow_hyphen_values = true)]
    pub cmd: Vec<String>,
}

pub fn parse_cli_args() -> Cli {
    Cli::parse()
}
