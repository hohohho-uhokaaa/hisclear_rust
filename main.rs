use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

// bash_history の保持範囲を区切る文字列。
const BASH_HISTORY_DELIMITER: &str = "###";
// .bash_history がない場合に使う既定履歴ファイル。
const DEFAULT_HISTORY_FILE: &str = "default_history.txt";
// 表示メッセージを定義する設定ファイル。
const MESSAGES_FILE: &str = "messages.ini";

// Linux 環境を確認し、履歴の整理とログファイルの削除を順に実行する。
fn main() {
    // 設定ファイルの配置先を決め、メッセージ辞書を読み込む。
    let app_dir = application_directory();
    let messages = load_messages(&app_dir);

    // Linux 専用ツールのため、他の OS では処理を行わず終了する。
    if !cfg!(target_os = "linux") {
        println!("{}", get_message(&messages, "ProgramNotForLinux"));
        return;
    }

    println!("{}", get_message(&messages, "StartingCleanup"));

    // ホームディレクトリが不明な場合、誤った場所を操作しないよう終了する。
    let Some(home_dir) = env::var_os("HOME").map(PathBuf::from) else {
        eprintln!("Could not determine the user's home directory (HOME is not set). ");
        return;
    };

    // bash 履歴を先に処理し、その後ホーム直下の対象ログを削除する。
    clean_bash_history(&home_dir, &app_dir, &messages);
    delete_log_files(&home_dir, &messages);
}

// 実行ファイルの隣を優先し、そこに messages.ini がなければ Cargo のプロジェクトルートを使う。
fn application_directory() -> PathBuf {
    // 実行ファイルの場所を取得できない場合も、後段のフォールバックで処理する。
    let executable_dir = env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));

    // messages.ini が実際にあるディレクトリだけを設定ファイルの配置先として採用する。
    if let Some(directory) = executable_dir
        && directory.join(MESSAGES_FILE).exists()
    {
        return directory;
    }

    // cargo run では実行ファイルと設定ファイルの場所が異なるため、ビルド時のプロジェクトルートを使う。
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

// 最初の区切り行までの履歴を保持し、履歴ファイルがなければ既定内容で作成する。
fn clean_bash_history(home_dir: &Path, app_dir: &Path, messages: &HashMap<String, String>) {
    let history_path = home_dir.join(".bash_history");

    // 既存履歴を読み込めた場合は区切りの有無に応じて処理する。
    match fs::read_to_string(&history_path) {
        Ok(contents) => {
            let lines: Vec<&str> = contents.lines().collect();
            // 区切り文字を含む行自身は残し、その後のコマンドだけを取り除く。
            if let Some(index) = lines
                .iter()
                .position(|line| line.contains(BASH_HISTORY_DELIMITER))
            {
                let cleaned = lines[..=index].join("\n");
                // 書き換え時に元ファイル末尾の改行有無を変えないようにする。
                let cleaned = if contents.ends_with('\n') {
                    format!("{cleaned}\n")
                } else {
                    cleaned
                };
                match fs::write(&history_path, cleaned) {
                    Ok(()) => println!("{}", get_message(messages, "BashHistoryCleaned")),
                    Err(error) => print_history_error(messages, &error),
                }
            } else {
                // 区切りがない履歴は内容を変更しない。
                println!("{}", get_message(messages, "BashHistoryNotFound"));
            }
        }
        // 履歴が存在しない場合だけ、既定履歴から新しいファイルを作成する。
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            match load_default_history(app_dir) {
                Ok(Some(contents)) if !contents.is_empty() => {
                    match fs::write(&history_path, contents) {
                        Ok(()) => println!("{}", get_message(messages, "BashHistoryCreated")),
                        Err(error) => print_history_error(messages, &error),
                    }
                }
                // ファイル未配置または空ファイルなら履歴を作らず、読み込み失敗を通知する。
                Ok(_) => println!("{}", get_message(messages, "DefaultHistoryLoadFailed")),
                Err(error) => print_history_error(messages, &error),
            }
        }
        // 権限不足など、ファイル不存在以外の読み込みエラーを表示する。
        Err(error) => print_history_error(messages, &error),
    }
}

