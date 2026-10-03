# hisclear

Linux 向けのシェル履歴・ログ整理ツールです。`~/.bash_history` の一部を整理し、ホームディレクトリ直下にある `server-*.log` ファイルを削除します。

## 注意事項

このプログラムは `.bash_history` を上書きし、該当するログファイルを削除します。削除した内容は復元できないため、必要なデータを事前にバックアップしてください。`cargo run` で実行する場合も、実際のユーザーのホームディレクトリが処理対象になります。

Linux 以外ではクリーンアップ処理を行いません。

## 動作

### bash 履歴

- ホームディレクトリの `.bash_history` を読み込み、最初に `###` を含む行を探します。
- 区切り行が見つかった場合、その行までは残し、それより後の行を削除します。区切り行自体は残ります。
- `###` が見つからない場合はファイルを変更しません。
- `.bash_history` がない場合は、`default_history.txt` の内容から新規作成します。ファイルがない、または空の場合は作成しません。

### ログファイル

- ホームディレクトリ直下にある、名前が `server-` で始まり `.log` で終わるファイルを削除します。
- サブディレクトリ内のファイルや、名前のパターンに合わないファイルは対象外です。

## 必要な環境

- Rust と Cargo（Rust Edition 2024 対応）
- Linux（実際のクリーンアップ処理を行う場合）

外部クレートは使用していません。

## ビルド・テスト・実行

リポジトリのルートで実行します。

```bash
# テスト
cargo test

# リリース用バイナリをビルド
cargo build --release
```

開発実行は次のコマンドです。**実行すると実際の `$HOME/.bash_history` とホーム直下のログファイルが処理されるため、実行前に注意してください。**

```bash
cargo run
```

`cargo run` の場合、設定ファイルはプロジェクトルートから読み込まれます。ビルドしたバイナリを単独で配置する場合は、`messages.ini` と `default_history.txt` をバイナリと同じディレクトリに置いてください。

```bash
cp messages.ini default_history.txt target/release/
./target/release/hisclear
```

実行ファイルの隣に `messages.ini` があると、そのディレクトリを設定ファイルの場所として使用します。ない場合は Cargo のプロジェクトルートにフォールバックします。

## 設定ファイル

### `messages.ini`

`[Messages]` セクション内に `key=value` 形式でメッセージを記述します。セクション名は大文字・小文字を区別せず、セクション名の前後の空白も無視します。UTF-8 BOM 付きのファイルも読み込めます。空行と `;` または `#` で始まるコメント行は無視されます。

```ini
[Messages]
ProgramNotForLinux=このプログラムはLinux用です。
StartingCleanup=Starting cleanup...
BashHistoryCleaned=~/.bash_history の "###" の行以降の行を削除しました。
BashHistoryNotFound=~/.bash_history に "###" の行が見つかりませんでした。変更なし。
BashHistoryCreated=~/.bash_history が見つからないため、新しく作成しました。
BashHistoryError=~/.bash_history の処理中にエラーが発生しました: {0}
LogFileDeleted=削除しました: {0}
LogFileDeleteFailed=削除に失敗しました: {0} - {1}
IniNotFound=messages.ini が見つかりません。
DefaultHistoryLoadFailed=デフォルト履歴ファイルの読み込みに失敗しました。
```

メッセージ内の `{0}`、`{1}` は、ファイル名やエラー内容などに置き換えられます。未定義のキーは `Message not found: <key>` と表示されます。

### `default_history.txt`

`.bash_history` が存在しない場合に使う初期履歴です。各行にコマンドを記述し、保持範囲の目印として `###` を含む行を用意してください。内容は用途に合わせて編集できます。

## エイリアス設定例

ビルドした実行ファイルの場所に合わせてパスを変更し、`~/.bashrc` に追加します。

```bash
alias hc="$HOME/path/to/hisclear"
```

設定を反映するには、次を実行します。

```bash
source ~/.bashrc
```

## ファイル構成

```text
hisclear/
├── Cargo.toml
├── Cargo.lock
├── src/
│   └── main.rs
├── messages.ini
├── default_history.txt
└── README.md
```

## C# から Rust への移行について

このプロジェクトは当初 C#（.NET）で実装されていました。既存の機能を保ちながら Rust に書き換える作業を、Zed エディター上で GPT-6 Luna を利用して行いました。Rust 版では Cargo の標準的な構成に移行し、外部クレートを使わずに設定ファイルの解析、履歴の整理、ログの削除を実装しています。移行後は `cargo test` で基本動作を確認しています。
