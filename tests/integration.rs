use std::io::Write;
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_tumbler");

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(BIN)
        .args(args)
        .output()
        .expect("failed to spawn binary");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    (out.status.success(), stdout, stderr)
}

fn run_with_stdin(args: &[&str], input: &str) -> (bool, String, String) {
    let mut child = Command::new(BIN)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn binary");
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all(input.as_bytes())
            .expect("failed to write stdin");
    }
    let out = child.wait_with_output().expect("failed to wait for binary");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    (out.status.success(), stdout, stderr)
}

// ── output shape ────────────────────────────────────────────────────────────

#[test]
fn default_six_words_capitalized_no_separator() {
    let (ok, stdout, _) = run(&["--print"]);
    assert!(ok);
    let phrase = stdout.trim();
    // Words are alpha + optional hyphens (e.g. "Drop-down"); no spaces
    assert!(
        phrase.chars().all(|c| c.is_ascii_alphabetic() || c == '-'),
        "expected alpha/hyphen output, got: {phrase:?}"
    );
    assert!(
        phrase.chars().next().unwrap().is_uppercase(),
        "first char should be uppercase"
    );
}

#[test]
fn word_count_respected() {
    for n in ["1", "4", "6", "8"] {
        let (ok, stdout, _) = run(&["--print", "--words", n, "--separator", " "]);
        assert!(ok, "tumbler exited with error for --words {n}");
        let count = stdout.trim().split(' ').count();
        assert_eq!(
            count,
            n.parse::<usize>().unwrap(),
            "--words {n} produced {count} words"
        );
    }
}

#[test]
fn no_capitalize_flag_produces_lowercase() {
    let (ok, stdout, _) = run(&["--print", "--no-capitalize", "--separator", " "]);
    assert!(ok);
    for word in stdout.trim().split(' ') {
        assert_eq!(
            word,
            word.to_lowercase(),
            "word {word:?} should be all lowercase"
        );
    }
}

#[test]
fn separator_appears_between_words() {
    let (ok, stdout, _) = run(&["--print", "--words", "4", "--separator", "-"]);
    assert!(ok);
    let phrase = stdout.trim();
    assert_eq!(
        phrase.matches('-').count(),
        3,
        "4 words with '-' separator need exactly 3 hyphens; got: {phrase:?}"
    );
}

#[test]
fn all_words_from_wordlist() {
    // Words are drawn from an alpha+hyphen wordlist (e.g. "drop-down").
    let (ok, stdout, _) = run(&["--print", "--words", "6", "--separator", " "]);
    assert!(ok);
    for word in stdout.trim().split(' ') {
        assert!(
            !word.is_empty() && word.chars().all(|c| c.is_ascii_alphabetic() || c == '-'),
            "unexpected token {word:?} in output"
        );
    }
}

// ── entropy flag ─────────────────────────────────────────────────────────────

#[test]
fn entropy_flag_writes_stats_to_stderr() {
    let (ok, _, stderr) = run(&["--print", "--entropy"]);
    assert!(ok);
    assert!(
        stderr.contains("bits/word") && stderr.contains("bits"),
        "expected entropy info in stderr, got: {stderr:?}"
    );
}

#[test]
fn entropy_output_goes_to_stderr_not_stdout() {
    let (ok, stdout, _) = run(&["--print", "--entropy", "--separator", " "]);
    assert!(ok);
    // stdout should still be just the passphrase words, not the stats line
    for token in stdout.trim().split(' ') {
        assert!(
            token.chars().all(|c| c.is_ascii_alphabetic() || c == '-'),
            "entropy stats leaked into stdout: {token:?}"
        );
    }
}

// ── dice mode ────────────────────────────────────────────────────────────────

#[test]
fn dice_mode_prompts_show_incrementing_word_number() {
    // 3 words × 5 dice each = 15 single-digit inputs.
    // Word 1: all 1s → index 0; Word 2: all 6s → index 7775; Word 3: 2,5,3,4,1 → index 2250
    let input = "1\n1\n1\n1\n1\n6\n6\n6\n6\n6\n2\n5\n3\n4\n1\n";
    let (ok, _stdout, stderr) = run_with_stdin(&["--dice", "--words", "3", "--print"], input);
    assert!(ok, "dice mode exited with error; stderr: {stderr}");
    assert!(
        stderr.contains("Word 1/3"),
        "expected 'Word 1/3' in stderr; got: {stderr:?}"
    );
    assert!(
        stderr.contains("Word 2/3"),
        "expected 'Word 2/3' in stderr; got: {stderr:?}"
    );
    assert!(
        stderr.contains("Word 3/3"),
        "expected 'Word 3/3' in stderr; got: {stderr:?}"
    );
}

#[test]
fn dice_mode_produces_correct_word_count() {
    // 4 words × 5 dice each = 20 single-digit inputs.
    let input = "1\n1\n1\n1\n1\n2\n2\n2\n2\n2\n3\n3\n3\n3\n3\n4\n4\n4\n4\n4\n";
    let (ok, stdout, stderr) = run_with_stdin(
        &["--dice", "--words", "4", "--print", "--separator", " "],
        input,
    );
    assert!(ok, "dice mode exited with error; stderr: {stderr}");
    assert_eq!(
        stdout.trim().split(' ').count(),
        4,
        "--words 4 in dice mode should produce 4 words; stdout: {stdout:?}"
    );
}

// ── count mode ───────────────────────────────────────────────────────────────

