use std::io::Write;

/// Run `command args` with `text` on stdin and return stdout. The formatter is
/// expected to be a stdin/stdout filter (e.g. `oxfmt --stdin`, `rustfmt`).
pub(crate) fn run_formatter(command: &str, args: &str, text: &str) -> Result<String, String> {
    let mut cmd = std::process::Command::new(command);
    for arg in split_args(args) {
        cmd.arg(arg);
    }
    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().map_err(|err| err.to_string())?;
    // Feed stdin from its own thread while the output is drained: a filter that
    // streams its output fills the stdout pipe, blocks on that write and stops
    // reading stdin, so writing the input here first would deadlock both.
    let input = text.as_bytes().to_vec();
    let mut stdin = child.stdin.take();
    let writer = std::thread::spawn(move || {
        if let Some(stdin) = stdin.as_mut() {
            // A filter that exits without reading it all (or fails) closes the
            // pipe under us; the exit status is the real verdict.
            let _ = stdin.write_all(&input);
        }
    });
    let output = child.wait_with_output().map_err(|err| err.to_string())?;
    let _ = writer.join();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim();
        return Err(if stderr.is_empty() {
            format!("formatter exited with status {}", output.status)
        } else {
            format!("formatter exited with status {}: {stderr}", output.status)
        });
    }
    String::from_utf8(output.stdout).map_err(|err| err.to_string())
}

/// Split `args` on whitespace, keeping single- and double-quoted runs (quotes
/// removed) so `--config "a b.json"` stays one argument.
fn split_args(args: &str) -> Vec<String> {
    let mut parsed = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut started = false;
    for character in args.chars() {
        match quote {
            Some(open) if character == open => quote = None,
            Some(_) => current.push(character),
            None if character == '"' || character == '\'' => {
                quote = Some(character);
                started = true;
            }
            None if character.is_whitespace() => {
                if started {
                    parsed.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            None => {
                current.push(character);
                started = true;
            }
        }
    }
    if started {
        parsed.push(current);
    }
    parsed
}

#[cfg(test)]
mod tests {
    use super::{run_formatter, split_args};

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

    /// A filter that streams stdout before it has read all of stdin must not
    /// deadlock the editor's write. Run on its own thread so a regression times
    /// out instead of hanging the whole suite.
    #[test]
    fn streaming_filter_does_not_deadlock() {
        let mut text = String::new();
        for index in 0..20_000 {
            text.push_str(&format!("line {index}\n"));
        }
        assert!(text.len() > 128 * 1024, "wider than a pipe buffer");

        let expected = text.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(run_formatter("cat", "", &text));
        });
        let result = rx
            .recv_timeout(std::time::Duration::from_secs(30))
            .expect("the formatter round-trip must not deadlock");
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn args_split_on_whitespace_with_quotes_kept() {
        assert_eq!(split_args(""), Vec::<String>::new());
        assert_eq!(split_args("  -c   'exit 3' "), vec!["-c", "exit 3"]);
        assert_eq!(
            split_args("--config \"a b.json\" --write"),
            vec!["--config", "a b.json", "--write"]
        );
        assert_eq!(split_args("--empty ''"), vec!["--empty", ""]);
    }
}
