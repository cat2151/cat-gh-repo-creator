# cat-gh-repo-creator

A Windows application written in Rust that detects local, un-git-managed Rust projects and interactively guides you through the process of publishing them to GitHub using a TUI.

---

## Challenges and Solutions
- Previous Challenges
    - I want to easily publish Rust projects generated locally by a coding agent to GitHub.
    - It can be done manually, but I'd prefer an easier way with a tool.
- This Tool's Solution
    - Solves the problem with easy TUI operations!

## System Requirements

- Windows & Rust
- git & [gh CLI](https://cli.github.com/) (Must be `gh auth login`'d beforehand)

---

## Installation

Rust is required.

```
cargo install --force --git https://github.com/cat2151/cat-gh-repo-creator
```

## First Run

```
cat-gh-repo-creator
```

`config.toml` will be automatically generated, and the application will exit. Please configure the settings below.

---

## Configuration

Upon first launch, `config.toml` will be automatically generated at the following path:

```
%LOCALAPPDATA%\cat-gh-repo-creator\config.toml
```

**Edit `scan_directory` in `config.toml` to match your environment (an error will occur if not edited).**

```toml
scan_directory = "C:\\Users\\<YOUR NAME>\\repos"
```

## Execution

```
cat-gh-repo-creator
```

## Help

```sh
cat-gh-repo-creator --help
```

## Checking for Updates

```sh
cat-gh-repo-creator check
```

This compares the commit hash at build time with the latest commit on the `main` branch of GitHub. It displays the build-time hash as `embedded`, the latest remote hash as `remote`, and the `result` as `up-to-date` (matches) or `update available` (differences found).

## Update

```sh
cat-gh-repo-creator update
```

This command executes `cargo install --force --git https://github.com/cat2151/cat-gh-repo-creator` in a separate process and then terminates the main application. After the update is complete, please manually restart `cat-gh-repo-creator`. Updating requires Rust, Git, and Python in your PATH. If the update command itself does not work, please re-run the installation command.

---

## Workflow

```
[DirList]      Scan results list. Move with j/k, select with ENTER.
    ↓ ENTER
[RepoInspect]  Analysis results (OK/NG) and internal repository tree display.
    ↓ ENTER (only if OK)
[CopyDialog]   Confirm copy candidate files found from nearby repositories. y / [N]
    ↓ y
[CopyResult]   Tree display after copying.
    ↓ ENTER
[FetchFiles]   cURL .gitignore / LICENSE.
    ↓ Complete
[FetchResult]  Display fetch results.
    ↓ Automatic
[CreateDialog] Confirm git init to gh repo create settings. y / [N]
    ↓ y
[Executing]    Sequentially executes git init / add / commit / branch -M main / gh repo create, displaying a spinner and "Processing, please wait" every 250ms from the start.
    ↓ Complete
[Done]         Automatically opens the repository page in a browser → ENTER to exit.
```

If the analysis is NG, or if you choose N in a dialog, you will transition to an interruption dialog, and pressing ENTER will exit the application.

---

## Key Operations

| Key | Valid Screen | Action |
|-----|--------------|--------|
| `j` / `↓` | DirList | Cursor down (moves only to target dirs) |
| `k` / `↑` | DirList | Cursor up (moves only to target dirs) |
| `q` | All screens | Exit application |
| `ENTER` | All screens | Confirm / Next step |
| `y` | CopyDialog / CreateDialog | Yes (execute) |
| `n` / `N` / `ENTER` | CopyDialog / CreateDialog | No (to interruption dialog) |

---

## Directory Filter

Directories directly under `scan_directory` are listed in descending order of creation date. Only those that meet **both of the following conditions** are considered targets:

- `.git/` does not exist
- `Cargo.toml` exists

In the TUI, targets are displayed in white, and non-targets in gray. The cursor moves only between target directories.

---

## File Copy

- Repositories under `scan_directory` that contain `.git/` are scanned as "nearby repositories".
- For each file configured in `copy_files` in `config.toml`, the **most recently modified file** from all nearby repositories is selected and copied. The destination is directly under the target directory chosen with ENTER.
- For `_config.yml`, the following lines are automatically rewritten after copying:

| Target for rewrite | Conversion content |
|--------------------|--------------------|
| `repository: owner/old_name` | `repository: owner/<new_repo_name>` |
| `baseurl: /old_name` | `baseurl: /<new_repo_name>` |

---

## Commands executed by `gh repo create`

```sh
git init
git add .
git commit -m "Initial commit (generated via <chosen option>)"  # Choose from Claude chat UI / AI / Free input in the Message field
git branch -M main
gh repo create <repository_name> --public --source=. --remote=origin --push --disable-wiki
```

After completion, `https://github.com/<repository_name>` will automatically open in your browser.

## Assumptions
- This application is intended for my personal use, not for others. If you want similar functionality, I recommend cloning or creating your own.
- Destructive changes will be made frequently.

## What this app aims for
- PoC: To demonstrate (and has demonstrated) that useful personal applications can be created with Claude's free chat.

## What it does not aim for (Out of scope)
- Support: Responding to requests or suggestions.