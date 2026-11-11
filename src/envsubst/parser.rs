use crate::cli::EnvErrorMode;
use anyhow::{Result, bail};
use std::{
    io::{Read, Write},
    str,
};

#[cfg(not(test))]
const READ_BUFFER_SIZE: usize = 1024 * 256;
#[cfg(not(test))]
const MAX_ENV_LEN: usize = 1024 * 4;
#[cfg(not(test))]
const MAX_ERROR_BUFFER_LEN: usize = 50;
const ENV_SIGN: u8 = b'$';
const START_SIGN: u8 = b'{';
const END_SIGN: u8 = b'}';
const DEFAULT_VALUE_SEP: &str = ":";
// Guard: forces a compile error here if windows support is ever added,
// as a reminder to pick the right line ending (tests won't catch \r vs \n).
#[cfg(not(windows))]
const NEWLINE_CHAR: &str = "\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanStep {
    Normal,
    FindEnvSign,
    CollectEnvName,
}

pub fn substitute<R, W, F, K>(
    mut reader: R,
    mut writer: W,
    env_error_mode: &EnvErrorMode,
    mut resolve_env_func: F,
    mut filter_env_function: K,
) -> Result<()>
where
    R: Read,
    W: Write,
    F: FnMut(&str) -> Result<String>,
    K: FnMut(&str) -> Result<()>,
{
    let mut read_buffer: [u8; READ_BUFFER_SIZE] = [0; READ_BUFFER_SIZE];
    let mut scan_step = ScanStep::Normal;
    let mut env_name_buffer = Vec::with_capacity(MAX_ENV_LEN);
    let mut errors_buffer: Vec<String> = Vec::with_capacity(MAX_ERROR_BUFFER_LEN);
    let mut out_buffer: Vec<u8> = Vec::with_capacity(READ_BUFFER_SIZE);
    let mut read_size: usize = 1;

    while read_size > 0 {
        read_size = reader.read(&mut read_buffer)?;
        for &byte in &read_buffer[..read_size] {
            match scan_step {
                ScanStep::Normal => {
                    if byte == ENV_SIGN {
                        scan_step = ScanStep::FindEnvSign;
                        continue;
                    }
                    out_buffer.push(byte);
                }
                ScanStep::FindEnvSign => {
                    if byte == START_SIGN {
                        scan_step = ScanStep::CollectEnvName;
                        continue;
                    }
                    out_buffer.push(ENV_SIGN);
                    out_buffer.push(byte);
                    scan_step = ScanStep::Normal;
                }
                ScanStep::CollectEnvName => {
                    if env_name_buffer.len() >= MAX_ENV_LEN {
                        out_buffer.push(ENV_SIGN);
                        out_buffer.push(START_SIGN);
                        out_buffer.extend_from_slice(&env_name_buffer);
                        out_buffer.push(byte);
                        env_name_buffer.clear();
                        scan_step = ScanStep::Normal;
                    } else if byte == END_SIGN {
                        let placeholder = str::from_utf8(&env_name_buffer)?;
                        let restored_string = process_env(
                            placeholder,
                            env_error_mode,
                            &mut errors_buffer,
                            &mut resolve_env_func,
                        )?;
                        filter_env_function(&parse_placeholder(placeholder).0)?;
                        out_buffer.extend(restored_string.as_bytes());
                        env_name_buffer.clear();
                        scan_step = ScanStep::Normal;
                    } else {
                        env_name_buffer.push(byte);
                    }
                }
            }
        }
        if errors_buffer.is_empty() {
            writer.write_all(&out_buffer)?;
        }
        out_buffer.clear();
    }
    if !errors_buffer.is_empty() {
        bail!("{NEWLINE_CHAR}{}", errors_buffer.join(NEWLINE_CHAR))
    }
    flush_pending_state(scan_step, &env_name_buffer, &mut out_buffer);
    writer.write_all(&out_buffer)?;
    Ok(())
}

fn parse_placeholder(placeholder: &str) -> (String, Option<String>) {
    let (env_name, default_value) = placeholder
        .split_once(DEFAULT_VALUE_SEP)
        .map_or((placeholder.to_owned(), None), |(name, default)| {
            (name.to_owned(), Some(default.to_owned()))
        });
    (env_name, default_value)
}

