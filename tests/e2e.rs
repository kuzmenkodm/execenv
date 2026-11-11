use assert_cmd::Command;
use pretty_assertions::assert_eq;
use std::fs;

use tempfile::tempdir;

fn create_cmd() -> Command {
    Command::cargo_bin("execenv").unwrap()
}

#[test]
fn subst_env_in_place_and_exec() {
    let mut command = create_cmd();

    let text = "${NAME} was here. Briefly.";
    let result_text = "Brian was here. Briefly.";
    let dir = tempdir().unwrap();
    let input_file_path = dir.path().join("conf.tmp");

    fs::write(&input_file_path, text).unwrap();

    let assert = command
        .args([
            "--files",
            input_file_path.to_str().unwrap(),
            "--exec",
            "/bin/true",
        ])
        .env("NAME", "Brian")
        .assert();

    assert.success();
    assert_eq!(result_text, fs::read_to_string(input_file_path).unwrap());
}

#[test]
fn subst_env_in_output_file_and_exec() {
    let mut command = create_cmd();

    let text = "${NAME} was here. Briefly.";
    let result_text = "Brian was here. Briefly.";
    let dir = tempdir().unwrap();
    let input_file_path = dir.path().join("conf.tmp");
    let output_file_path = dir.path().join("conf.txt");
    let mapping = format!(
        "{}={}",
        input_file_path.display(),
        output_file_path.display()
    );
    fs::write(&input_file_path, text).unwrap();

    let assert = command
        .args(["--files", &mapping, "--exec", "/bin/true"])
        .env("NAME", "Brian")
        .assert();

    assert.success();
    assert_eq!(result_text, fs::read_to_string(output_file_path).unwrap());
    assert_eq!(text, fs::read_to_string(input_file_path).unwrap());
}

#[test]
fn render_error_aborts_exec() {
    let mut command = create_cmd();

    let text = "${MISSING} was here. Briefly.";
    let dir = tempdir().unwrap();
    let input_file_path = dir.path().join("conf.tmp");
    fs::write(&input_file_path, text).unwrap();

    let assert = command
        .args([
            "--files",
            input_file_path.to_str().unwrap(),
            "--missing-env",
            "error",
            "--exec",
            "/bin/true",
        ])
        .assert();

    assert
        .failure()
        .stderr(predicates::str::contains("MISSING"));
    assert_eq!(text, fs::read_to_string(input_file_path).unwrap());
}

#[test]
fn subst_env_in_multiple_files_in_place() {
    let mut command = create_cmd();

    let text_1 = "${NAME_1} was here. Briefly.";
    let result_text_1 = "Brian was here. Briefly.";
    let text_2 = "${NAME_2} was here. Briefly.";
    let result_text_2 = "Alex was here. Briefly.";

    let dir = tempdir().unwrap();
    let input_file_path_1 = dir.path().join("conf_1.txt");
    let input_file_path_2 = dir.path().join("conf_2.txt");

    fs::write(&input_file_path_1, text_1).unwrap();
    fs::write(&input_file_path_2, text_2).unwrap();

    let assert = command
        .args([
            "--files",
            input_file_path_1.to_str().unwrap(),
            input_file_path_2.to_str().unwrap(),
            "--exec",
            "/bin/true",
        ])
        .env("NAME_1", "Brian")
        .env("NAME_2", "Alex")
        .assert();
    assert.success();
    assert_eq!(
        result_text_1,
        fs::read_to_string(input_file_path_1).unwrap()
    );
    assert_eq!(
        result_text_2,
        fs::read_to_string(input_file_path_2).unwrap()
    );
}

#[test]
fn unset_rendered_envs() {
    let mut command = create_cmd();

    let text_1 = "${NAME_1} was here. Briefly.";
    let result_text_1 = "Brian was here. Briefly.";
    let text_2 = "${NAME_2} was here. Briefly.";
    let result_text_2 = "Alex was here. Briefly.";

    let dir = tempdir().unwrap();
    let input_file_path_1 = dir.path().join("conf_1.txt");
    let input_file_path_2 = dir.path().join("conf_2.txt");

    fs::write(&input_file_path_1, text_1).unwrap();
    fs::write(&input_file_path_2, text_2).unwrap();

    let assert = command
        .args([
            "--files",
            input_file_path_1.to_str().unwrap(),
            input_file_path_2.to_str().unwrap(),
            "--unset-rendered-envs",
            "--exec",
            "/bin/env",
        ])
        .env("NAME_1", "Brian")
        .env("NAME_2", "Alex")
        .env("NAME_3", "Toby")
        .assert();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert.success();
    assert_eq!(
        result_text_1,
        fs::read_to_string(input_file_path_1).unwrap()
    );
    assert_eq!(
        result_text_2,
        fs::read_to_string(input_file_path_2).unwrap()
    );
    assert!(!stdout.contains("NAME_1=Brian"));
    assert!(!stdout.contains("NAME_2=Alex"));
    assert!(stdout.contains("NAME_3=Toby"));
}

