use crate::cli::EnvErrorMode;
use anyhow::{Context, Result, bail};
use std::{
    collections::HashSet,
    env,
    fs::{File, remove_file, rename},
    io::{BufWriter, Write},
    path::Path,
};

mod parser;

const MAX_FILTERED_ENVS: usize = 10000;

pub fn exclude_env(excluded: &mut HashSet<String>, name: &str) -> Result<()> {
    if excluded.len() >= MAX_FILTERED_ENVS {
        bail!("cannot exclude more than {MAX_FILTERED_ENVS} unique environment variables");
    }
    excluded.insert(name.to_owned());
    Ok(())
}

pub fn envsubst(
    env_file_path: &Path,
    result_file_path: &Path,
    env_error_mode: &EnvErrorMode,
    mut excluded_envs: Option<&mut HashSet<String>>,
) -> Result<()> {
    let tmp_file_path = result_file_path.with_added_extension("execenv_tmp");
    let input_file = File::open(env_file_path)
        .with_context(|| format!("failed to open input file `{}`", env_file_path.display()))?;
    let tmp_file = File::create(&tmp_file_path).with_context(|| {
        format!(
            "failed to create temporary file `{}`",
            tmp_file_path.display()
        )
    })?;
    tmp_file.set_permissions(input_file.metadata()?.permissions())?;
    let mut writer = BufWriter::new(tmp_file);

    let filter_env_function = |name: &str| {
        if let Some(excluded) = excluded_envs.as_deref_mut() {
            exclude_env(excluded, name)?
        }
        Ok::<(), anyhow::Error>(())
    };

    parser::substitute(
        input_file,
        &mut writer,
        env_error_mode,
        get_env,
        filter_env_function,
    )
    .and_then(|_| {
        writer
            .flush()
            .with_context(|| format!("failed to flush `{}`", tmp_file_path.display()))
    })
    .and_then(|_| {
        rename(&tmp_file_path, result_file_path).with_context(|| {
            format!(
                "failed to rename `{}` to `{}`",
                tmp_file_path.display(),
                result_file_path.display()
            )
        })
    })
    .inspect_err(|_| {
        let _ = remove_file(&tmp_file_path);
    })
}

fn get_env(key: &str) -> Result<String> {
    let env = env::var(key);
    match env {
        Ok(val) => Ok(val),
        Err(err) => Err(err.into()),
    }
}
