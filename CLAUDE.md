# Project Structure & Guidelines

This is a Cargo workspace with independent binary packages.

## Packages

- **ai**: Binary that expects a single string argument: `cargo run -- <query>`

## Development Workflow

### Running Commands

1. **Always `cd` into the specific package directory first** before running cargo commands
2. **Run binary**: Use `cargo run` (or `cargo run -- <args>` for ai package)

### Code Quality Pipeline

After completing any task, run these commands in order:

1. `cargo fmt` - Format code
2. `cargo clippy` - Fix all warnings
3. Update translations (see Translation Workflow below)

## Translation Workflow

This project uses gettext for internationalization. After completing a task:

1. **Extract strings**: Run `gettext/extract_strings.sh` from root directory
2. **Check changes**: Run `git diff gettext/po` to see what translations need updating
3. **Update translations**: Edit translation files in `gettext/po/`
4. **Clean up**: Remove all `#, fuzzy` comments and newly commented message strings
5. **Reformat**: Run `gettext/extract_strings.sh` again to reformat po files

## Git Guidelines

- **NEVER** include "Generated with Claude Code" lines in commit messages
