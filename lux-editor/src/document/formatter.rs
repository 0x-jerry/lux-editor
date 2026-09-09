use std::io::Write;

/// Run `command args` with `text` on stdin and return stdout. The formatter is
/// expected to be a stdin/stdout filter (e.g. `oxfmt --stdin`, `rustfmt`).
pub(crate) fn run_formatter(command: &str, args: &str, text: &str) -> Result<String, String> {
    let mut cmd = std::process::Command::new(command);
    for arg in args.split_whitespace() {
        cmd.arg(arg);
    }
    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    let mut child = cmd.spawn().map_err(|err| err.to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|err| err.to_string())?;
    }
    let output = child.wait_with_output().map_err(|err| err.to_string())?;
    if !output.status.success() {
        return Err(format!("formatter exited with status {}", output.status));
    }
    String::from_utf8(output.stdout).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::run_formatter;

    #[test]
    fn cat_passthrough_formats_identity() {
        let result = run_formatter("cat", "", "hello\nworld").unwrap();
        assert_eq!(result, "hello\nworld");
    }

    #[test]
    fn missing_command_errors() {
        let result = run_formatter("lux-formatter-does-not-exist", "", "x");
        assert!(result.is_err());
    }

    #[test]
    fn non_zero_exit_is_an_error() {
        let result = run_formatter("sh", "-c 'exit 3'", "x");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("status"));
    }
}
