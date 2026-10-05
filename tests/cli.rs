use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Output, Stdio};

fn grdy() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_grdy"));
    cmd.env("XDG_CONFIG_HOME", env!("CARGO_TARGET_TMPDIR"));
    cmd
}

fn run_with_stdin(args: &[&str], input: &[u8]) -> Output {
    let mut child = grdy()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn renders_stdin() {
    let out = run_with_stdin(&["-p"], br#"[{"a": 1}]"#);
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), "a\n1\n");
}

#[test]
fn missing_file_is_an_error_not_a_panic() {
    let out = grdy().arg("no-such-file.json").output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("no-such-file.json"),
        "{}",
        stderr(&out)
    );
    assert!(!stderr(&out).contains("panicked"), "{}", stderr(&out));
}

#[test]
fn non_utf8_input_is_an_error_not_a_panic() {
    let out = run_with_stdin(&[], b"[{\"a\": \"\xff\"}]");
    assert_eq!(out.status.code(), Some(1));
    assert!(!stderr(&out).contains("panicked"), "{}", stderr(&out));
}

#[test]
fn closed_stdout_exits_quietly() {
    let input: String = format!(
        "[{}]",
        (0..200_000)
            .map(|i| format!(r#"{{"a":{}}}"#, i))
            .collect::<Vec<_>>()
            .join(",")
    );
    let mut child = grdy()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let mut first = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut first)
        .unwrap();
    // stdout reader dropped here, closing the pipe
    let out = child.wait_with_output().unwrap();
    assert!(!stderr(&out).contains("panicked"), "{}", stderr(&out));
}

#[test]
fn invalid_config_warns_and_uses_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("grdy")).unwrap();
    std::fs::write(
        tmp.path().join("grdy").join("config.json"),
        r#"{"plain": tru}"#,
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_grdy"))
        .env("XDG_CONFIG_HOME", tmp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(br#"[{"a": 1}]"#)
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    assert!(stderr(&out).contains("config.json"), "{}", stderr(&out));
    assert!(String::from_utf8_lossy(&out.stdout).contains("╭"));
}