fn process_env<F: FnMut(&str) -> Result<String>>(
    placeholder: &str,
    env_error_mode: &EnvErrorMode,
    errors_buffer: &mut Vec<String>,
    resolve_env_func: &mut F,
) -> Result<String> {
    let (env_name, default_value) = parse_placeholder(placeholder);
    let result = resolve_env_func(&env_name);
    match result {
        Ok(val) => Ok(val),
        Err(err) => {
            if let Some(default_value) = default_value {
                return Ok(default_value.to_string());
            }
            match env_error_mode {
                EnvErrorMode::Empty => Ok(String::new()),
                EnvErrorMode::Keep => Ok(format!(
                    "{}{}{}{}",
                    ENV_SIGN as char, START_SIGN as char, placeholder, END_SIGN as char
                )),
                EnvErrorMode::Error => {
                    if errors_buffer.len() >= MAX_ERROR_BUFFER_LEN {
                        bail!("{NEWLINE_CHAR}{}", errors_buffer.join(NEWLINE_CHAR))
                    }
                    errors_buffer.push(format!("{err} `{env_name}`"));
                    Ok(String::new())
                }
            }
        }
    }
}

fn flush_pending_state(scan_step: ScanStep, env_name_buffer: &[u8], out_buffer: &mut Vec<u8>) {
    match scan_step {
        ScanStep::Normal => {}

        ScanStep::FindEnvSign => {
            out_buffer.push(ENV_SIGN);
        }

        ScanStep::CollectEnvName => {
            out_buffer.push(ENV_SIGN);
            out_buffer.push(START_SIGN);
            out_buffer.extend_from_slice(env_name_buffer);
        }
    }
}

#[cfg(test)]
const READ_BUFFER_SIZE: usize = 1;
#[cfg(test)]
const MAX_ENV_LEN: usize = 128;
#[cfg(test)]
const MAX_ERROR_BUFFER_LEN: usize = 2;

#[cfg(test)]
mod tests {

    use super::*;
    use pretty_assertions::assert_eq;
    use std::io::Cursor;

    fn mock_get_env(key: &str) -> Result<String> {
        if key.eq("SUBST_VAR") {
            Ok(String::from("value"))
        } else {
            bail!("Not found")
        }
    }

    fn mock_filter_env(_placeholder: &str) -> Result<()> {
        Ok(())
    }

    #[test]
    fn missing_var_empty_mode() {
        let mock_reader = b"This var must be empty: ${EMPTY_VAR}";
        let mut mock_writer = Cursor::new(Vec::new());
        substitute(
            &mock_reader[..],
            &mut mock_writer,
            &EnvErrorMode::Empty,
            mock_get_env,
            mock_filter_env,
        )
        .unwrap();
        let written_data = mock_writer.into_inner();
        assert_eq!(written_data, b"This var must be empty: ");
    }

    #[test]
    fn missing_var_error_mode() {
        let mock_reader = b"This must be an error: ${UNDECLARED_VAR}";
        let mut mock_writer = Cursor::new(Vec::new());
        let result = substitute(
            &mock_reader[..],
            &mut mock_writer,
            &EnvErrorMode::Error,
            mock_get_env,
            mock_filter_env,
        );
        assert!(result.is_err_and(|err| { err.to_string().contains("`UNDECLARED_VAR`") }));
    }

    #[test]
    fn check_max_error_len_in_error_mode() {
        let mock_reader =
            b"This must be an error: ${UNDECLARED_VAR} and ${OTHER_VAR} and ${ANOTHER_VAR}";
        let mut mock_writer = Cursor::new(Vec::new());
        let result = substitute(
            &mock_reader[..],
            &mut mock_writer,
            &EnvErrorMode::Error,
            mock_get_env,
            mock_filter_env,
        );
        assert!(result.is_err_and(|err| {
            err.to_string().contains("`UNDECLARED_VAR`")
                & err.to_string().contains("`OTHER_VAR`")
                & !err.to_string().contains("`ANOTHER_VAR`")
        }));
    }

    #[test]
    fn missing_var_keep_mode() {
        let mock_reader = b"This var must be the same: ${SAME_VAR}";
        let mut mock_writer = Cursor::new(Vec::new());
        substitute(
            &mock_reader[..],
            &mut mock_writer,
            &EnvErrorMode::Keep,
            mock_get_env,
            mock_filter_env,
        )
        .unwrap();
        let written_data = mock_writer.into_inner();
        assert_eq!(written_data, b"This var must be the same: ${SAME_VAR}");
    }

    #[test]
    fn multibyte_utf8_survives_tiny_reads() {
        let mock_reader = "привет ${MY_VAR} € 🎉 конец".as_bytes();
        let mut mock_writer = Cursor::new(Vec::new());
        substitute(
            mock_reader,
            &mut mock_writer,
            &EnvErrorMode::Keep,
            mock_get_env,
            mock_filter_env,
        )
        .unwrap();
        let written_data = mock_writer.into_inner();
        assert_eq!(written_data, "привет ${MY_VAR} € 🎉 конец".as_bytes());
    }

    #[test]
    fn normal_substitution() {
        let mock_reader = b"This var must be substitute: ${SUBST_VAR}";
        let mut mock_writer = Cursor::new(Vec::new());
        substitute(
            &mock_reader[..],
            &mut mock_writer,
            &EnvErrorMode::Keep,
            mock_get_env,
            mock_filter_env,
        )
        .unwrap();
        let written_data = mock_writer.into_inner();
        assert_eq!(written_data, b"This var must be substitute: value");
    }

