# cat-gh-repo-creator

A Windows application written in Rust that detects local, unmanaged Rust projects and interactively guides you through the process of publishing them to GitHub via a TUI.

---

## Problem and Solution
- Previous Problem
    - I want to easily publish Rust projects generated locally by a coding agent to GitHub.
    - While it can be done manually, I want a tool to make it even easier.
- This Tool's Solution
    - Solved with easy operations in a TUI!

## Operating Environment

- Windows & Rust
- git & [gh CLI](https://cli.github.com/) ( `gh auth login` must be completed beforehand)

---

## Installation

Rust is required.

```
cargo install --force --git https://github.com/cat2151/cat-gh-repo-creator
```

## Execution

```
cat-gh-repo-creator
```

---

## Configuration

On first launch, `config.toml` is automatically generated at the following path:

```
%LOCALAPPDATA%\cat-gh-repo-creator\config.toml
```

**Please edit `scan_directory` to match your environment (will cause an error if not edited)**

```toml
scan_directory = "C:\\Users\\<YOUR NAME>\\repos"
commit_message = "Initial commit (generated via Claude chat UI)"
gitignore_template = "Rust"
license = "mit"
log_file = "cat-gh-repo-creator.log"

copy_files = [
  ".github/workflows/call-check-large-files.yml",
  ".github/workflows/call-issue-note.yml",
  ".github/workflows/call-translate-readme.yml",
  "_config.yml",
]
```

---

## Workflow

```
[DirList]      List of scan results. Move with j/k, select with ENTER
    ↓ ENTER
[RepoInspect]  Analysis results (OK/NG) and internal repository tree display
    ↓ ENTER (only when OK)
[CopyDialog]   Confirmation of candidate files found from nearby repos to copy  y / [N]
    ↓ y
[CopyResult]   Tree display after copying
    ↓ ENTER
[FetchFiles]   curl .gitignore / LICENSE
    ↓ complete
[FetchResult]  Show fetched files
    ↓ auto
[CreateDialog] Confirmation of git init ~ gh repo create settings  y / [N]
    ↓ y
[Executing]    Execute git init / add / commit / branch -M main / gh repo create sequentially
    ↓ Done
[Done]         Automatically open the repository page in the browser → Press ENTER to exit
```

If the analysis result is NG, or if 'N' is selected in a dialog, the app will transition to an interruption dialog and then exit by pressing ENTER.

---

## Key Operations

| Key | Active Screen | Action |
|------|-----------|------|
| `j` / `↓` | DirList | Move cursor down (moves only within target dirs) |
| `k` / `↑` | DirList | Move cursor up (moves only within target dirs) |
| `q` | All Screens | Exit application |
| `ENTER` | All Screens | Confirm / Next step |
| `y` | CopyDialog / CreateDialog | Yes (execute) |
| `n` / `N` / `ENTER` | CopyDialog / CreateDialog | No (to interruption dialog) |

---

## Directory Filtering

Directories directly under `scan_directory` are listed in descending order of creation date, and those that meet **both of the following conditions** are considered targets:

- `.git/` does not exist
- `Cargo.toml` exists

In the TUI, targets are displayed in white and non-targets in gray. The cursor only moves within target directories.

---

## File Copying

- Repositories with `.git/` under `scan_directory` are scanned as "nearby repositories".
- For each file configured in `copy_files` in the toml, the **most recently modified** file is selected from all nearby repositories and copied. The destination is directly under the target directory selected with ENTER.
- `_config.yml` automatically rewrites the following lines after copying:

| Target for Rewrite | Transformed Content |
|--------------------|---------------------|
| `repository: owner/old_name` | `repository: owner/<new_repo_name>` |
| `baseurl: /old_name` | `baseurl: /<new_repo_name>` |

---

## Commands Executed by gh repo create

```sh
git init
git add .
git commit -m "Initial commit (generated via Claude chat UI)"
git branch -M main
gh repo create <repository_name> --public --source=. --remote=origin --push --disable-wiki
```

After completion, `https://github.com/<repository_name>` will automatically open in your browser.

## Assumptions
- This application is intended for my personal use, and is not designed for others. If you need similar functionality, I recommend cloning or developing your own.
- Breaking changes will be made frequently.

## Goals of this Application
- PoC. To demonstrate (and has demonstrated) that useful personal applications can be created using Claude's free chat.

## Out of Scope
- Support. Responding to requests or suggestions.