// 設定ファイルと同じディレクトリから既定履歴を読み込み、未配置は None として区別する。
fn load_default_history(app_dir: &Path) -> io::Result<Option<String>> {
    let history_path = app_dir.join(DEFAULT_HISTORY_FILE);
    match fs::read_to_string(history_path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

// 履歴の読み書きエラーを、messages.ini の {0} に埋め込んで表示する。
fn print_history_error(messages: &HashMap<String, String>, error: &io::Error) {
    println!(
        "{}",
        format_message(
            &get_message(messages, "BashHistoryError"),
            &[&error.to_string()]
        )
    );
}

// ホームディレクトリ直下を走査し、server-*.log に一致するファイルだけを削除する。
fn delete_log_files(home_dir: &Path, messages: &HashMap<String, String>) {
    // サブディレクトリには降りず、ホーム直下の項目だけを列挙する。
    let entries = match fs::read_dir(home_dir) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!(
                "Could not read home directory {}: {error}",
                home_dir.display()
            );
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                eprintln!(
                    "Could not inspect an entry in {}: {error}",
                    home_dir.display()
                );
                continue;
            }
        };
        let path = entry.path();
        // UTF-8 に変換できないファイル名は対象パターンと比較できないため対象外にする。
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !is_server_log(file_name) {
            continue;
        }

        // 削除の成否を個別に報告し、失敗しても残りのログ処理を続ける。
        match fs::remove_file(&path) {
            Ok(()) => println!(
                "{}",
                format_message(&get_message(messages, "LogFileDeleted"), &[file_name])
            ),
            Err(error) => println!(
                "{}",
                format_message(
                    &get_message(messages, "LogFileDeleteFailed"),
                    &[file_name, &error.to_string()]
                )
            ),
        }
    }
}

// 削除対象を server- で始まり .log で終わるファイル名に限定する。
fn is_server_log(file_name: &str) -> bool {
    file_name.starts_with("server-") && file_name.ends_with(".log")
}

// messages.ini を読み込み、解析したメッセージキーと値の辞書を返す。
fn load_messages(app_dir: &Path) -> HashMap<String, String> {
    let ini_path = app_dir.join(MESSAGES_FILE);
    // 設定ファイルが読めない場合は空の辞書で続行し、キー取得時に代替文を使う。
    let contents = match fs::read_to_string(&ini_path) {
        Ok(contents) => contents,
        Err(error) => {
            println!("{}", get_message(&HashMap::new(), "IniNotFound"));
            if error.kind() != io::ErrorKind::NotFound {
                eprintln!("Could not read {}: {error}", ini_path.display());
            }
            return HashMap::new();
        }
    };

    parse_messages(&contents)
}

// BOM を除去してから Messages セクションだけを解析し、空行とコメントを無視する。
fn parse_messages(contents: &str) -> HashMap<String, String> {
    let mut messages = HashMap::new();
    let mut in_messages_section = false;

    for line in contents.trim_start_matches('\u{feff}').lines() {
        let line = line.trim();
        // 行頭・行末の空白を除いてから空行やコメントを判定する。
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            // セクションが切り替わるたびに対象セクションかどうかを更新する。
            in_messages_section = line[1..line.len() - 1]
                .trim()
                .eq_ignore_ascii_case("Messages");
            continue;
        }

        // Messages セクション内では最初の '=' だけで分割し、値中の '=' は保持する。
        if in_messages_section && let Some((key, value)) = line.split_once('=') {
            messages.insert(key.trim().to_owned(), value.trim().to_owned());
        }
    }

    messages
}

// 指定されたメッセージを返し、未定義キーは調査しやすい代替文にする。
fn get_message(messages: &HashMap<String, String>, key: &str) -> String {
    messages
        .get(key)
        .cloned()
        .unwrap_or_else(|| format!("Message not found: {key}"))
}

// メッセージ内の {0}, {1} などの番号付きプレースホルダーを順に置換する。
fn format_message(template: &str, arguments: &[&str]) -> String {
    arguments
        .iter()
        .enumerate()
        .fold(template.to_owned(), |message, (index, value)| {
            message.replace(&format!("{{{index}}}"), value)
        })
}

// BOM 付き設定の解析、ログ名の判定、メッセージ置換をユニットテストで確認する。
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_messages_section_and_splits_on_first_equals() {
        let parsed = parse_messages(
            "\u{feff}; comment\n[Other]\nignored=value\n  [  Messages  ]  \nGreeting=hello=world\n# ignored\n",
        );

        assert_eq!(
            parsed.get("Greeting").map(String::as_str),
            Some("hello=world")
        );
        assert!(!parsed.contains_key("ignored"));
    }

    #[test]
    fn matches_only_server_log_names() {
        assert!(is_server_log("server-app.log"));
        assert!(is_server_log("server-.log"));
        assert!(!is_server_log("server-app.txt"));
        assert!(!is_server_log("my-server-app.log"));
    }

    #[test]
    fn formats_message_arguments() {
        assert_eq!(
            format_message("{0}: {1}", &["file.log", "denied"]),
            "file.log: denied"
        );
    }
}