#[test]
fn count_produces_correct_number_of_lines() {
    for n in ["1", "3", "5"] {
        let (ok, stdout, stderr) = run(&["--print", "--count", n, "--separator", " "]);
        assert!(ok, "--count {n} exited with error; stderr: {stderr}");
        let lines: Vec<&str> = stdout.lines().collect();
        assert_eq!(
            lines.len(),
            n.parse::<usize>().unwrap(),
            "--count {n} should produce {n} lines; stdout: {stdout:?}"
        );
    }
}

#[test]
fn count_each_line_is_valid_passphrase() {
    let (ok, stdout, _) = run(&["--print", "--count", "4", "--separator", " "]);
    assert!(ok);
    for line in stdout.lines() {
        assert!(!line.is_empty(), "output line should not be empty");
        for word in line.split(' ') {
            assert!(
                word.chars().all(|c| c.is_ascii_alphabetic() || c == '-'),
                "unexpected token {word:?} in line {line:?}"
            );
        }
    }
}

#[test]
fn count_zero_exits_with_error() {
    let (ok, _, stderr) = run(&["--print", "--count", "0"]);
    assert!(!ok, "expected non-zero exit for --count 0");
    assert!(
        stderr.contains("count"),
        "error message should mention 'count', got: {stderr:?}"
    );
}

// ── bits mode ────────────────────────────────────────────────────────────────

#[test]
fn bits_produces_enough_words_for_target_entropy() {
    // EFF long list: 7776 words → ~12.925 bits/word
    // 80 bits → ceil(80/12.925) = 7 words
    let (ok, stdout, stderr) = run(&["--print", "--bits", "80", "--separator", " "]);
    assert!(ok, "--bits 80 exited with error; stderr: {stderr}");
    let count = stdout.trim().split(' ').count();
    assert_eq!(
        count, 7,
        "--bits 80 should produce 7 words; stdout: {stdout:?}"
    );
}

#[test]
fn bits_with_count_produces_correct_grid() {
    // --bits 80 → 7 words per line; --count 3 → 3 lines
    let (ok, stdout, stderr) = run(&[
        "--print",
        "--bits",
        "80",
        "--count",
        "3",
        "--separator",
        " ",
    ]);
    assert!(
        ok,
        "--bits 80 --count 3 exited with error; stderr: {stderr}"
    );
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3, "expected 3 lines; stdout: {stdout:?}");
    for line in &lines {
        assert_eq!(
            line.split(' ').count(),
            7,
            "each line should have 7 words; line: {line:?}"
        );
    }
}

#[test]
fn bits_entropy_shows_computed_word_count() {
    // --bits 80 → 7 words; --entropy should report 7, not the default 6
    let (ok, _, stderr) = run(&["--print", "--bits", "80", "--entropy"]);
    assert!(
        ok,
        "--bits 80 --entropy exited with error; stderr: {stderr}"
    );
    assert!(
        stderr.contains("7 words"),
        "entropy line should report computed word count (7), got: {stderr:?}"
    );
}

#[test]
fn bits_zero_exits_with_error() {
    let (ok, _, stderr) = run(&["--print", "--bits", "0"]);
    assert!(!ok, "expected non-zero exit for --bits 0");
    assert!(
        stderr.contains("bits"),
        "error message should mention 'bits', got: {stderr:?}"
    );
}

// ── version flag ─────────────────────────────────────────────────────────────

#[test]
fn version_flag_prints_version() {
    let (ok, stdout, _) = run(&["--version"]);
    assert!(ok);
    let version = stdout.trim();
    assert!(
        version.starts_with(env!("CARGO_PKG_VERSION")),
        "version output should start with package version; got: {version:?}"
    );
}

#[test]
fn version_short_flag_matches_long() {
    let (_, long, _) = run(&["--version"]);
    let (_, short, _) = run(&["-V"]);
    assert_eq!(
        long.trim(),
        short.trim(),
        "-V and --version should produce identical output"
    );
}

#[test]
fn version_dev_includes_git_sha() {
    let pkg_version = env!("CARGO_PKG_VERSION");
    if !pkg_version.contains("-dev") {
        return;
    }
    let (ok, stdout, _) = run(&["--version"]);
    assert!(ok);
    let version = stdout.trim();
    // dev builds: "0.1.2-dev+<sha>" where sha is 7 hex chars
    let sha_part = version
        .strip_prefix(pkg_version)
        .and_then(|s| s.strip_prefix('+'));
    assert!(
        sha_part.is_some_and(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit())),
        "dev version should end with +<hex-sha>; got: {version:?}"
    );
}

#[test]
fn version_exits_without_generating_passphrase() {
    let (ok, stdout, _) = run(&["--version"]);
    assert!(ok);
    // Output is just the version line, no passphrase words
    assert_eq!(
        stdout.lines().count(),
        1,
        "expected exactly one output line; got: {stdout:?}"
    );
}

// ── error handling ───────────────────────────────────────────────────────────

#[test]
fn zero_words_exits_with_error() {
    let (ok, _, stderr) = run(&["--print", "--words", "0"]);
    assert!(!ok, "expected non-zero exit for --words 0");
    assert!(
        stderr.contains("words"),
        "error message should mention 'words', got: {stderr:?}"
    );
}

#[test]
fn bad_device_exits_with_error() {
    let (ok, _, stderr) = run(&["--print", "--device", "/nonexistent/device"]);
    assert!(!ok, "expected non-zero exit for bad device path");
    assert!(
        stderr.contains("/nonexistent/device"),
        "error should name the bad device, got: {stderr:?}"
    );
}
