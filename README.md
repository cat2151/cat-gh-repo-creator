# cat-gh-repo-creator

A Windows application written in Rust that detects local un-git-managed Rust projects and interactively guides you through the process of publishing them to GitHub using a TUI.

---

## Challenges and Solutions
- The Problem So Far
    - I want to easily publish Rust projects generated locally by a Coding agent to GitHub.
    - While it can be done manually, I want to make it easier with a tool.
- This Tool's Solution
    - Solved with easy operations on a TUI!

## Requirements

- Windows & Rust
- git & [gh command](https://cli.github.com/) (ensure `gh auth login` has been completed beforehand)

---

## Installation

Rust is required.

```
cargo install --force --git https://github.com/cat2151/cat-gh-repo-creator
```

## Initial Execution

```
cat-gh-repo-creator
```

A `config.toml` file will be automatically generated, and the application will exit. Please configure the following settings:

---

## Configuration

Upon first launch, `config.toml` will be automatically generated at the following path:

```
%LOCALAPPDATA%\cat-gh-repo-creator\config.toml
```

**Please edit `scan_directory` in `config.toml` to match your environment (an error will occur if not edited)**

```toml
scan_directory = "C:\\Users\\<YOUR NAME>\\repos"
```

## Run

```
cat-gh-repo-creator
```

## Help

```sh
cat-gh-repo-creator --help
```

---

## Workflow

```
[DirList]      Scan result list. Move with j/k, select with ENTER
    ↓ ENTER
[RepoInspect]  Analysis result (OK/NG) and internal repository tree display
    ↓ ENTER (only when OK)
[CopyDialog]   Confirm copy candidate files found from nearby repositories  y / [N]
    ↓ y
[CopyResult]   Tree display after copying
    ↓ ENTER
[FetchFiles]   Curl .gitignore / LICENSE
    ↓ Completed
[FetchResult]  Display fetch results
    ↓ Automatic
[CreateDialog] Confirm settings for git init to gh repo create  y / [N]
    ↓ y
[Executing]    Sequentially executes git init / add / commit / branch -M main / gh repo create, displaying a spinner every 250ms from the start, along with "Please wait, processing..."
    ↓ Completed
[Done]         Automatically opens the repository page in the browser → ENTER to exit
```

If analysis is NG, or if 'N' is selected in a dialog, the application transitions to an interruption dialog, and pressing ENTER will terminate the application.

---

## Key Operations

| Key | Active Screen | Action |
|-----|---------------|--------|
| `j` / `↓` | DirList | Cursor down (moves only to target dirs) |
| `k` / `↑` | DirList | Cursor up (moves only to target dirs) |
| `q` | All screens | Exit application |
| `ENTER` | All screens | Confirm / Proceed to next step |
| `y` | CopyDialog / CreateDialog | Yes (Execute) |
| `n` / `N` / `ENTER` | CopyDialog / CreateDialog | No (To interruption dialog) |

---

## Directory Filter

Directories directly under `scan_directory` are listed in descending order by creation date, and those that **meet both of the following conditions** are considered targets:

- No `.git/` directory exists
- A `Cargo.toml` file exists

On the TUI, targets are displayed in white, and non-targets in gray. The cursor moves only to target directories.

---

## File Copy

- Repositories under `scan_directory` that contain `.git/` are scanned as "nearby repositories".
- For each file configured in `copy_files` in the TOML, the **most recently modified version** is selected from all nearby repositories and copied. The destination is the root of the target directory selected with ENTER.
- After copying, `_config.yml` automatically rewrites the following lines:

| Target for Rewrite    | Transformed Content       |
|-----------------------|---------------------------|
| `repository: owner/OLD_NAME` | `repository: owner/<NEW_REPO_NAME>` |
| `baseurl: /OLD_NAME`     | `baseurl: /<NEW_REPO_NAME>`    |

---

## Commands Executed by `gh repo create`

```sh
git init
git add .
git commit -m "Initial commit (generated via <chosen_source>)"  # Select from Claude chat UI / AI / Free input in the Message field
git branch -M main
gh repo create <repository_name> --public --source=. --remote=origin --push --disable-wiki
```

Upon completion, `https://github.com/<repository_name>` will automatically open in your browser.

## Assumptions
- This application is intended for my personal use and is not designed for others. If you desire similar functionality, I recommend cloning the repository or building your own.
- Frequent breaking changes will be made.

## Project Goals
- PoC. To demonstrate that a useful personal application can be created using free Claude chat (demonstrated).

## What This Application Does Not Aim For (Out of Scope)
- Support. Responding to requests or suggestions.