    #[test]
    fn overflow_env_name_preserved() {
        let long_name = "V".repeat(MAX_ENV_LEN + 1);
        let mock_reader = format!("Long var: ${{{long_name}}}");
        let mut mock_writer = Cursor::new(Vec::new());
        substitute(
            mock_reader.as_bytes(),
            &mut mock_writer,
            &EnvErrorMode::Error,
            mock_get_env,
            mock_filter_env,
        )
        .unwrap();
        let written_data = mock_writer.into_inner();
        assert_eq!(written_data, mock_reader.as_bytes());
    }

    #[test]
    fn trailing_dollar_at_eof_is_preserved() {
        let mock_reader = b"This text must be the same: $";
        let mut mock_writer = Cursor::new(Vec::new());
        substitute(
            &mock_reader[..],
            &mut mock_writer,
            &EnvErrorMode::Keep,
            mock_get_env,
            mock_filter_env,
        )
        .unwrap();
        let written_data = mock_writer.into_inner();
        assert_eq!(written_data, b"This text must be the same: $");
    }

    #[test]
    fn unterminated_brace_at_eof_is_preserved() {
        let mock_reader = b"This text must be the same: ${SUBST_VA";
        let mut mock_writer = Cursor::new(Vec::new());
        substitute(
            &mock_reader[..],
            &mut mock_writer,
            &EnvErrorMode::Keep,
            mock_get_env,
            mock_filter_env,
        )
        .unwrap();
        let written_data = mock_writer.into_inner();
        assert_eq!(written_data, b"This text must be the same: ${SUBST_VA");
    }

    #[test]
    fn default_value_used_when_var_is_unset() {
        let mock_reader = b"port=${PORT:8080}";
        let mut mock_writer = Cursor::new(Vec::new());
        substitute(
            &mock_reader[..],
            &mut mock_writer,
            &EnvErrorMode::Error,
            mock_get_env,
            mock_filter_env,
        )
        .unwrap();
        let written_data = mock_writer.into_inner();
        assert_eq!(written_data, b"port=8080");
    }

    #[test]
    fn default_value_ignored_when_var_is_set() {
        let mock_reader = b"value=${SUBST_VAR:ignored}";
        let mut mock_writer = Cursor::new(Vec::new());
        substitute(
            &mock_reader[..],
            &mut mock_writer,
            &EnvErrorMode::Keep,
            mock_get_env,
            mock_filter_env,
        )
        .unwrap();
        let written_data = mock_writer.into_inner();
        assert_eq!(written_data, b"value=value");
    }

    #[test]
    fn default_value_not_used_for_empty_var() {
        let mock_reader = b"port=${PORT:8080}";
        let mut mock_writer = Cursor::new(Vec::new());
        let mock_get_env = |key: &str| {
            if key.eq("PORT") {
                Ok(String::new())
            } else {
                bail!("")
            }
        };
        substitute(
            &mock_reader[..],
            &mut mock_writer,
            &EnvErrorMode::Keep,
            mock_get_env,
            mock_filter_env,
        )
        .unwrap();
        let written_data = mock_writer.into_inner();
        assert_eq!(written_data, b"port=");
    }

    #[test]
    fn default_value_may_contain_colons() {
        let mock_reader = b"url=${API_URL:https://localhost:8080/api}";
        let mut mock_writer = Cursor::new(Vec::new());
        substitute(
            &mock_reader[..],
            &mut mock_writer,
            &EnvErrorMode::Error,
            mock_get_env,
            mock_filter_env,
        )
        .unwrap();
        let written_data = mock_writer.into_inner();
        assert_eq!(written_data, b"url=https://localhost:8080/api");
    }

    #[test]
    fn default_value_overrides_missing_env_error_mode() {
        let mock_reader = b"value=${MISSING:fallback}";
        let mut mock_writer = Cursor::new(Vec::new());
        substitute(
            &mock_reader[..],
            &mut mock_writer,
            &EnvErrorMode::Error,
            mock_get_env,
            mock_filter_env,
        )
        .unwrap();
        let written_data = mock_writer.into_inner();
        assert_eq!(written_data, b"value=fallback");
    }

    #[test]
    fn empty_default_value_used_when_var_is_unset() {
        let mock_reader = b"value=${VAR:}";
        let mut mock_writer = Cursor::new(Vec::new());
        substitute(
            &mock_reader[..],
            &mut mock_writer,
            &EnvErrorMode::Keep,
            mock_get_env,
            mock_filter_env,
        )
        .unwrap();
        let written_data = mock_writer.into_inner();
        assert_eq!(written_data, b"value=");
    }
}
