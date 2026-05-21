use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn lazylock_initializers_use_bug_expect_messages() {
    let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();

    for path in rust_files(&src_dir) {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));

        for initializer in lazylock_initializers(&source) {
            if initializer.contains(".unwrap()") {
                violations.push(format!(
                    "{} contains .unwrap() in a LazyLock initializer",
                    path.display()
                ));
            }

            for message in expect_messages(&initializer) {
                if !message.starts_with("BUG:") {
                    violations.push(format!(
                        "{} contains non-BUG LazyLock expect message: {message:?}",
                        path.display()
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "LazyLock initializer violations:\n{}",
        violations.join("\n")
    );
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rust_files(dir, &mut files);
    files
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(dir).unwrap_or_else(|err| panic!("failed to read {}: {err}", dir.display()))
    {
        let path = entry
            .unwrap_or_else(|err| panic!("failed to read entry in {}: {err}", dir.display()))
            .path();

        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

fn lazylock_initializers(source: &str) -> Vec<String> {
    let mut initializers = Vec::new();
    let mut current = Vec::new();
    let mut in_initializer = false;

    for line in source.lines() {
        if !in_initializer {
            let Some(start) = line.find("LazyLock::new") else {
                continue;
            };

            current.push(line[start..].to_string());
            in_initializer = true;
        } else {
            current.push(line.to_string());
        }

        let trimmed = line.trim_end();
        if trimmed.ends_with(");") || trimmed.ends_with("});") {
            initializers.push(current.join("\n"));
            current.clear();
            in_initializer = false;
        }
    }

    if in_initializer {
        initializers.push(current.join("\n"));
    }

    initializers
}

fn expect_messages(initializer: &str) -> Vec<&str> {
    let mut messages = Vec::new();
    let mut search_from = 0;

    while let Some(relative_start) = initializer[search_from..].find(".expect(\"") {
        let message_start = search_from + relative_start + ".expect(\"".len();
        let Some(relative_end) = initializer[message_start..].find('"') else {
            break;
        };
        let message_end = message_start + relative_end;
        messages.push(&initializer[message_start..message_end]);
        search_from = message_end + 1;
    }

    messages
}

#[test]
fn lazylock_scanner_handles_internal_semicolon_statements() {
    let source = r#"
static EXAMPLE: LazyLock<Regex> = LazyLock::new(|| {
    let pattern = r"\s+";
    Regex::new(pattern).expect("BUG: invalid EXAMPLE")
});
"#;

    let initializers = lazylock_initializers(source);

    assert_eq!(initializers.len(), 1);
    assert!(
        initializers[0].contains("Regex::new(pattern).expect"),
        "initializer should include statements after the internal semicolon"
    );
}

#[test]
fn lazylock_scanner_handles_crlf_line_endings() {
    let source = "static EXAMPLE: LazyLock<Regex> = LazyLock::new(|| {\r\n    Regex::new(r\"\\s+\").expect(\"BUG: invalid EXAMPLE\")\r\n});\r\nstatic SECOND: LazyLock<Regex> = LazyLock::new(|| Regex::new(r\"\\n+\").expect(\"BUG: invalid SECOND\"));\r\n";

    let initializers = lazylock_initializers(source);

    assert_eq!(initializers.len(), 2);
    assert!(initializers[0].contains("BUG: invalid EXAMPLE"));
    assert!(initializers[1].contains("BUG: invalid SECOND"));
}
