<div align="center">

# epitech-style-checker

**A native, Docker-free reimplementation of Epitech's Banana coding style checker — written in Rust.**

[![CI](https://github.com/daniilsys/epitech-style-checker/actions/workflows/ci.yml/badge.svg)](https://github.com/daniilsys/epitech-style-checker/actions/workflows/ci.yml)
[![Release](https://github.com/daniilsys/epitech-style-checker/actions/workflows/release.yml/badge.svg)](https://github.com/daniilsys/epitech-style-checker/actions/workflows/release.yml)
![Rust](https://img.shields.io/badge/rust-2024-orange?logo=rust)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

</div>

---

## Why this exists

If you've ever waited on `coding-style.sh` to pull a multi-hundred-megabyte Docker image just to find out
you forgot a space before a semicolon, you already know the problem.

Epitech's official checker — nicknamed **Banana** — runs `lambdananas` and a set of Python `vera++` rules
inside a Docker container. It works, but it's slow to start, occasionally crashes
(`Segmentation fault`, yes, really), and gives you no feedback while you're actually writing code.

**`epitech-style-checker`** re-implements the same rule set natively in Rust, using [`tree-sitter`](https://tree-sitter.github.io/tree-sitter/)
for real C parsing. No Docker, no Python, no network — just a single static binary that runs instantly and
can be wired into your editor, a pre-commit hook, or a shell alias.

I'm an Epitech student myself, and I built this to actually understand the coding style rules well enough
to automate them — and to stop losing ten seconds to Docker every time I want to check a two-line fix.

<details>
<summary><strong>⚠️ A friendly disclaimer</strong></summary>
<br>

This tool is a **best-effort reimplementation**, not an official Epitech product. Some rules (like
"a function should do one thing" or "structures should be kept small") are inherently subjective and are
**intentionally skipped** — see [below](#not-implemented-on-purpose). Others (like precise-typing or
static-usage suggestions) rely on heuristics that favor **fewer false positives over perfect recall**.

**Always run the official Banana checker before a graded submission.** Use this tool as a fast local
feedback loop, not as a replacement for the real evaluation.

</details>

---

## Features

<table>
<tr>
<td width="33%" valign="top">

### 🚀 Fast
A single native binary. No Docker daemon, no image pulls, no waiting.

</td>
<td width="33%" valign="top">

### 🌳 AST-aware
Built on `tree-sitter-c` for structural rules (function length, nesting, brace placement...),
with regex only where a real parse tree would be overkill.

</td>
<td width="33%" valign="top">

### 📦 Portable
Prebuilt binaries for Linux, macOS, and Windows on every [release](../../releases) — just download and run.

</td>
</tr>
</table>

---

## Installation

### Option 1 — download a prebuilt binary

Grab the archive for your OS from the [latest release](../../releases/latest), extract it, and drop the
binary somewhere on your `PATH`.

```bash
# example: macOS / Linux
tar xzf epitech-style-checker-*.tar.gz
mv epitech-style-checker ~/my_scripts/   # or anywhere in your $PATH
chmod +x ~/my_scripts/epitech-style-checker
```

### Option 2 — build from source

```bash
git clone https://github.com/daniilsys/epitech-style-checker.git
cd epitech-style-checker
cargo build --release
# binary at target/release/epitech-style-checker
```

### Set up an alias

```bash
# ~/.zshrc or ~/.bashrc
alias banana="epitech-style-checker"
```

---

## Usage

```bash
# check the current directory, recursively
epitech-style-checker .

# check a specific file or folder
epitech-style-checker path/to/project
```

```text
./src/main.c: 1:C-G1: Minor:Missing or invalid Epitech header
./src/main.c: 42:C-F4: Major:Function exceeds 20 lines (23)
./include/my.h: 1:C-H2: Major:Header must be protected by include guard

3 error(s) found
```

Each line follows the same shape as Banana's own output: **file, line, rule code, severity, message.**

---

## Rule coverage

<div align="center">

**40 / 43** rules implemented — every rule that can be checked mechanically and objectively.

</div>

<details>
<summary><strong>C-A — Advanced</strong> (4/4)</summary>
<br>

| Rule | Description |
|------|-------------|
| `C-A1` | Pointer parameters that never mutate their pointee should be `const` |
| `C-A2` | Prefer precise types (`unsigned int` for counters, `size_t` for sizes) |
| `C-A3` | Files must end with a line break |
| `C-A4` | Functions/globals unused outside their file should be `static` |

</details>

<details>
<summary><strong>C-C — Control structures</strong> (3/3)</summary>
<br>

| Rule | Description |
|------|-------------|
| `C-C1` | No more than 3 branches / nested conditionals of depth ≥ 3 |
| `C-C2` | No nested/chained ternaries; the result must always be used |
| `C-C3` | `goto` is forbidden |

</details>

<details>
<summary><strong>C-F — Functions</strong> (8/9)</summary>
<br>

| Rule | Description |
|------|-------------|
| `C-F2` | Function names in `snake_case`, must contain a verb |
| `C-F3` | Lines must not exceed 80 columns |
| `C-F4` | Function body must not exceed 20 lines |
| `C-F5` | No more than 4 parameters |
| `C-F6` | Parameterless functions must take `void` |
| `C-F7` | Structures passed by pointer, never by copy |
| `C-F8` | No comments inside a function body |
| `C-F9` | No nested functions |

*`C-F1` (single-responsibility) is skipped — see [below](#not-implemented-on-purpose).*

</details>

<details>
<summary><strong>C-G — Global scope</strong> (9/10)</summary>
<br>

| Rule | Description |
|------|-------------|
| `C-G1` | Standard Epitech file header |
| `C-G2` | Functions separated by exactly one empty line |
| `C-G3` | Preprocessor directives indented by nesting level |
| `C-G4` | Global variables must be `const` |
| `C-G5` | `#include` directives must target `.h` files |
| `C-G6` | Unix line endings, no trailing backslash |
| `C-G7` | No trailing whitespace |
| `C-G8` | No leading blank lines, at most one trailing blank line |
| `C-G10` | No inline assembly |

*`C-G9` (non-trivial constants should be named) is skipped — see [below](#not-implemented-on-purpose).*

</details>

<details>
<summary><strong>C-H — Header files</strong> (3/3)</summary>
<br>

| Rule | Description |
|------|-------------|
| `C-H1` | Prototypes/typedefs/etc. belong in headers, not source files |
| `C-H2` | Include guards (`#ifndef`/`#define`/`#endif`) |
| `C-H3` | Macros must fit on a single statement, single line |

</details>

<details>
<summary><strong>C-L — Layout inside a function</strong> (6/6)</summary>
<br>

| Rule | Description |
|------|-------------|
| `C-L1` | One statement per line |
| `C-L2` | 4-space indentation, no tabs |
| `C-L3` | Consistent, minimal spacing around operators/keywords |
| `C-L4` | K&R brace placement |
| `C-L5` | One variable declaration per statement, declared up top |
| `C-L6` | Exactly one blank line between declarations and the rest |

</details>

<details>
<summary><strong>C-O — File organization</strong> (4/4)</summary>
<br>

| Rule | Description |
|------|-------------|
| `C-O1` | No compiled/temporary/unnecessary files in the repo |
| `C-O2` | Only `.c`/`.h` extensions for source files |
| `C-O3` | At most 10 functions per file (5 non-static) |
| `C-O4` | File/folder names in `snake_case`, descriptive |

</details>

<details>
<summary><strong>C-V — Variables and types</strong> (2/3)</summary>
<br>

| Rule | Description |
|------|-------------|
| `C-V1` | Identifiers in `snake_case`; typedefs end in `_t`; macros/enums `UPPER_SNAKE_CASE` |
| `C-V3` | Pointer `*` attached to the variable name, not the type |

*`C-V2` (structures should stay small/coherent) is skipped — see [below](#not-implemented-on-purpose).*

</details>

<details>
<summary><strong>C-Z — Miscellaneous</strong> (1/1)</summary>
<br>

| Rule | Description |
|------|-------------|
| `C-Z1` | No null bytes in source files |

</details>

### Not implemented on purpose

Three rules from the official coding style are **inherently subjective** and cannot be checked
mechanically without producing constant false positives:

- **`C-F1`** — a function should do "one thing" (single-responsibility principle)
- **`C-G9`** — "non-trivial" constant values should be named
- **`C-V2`** — structures should be "small" and group a "coherent" entity

These require human judgment. Use your own eyes — that's what Epitech's docs recommend too.

---

## How it works

```text
┌──────────────┐     ┌───────────────┐     ┌──────────────────┐
│  .c / .h     │ ──▶ │  tree-sitter  │ ──▶ │  per-rule checks  │
│  source file │     │  C parser     │     │  (src/rules/c_*)  │
└──────────────┘     └───────────────┘     └──────────────────┘
                                                     │
                                                     ▼
                                            ┌──────────────────┐
                                            │   diagnostics     │
                                            │ file:line:code    │
                                            └──────────────────┘
```

Structural rules (function length, brace placement, nesting depth...) walk the `tree-sitter-c` AST.
Formatting rules (spacing, line length, trailing whitespace...) operate directly on the source text via
regex, since a full parse buys nothing there. A dedicated cross-file pass powers `C-A4`, since "used
outside this file" requires seeing the whole project at once, not just one file.

Every rule lives in `src/rules/c_<letter>.rs`, with unit tests covering both a triggering and a
non-triggering case.

---

## Development

```bash
cargo build            # debug build
cargo test              # run the test suite
cargo build --release   # optimized binary
```

Pull requests welcome — especially bug reports with a minimal `.c` snippet that reproduces a false
positive or false negative.

---

<div align="center">

Built by an Epitech student, for Epitech students. 🍌

</div>