#[test]
fn exclude_specified_envs() {
    let mut command = create_cmd();

    let text_1 = "${NAME_1} was here. Briefly.";
    let result_text_1 = "Brian was here. Briefly.";
    let text_2 = "${NAME_2} was here. Briefly.";
    let result_text_2 = "Alex was here. Briefly.";

    let dir = tempdir().unwrap();
    let input_file_path_1 = dir.path().join("conf_1.txt");
    let input_file_path_2 = dir.path().join("conf_2.txt");

    fs::write(&input_file_path_1, text_1).unwrap();
    fs::write(&input_file_path_2, text_2).unwrap();

    let assert = command
        .args([
            "--files",
            input_file_path_1.to_str().unwrap(),
            input_file_path_2.to_str().unwrap(),
            "--excluded-envs",
            "NAME_1,NAME_2",
            "--exec",
            "/bin/env",
        ])
        .env("NAME_1", "Brian")
        .env("NAME_2", "Alex")
        .env("NAME_3", "Toby")
        .assert();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert.success();
    assert_eq!(
        result_text_1,
        fs::read_to_string(input_file_path_1).unwrap()
    );
    assert_eq!(
        result_text_2,
        fs::read_to_string(input_file_path_2).unwrap()
    );
    assert!(!stdout.contains("NAME_1=Brian"));
    assert!(!stdout.contains("NAME_2=Alex"));
    assert!(stdout.contains("NAME_3=Toby"));
}

#[test]
fn unset_rendered_and_exclude_unreferenced_envs() {
    let mut command = create_cmd();

    let text_1 = "${NAME_1} was here. Briefly.";
    let result_text_1 = "Brian was here. Briefly.";
    let text_2 = "${NAME_2} was here. Briefly.";
    let result_text_2 = "Alex was here. Briefly.";

    let dir = tempdir().unwrap();
    let input_file_path_1 = dir.path().join("conf_1.txt");
    let input_file_path_2 = dir.path().join("conf_2.txt");

    fs::write(&input_file_path_1, text_1).unwrap();
    fs::write(&input_file_path_2, text_2).unwrap();

    let assert = command
        .args([
            "--files",
            input_file_path_1.to_str().unwrap(),
            input_file_path_2.to_str().unwrap(),
            "--unset-rendered-envs",
            "--excluded-envs",
            "NAME_3",
            "--exec",
            "/bin/env",
        ])
        .env("NAME_1", "Brian")
        .env("NAME_2", "Alex")
        .env("NAME_3", "Toby")
        .assert();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert.success();
    assert_eq!(
        result_text_1,
        fs::read_to_string(input_file_path_1).unwrap()
    );
    assert_eq!(
        result_text_2,
        fs::read_to_string(input_file_path_2).unwrap()
    );
    assert!(!stdout.contains("NAME_1=Brian"));
    assert!(!stdout.contains("NAME_2=Alex"));
    assert!(!stdout.contains("NAME_3=Toby"));
}

fn excluded_names(count: usize) -> String {
    (0..count)
        .map(|index| format!("V{index}"))
        .collect::<Vec<_>>()
        .join(",")
}

#[test]
fn env_limit_applies_to_explicit_names_before_exec() {
    let dir = tempdir().unwrap();
    let marker = dir.path().join("executed");

    for count in [9999, 10000, 10001] {
        let mut command = create_cmd();
        let assert = command
            .args([
                "--excluded-envs",
                &excluded_names(count),
                "--exec",
                "/bin/touch",
            ])
            .arg(&marker)
            .assert();

        if count <= 10000 {
            assert.success();
            assert!(marker.exists());
            fs::remove_file(&marker).unwrap();
        } else {
            assert.failure().stderr(predicates::str::contains("10000"));
            assert!(!marker.exists());
        }
    }
}

#[test]
fn env_limit_is_fatal_for_all_missing_modes_and_defaults() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("template");
    let output = dir.path().join("config");
    let marker = dir.path().join("executed");
    let mapping = format!("{}={}", input.display(), output.display());
    let rendered_names = (0..10000)
        .map(|index| format!("${{V{index}:fallback}}"))
        .collect::<Vec<_>>()
        .join("\n");

    for explicit in [false, true] {
        for mode in ["empty", "keep", "error"] {
            for placeholder in ["${EXTRA}", "${EXTRA:fallback}", "${EXTRA:}"] {
                let template = if explicit {
                    placeholder.to_owned()
                } else {
                    format!("{rendered_names}\n{placeholder}")
                };
                fs::write(&input, &template).unwrap();
                fs::write(&output, "existing configuration").unwrap();

                let mut command = create_cmd();
                command.env_clear().env("EXTRA", "actual").args([
                    "--files",
                    &mapping,
                    "--unset-rendered-envs",
                    "--missing-env",
                    mode,
                ]);
                if explicit {
                    command.args(["--excluded-envs", &excluded_names(10000)]);
                }
                command
                    .args(["--exec", "/bin/touch"])
                    .arg(&marker)
                    .assert()
                    .failure()
                    .stderr(predicates::str::contains("10000"));

                assert!(!marker.exists());
                assert_eq!(fs::read_to_string(&input).unwrap(), template);
                assert_eq!(
                    fs::read_to_string(&output).unwrap(),
                    "existing configuration"
                );
                assert!(!output.with_added_extension("execenv_tmp").exists());
            }
        }
    }
}
