# cat-gh-repo-creator

ローカルの未git管理Rustプロジェクトを検出し、GitHub公開までの一連の手順をTUI上でインタラクティブに進めるWindows向けアプリ。Rustで書かれています。

---

## 課題と解決
- これまでの課題
    - localでCoding agentに生成させたRustプロジェクトを、GitHubに楽に公開したい
    - 手動でもできますが、ツールでもっと楽をしたい
- このツールの解決
    - TUIでらくらく操作で解決！

## 動作環境

- Windows & Rust
- git & [ghコマンド](https://cli.github.com/) （事前に `gh auth login` 済みであること）

---

## インストール

Rustが必要です。

```
cargo install --force --git https://github.com/cat2151/cat-gh-repo-creator
```

## 初回実行

```
cat-gh-repo-creator
```

config.tomlを自動生成して終了となりますので、以下の設定をしてください

---

## 設定

初回起動時、以下のパスに `config.toml` を自動生成します

```
%LOCALAPPDATA%\cat-gh-repo-creator\config.toml
```

**`config.toml` の `scan_directory` を自分の環境に合わせて編集してください（編集しないとエラー）**

```toml
scan_directory = "C:\\Users\\<YOUR NAME>\\repos"
```

## 実行

```
cat-gh-repo-creator
```

---

## 作業フロー

```
[DirList]      スキャン結果一覧。j/kで移動、ENTERで選択
    ↓ ENTER
[RepoInspect]  分析結果（OK/NG）とリポジトリ内部のツリー表示
    ↓ ENTER（OK時のみ）
[CopyDialog]   近隣repoから見つかったコピー候補ファイルの確認  y / [N]
    ↓ y
[CopyResult]   コピー後のツリー表示
    ↓ ENTER
[FetchFiles]   .gitignore / LICENSE を curl
    ↓ 完了
[FetchResult]  取得結果を表示
    ↓ 自動
[CreateDialog] git init〜gh repo create の設定確認  y / [N]
    ↓ y
[Executing]    git init / add / commit / branch -M main / gh repo create を順次実行
    ↓ 完了
[Done]         ブラウザでリポジトリページを自動で開く → ENTERで終了
```

分析NGの場合、またはダイアログでNを選んだ場合は中断ダイアログへ遷移し、ENTERでアプリ終了となります

---

## キー操作

| キー | 有効な画面 | 動作 |
|------|-----------|------|
| `j` / `↓` | DirList | カーソル下（target dirのみに移動） |
| `k` / `↑` | DirList | カーソル上（target dirのみに移動） |
| `q` | 全画面 | アプリ終了 |
| `ENTER` | 全画面 | 決定・次のステップへ |
| `y` | CopyDialog / CreateDialog | Yes（実行） |
| `n` / `N` / `ENTER` | CopyDialog / CreateDialog | No（中断ダイアログへ） |

---

## ディレクトリフィルタ

`scan_directory` 直下のディレクトリを作成日降順で列挙し、以下の**両条件を満たす**ものをtargetとします

- `.git/` が存在しないこと
- `Cargo.toml` が存在すること

TUI上では target を白、非targetをグレーで表示します。カーソルはtargetのみに移動します

---

## ファイルコピー

- `scan_directory` 配下の `.git/` ありリポジトリを「近隣リポジトリ」としてscanします
- tomlの `copy_files` で設定したファイルごとに、近隣リポジトリ全体から**最終更新日時が最新のもの**を選択してコピーします。コピー先はENTERで選んだtargetディレクトリの直下です
- `_config.yml` は、コピー後に以下の行を自動書き換えします

| 書き換え対象 | 変換内容 |
|------------|---------|
| `repository: owner/旧名` | `repository: owner/<新repo名>` |
| `baseurl: /旧名` | `baseurl: /<新repo名>` |

---

## gh repo create で実行されるコマンド

```sh
git init
git add .
git commit -m "Initial commit (generated via Claude chat UI)"
git branch -M main
gh repo create <リポジトリ名> --public --source=. --remote=origin --push --disable-wiki
```

完了後、ブラウザで `https://github.com/<リポジトリ名>` を自動で開きます

## 前提
- 自分用のアプリですので、他の人が使うことを想定していません。似たような機能がほしいときはcloneや自作をおすすめします。
- 頻繁に破壊的変更を行います。

## このアプリが目指すもの
- PoC。Claude無料chatで自分用にあると助かるアプリが作れることを実証する（実証した）

## 目指さないもの（スコープ外）
- サポート。要望や提案に応える
