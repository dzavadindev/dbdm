<p align="center">
    <img  style="transform: scale(1.4)" src="./media/dbdm-logo.svg"/>
</p>

# DBDM - Dan's Boring Dotfiles Manager

A simple utility to match the system state to the state defined in the config file, by creating symlinks to the appropriate config directories.

## Overview

DBDM (Dan's Boring Dotfiles Manager) is a small Rust CLI that keeps your
system's dotfiles in sync with a declarative config. It reads a `dbdm.conf`
in the current directory, validates its links, and then creates or fixes
symlinks so real config files point to the sources you define.


Some main functionalities is to check whether each link already points to the intended target, sync by replacing missing/empty targets, or prompting when a conflict exists, to back up conflicting files/dirs before replacement. 

And optional `--force` mode to skip prompts and replace immediately.

## Installing

Build and install locally from the repo:

```sh
cargo install --path .
```

Cargo bin path must be in your $PATH though

## Usage

Run `dbdm` from the directory containing `dbdm.conf`:

```sh
dbdm check
dbdm sync
```

Flags:
- `--force`: replace conflicting targets without prompting.

Commands:
- `check` prints green links when targets match, red when they don't.
- `sync` prints a plan, previews conflicts, and asks how to resolve them:
  - replace, backup+replace, or skip.

## Config Definition 

DBDM expects a `dbdm.conf` in the current directory. Each line declares a link:

```
link = <from> <to>
```

Those links must be full paths, including the name of the link to be made. Additionally, its possible to use keywords that are expanded during parsing from the environment variables of the user running the util.

Example using keywords:

```
link = !here/nvim !xdg_conf/nvim
link = !here/.gitconfig !home/.gitconfig
```

Supported keywords are:
- `!here` -> current working directory
- `!home` -> `$HOME`
- `!xdg_conf` -> `$XDG_CONFIG_HOME` (or `~/.config` if unset)

## Notes

When you choose backup, DBDM moves the existing `<to>` into a `.bak.dbdm` path and then creates the symlink. Backups are placed next to the source (or its parent for files), with numeric suffixes if needed, e.g. `nvim.bak.dbdm`, `nvim.bak.dbdm.1`.
