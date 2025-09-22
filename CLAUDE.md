# Project Structure & Guidelines

This is a Cargo workspace with independent binary packages.

## Packages

- **ai**: Binary that expects a single string argument: `cargo run -- <query>`

## Development Workflow

### Running Commands

1. **Always `cd` into the specific package directory first** before running
   cargo commands
2. **Run binary**: Use `cargo run` (or `cargo run -- <args>` for ai package)

## 🚨 MANDATORY POST-TASK CHECKLIST 🚨

**CRITICAL**: After completing ANY task, you MUST execute these steps IN ORDER:

### Step 1: Code Quality

**Note**: Running cargo only applies to Rust projects.

```bash
cargo fmt         # Format code
cargo clippy      # Fix ALL warnings before proceeding
```

### Step 2: Translation Updates

**Note**: gettext/extract_strings.sh needs to be run from the project root.

```bash
gettext/extract_strings.sh    # Extract new translatable strings
git diff gettext/po           # Review translation changes
```

### Step 3: Translation Cleanup

1. Edit translation files in `gettext/po/`
2. Remove all `#, fuzzy` comments
3. Remove commented message strings
4. Run `gettext/extract_strings.sh` again to reformat po files

**NO EXCEPTIONS**: These steps are mandatory for every code change, no matter
how small.

## Git Guidelines

- **NEVER** include "Generated with Claude Code" lines in commit messages
