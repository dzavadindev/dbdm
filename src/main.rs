use crate::config_parser::Config;
use dbdm::{backup_and_replace, canonicalize_or_fallback, replace_link, resolve_symlink_target};
use std::io::Read;
use std::path::{Path, PathBuf};

mod config_parser;

struct RunMode {
    test_mode: bool,
}

macro_rules! app_println {
    ($mode:expr, $($arg:tt)*) => {
        if !$mode.test_mode {
            println!($($arg)*);
        }
    };
}

macro_rules! app_print {
    ($mode:expr, $($arg:tt)*) => {
        if !$mode.test_mode {
            print!($($arg)*);
        }
    };
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = RunMode {
        test_mode: args.iter().any(|arg| arg == "--test-mode"),
    };
    let force = args.iter().any(|arg| arg == "--force");

    // Grab current dir
    let mut pwd = match std::env::current_dir() {
        Ok(path) => path,
        Err(err) => {
            app_println!(&mode, "Could not parse the {}", err.to_string());
            return;
        }
    };

    // Check for presence of dbdm.conf
    pwd.push("dbdm.conf");
    if !pwd.exists() {
        let mut path_str = pwd.clone();
        path_str.pop();
        app_println!(
            &mode,
            "dbdm.conf doesn exist in {}",
            path_str.to_str().expect("Can't parse dir path")
        );
        return;
    }

    // Parse the config
    let config = match config_parser::read_config(&pwd) {
        Ok(res) => res,
        Err(err) => {
            app_println!(&mode, "Error in config:\n\n{}", err);
            return;
        }
    };

    // Handle the command
    let command = args
        .iter()
        .find(|arg| !arg.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| String::from("help"));
    match command.as_str() {
        "check" => check(&config, &mode),
        "sync" => sync(&config, &mode, force),
        "help" => help(&mode),
        _ => help(&mode),
    }
}

// One of the command handlers
// Allows to check if the current state of the system matches
// the desired state that is specified in the provided config
//
// @param config: &Config - the parsed config state
fn check(config: &Config, mode: &RunMode) {
    for link in &config.links {
        // Get an absolute path to the files
        let from_full = std::fs::canonicalize(&link.from).unwrap_or_else(|_| link.from.clone());
        let to_full = std::fs::canonicalize(&link.to).unwrap_or_else(|_| link.to.clone());

        let is_match = match std::fs::read_link(&link.to) {
            Ok(target) => {
                let target_full = std::fs::canonicalize(&target).unwrap_or(target);
                target_full == from_full
            }
            Err(_) => false,
        };

        if is_match {
            app_println!(
                mode,
                "\x1b[32m{} -> {}\x1b[0m",
                from_full.display(),
                to_full.display()
            );
        } else {
            app_println!(
                mode,
                "\x1b[31m{} -> {}\x1b[0m",
                from_full.display(),
                to_full.display()
            );
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SyncAction {
    Ignore,
    Replace,
    BackupReplace,
    Skip,
    Pending, // Temp state to mark files that need to be acted upon
}

#[derive(Debug)]
struct PlanItem {
    from: PathBuf,
    to: PathBuf,
    action: SyncAction,
    reason: Option<String>,
}

// One of the command handlers
// Allows to perform a sync of system state to the desired state specified in the config.
//
// Accepts a `--force` flag if a non-interactive execution is preferred.
//
// Otherwise tires to sync the state described in the config with the system state
//
// @param config: &Config - the parsed config state
fn sync(config: &Config, mode: &RunMode, force: bool) {
    // The plan to be previewed and then executed
    let mut plan: Vec<PlanItem> = Vec::new();
    // To have a quicker lookup for which plan items require care
    let mut pending_indices: Vec<usize> = Vec::new();

    for link in &config.links {
        let from = link.from.clone();
        let to = link.to.clone();

        // Check if the path is valid and we have permission to modify it
        match std::fs::symlink_metadata(&to) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    // Try grab the file the link points to
                    let target = std::fs::read_link(&to).unwrap_or_else(|_| to.clone());

                    let from_full = canonicalize_or_fallback(&from);
                    let target_full =
                        canonicalize_or_fallback(&resolve_symlink_target(&to, &target));

                    // Update the plan with an IGNORE
                    if target_full == from_full {
                        plan.push(PlanItem {
                            from,
                            to,
                            action: SyncAction::Ignore,
                            reason: None,
                        });
                        continue;
                    }
                }

                let is_empty = is_empty_path(&to, &meta).unwrap_or(false);
                let is_conflict = !is_empty;

                // Account for the flag
                let action = if force || !is_conflict {
                    SyncAction::Replace
                } else {
                    SyncAction::Pending
                };

                // Add to pending for later decision
                let idx = plan.len();
                plan.push(PlanItem {
                    from,
                    to,
                    action,
                    reason: None,
                });

                if !force && is_conflict {
                    pending_indices.push(idx);
                }
            }

            // Missing target: safe to replace without prompt
            Err(_) => {
                plan.push(PlanItem {
                    from,
                    to,
                    action: SyncAction::Replace,
                    reason: None,
                });
            }
        }
    }

    for &idx in pending_indices.iter() {
        let item = &plan[idx];
        app_println!(mode, "\nConflict at: {}", item.to.display());
        if let Err(err) = print_preview(mode, &item.to) {
            app_println!(mode, "Preview error: {}", err);
        }

        let action = prompt_action(mode);
        plan[idx].action = action;
    }

    print_plan(mode, "Planned actions", &plan);
    if !force && !pending_indices.is_empty() {
        if !confirm_proceed(mode) {
            app_println!(mode, "Aborted.");
            return;
        }
    }

    let mut executed: Vec<PlanItem> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for mut item in plan {
        match item.action {
            SyncAction::Ignore | SyncAction::Skip => {
                executed.push(item);
            }
            SyncAction::Replace => {
                if let Err(err) = replace_link(&item.from, &item.to) {
                    errors.push(format!("{}: {}", item.to.display(), err));
                    item.action = SyncAction::Skip;
                    item.reason = Some("replace failed".to_string());
                }
                executed.push(item);
            }
            SyncAction::BackupReplace => {
                if let Err(err) = backup_and_replace(&item.from, &item.to) {
                    errors.push(format!("{}: {}", item.to.display(), err));
                    item.action = SyncAction::Skip;
                    item.reason = Some("backup+replace failed".to_string());
                }
                executed.push(item);
            }
            SyncAction::Pending => {
                // TODO: I don't even know how to handle the ones that are still pending.
                // This technically shouldn't even happen, so yea
                continue;
            }
        }
    }

    print_plan(mode, "Outcome", &executed);
    if !errors.is_empty() {
        app_println!(mode, "\nErrors:");
        for err in errors {
            app_println!(mode, "- {}", err);
        }
    }
}

// Helper to print out a preview of what the utility is going to do
//
// @param path: &Path - the path to the symlink
// @return Result<()> - if print was successful
fn print_preview(mode: &RunMode, path: &Path) -> std::io::Result<()> {
    let meta = std::fs::symlink_metadata(path)?;

    if meta.file_type().is_symlink() {
        let target = std::fs::read_link(path)?;
        app_println!(mode, "SYMLINK: {} -> {}", path.display(), target.display());
        return Ok(());
    }

    if meta.is_file() {
        print_file_preview(mode, path)?;
        return Ok(());
    }

    if meta.is_dir() {
        print_dir_preview(mode, path)?;
    }

    Ok(())
}

// Helper to print preview for all files in a directory recursively
//
// @param path: &Path - the directory path to traverse
// @return Result<()> - if print was successful
fn print_dir_preview(mode: &RunMode, path: &Path) -> std::io::Result<()> {
    app_println!(mode, "\nDIRECTORY: {}", path.display());
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        let meta = std::fs::symlink_metadata(&entry_path)?;

        if meta.is_dir() {
            print_dir_preview(mode, &entry_path)?;
            continue;
        }

        if meta.file_type().is_symlink() {
            let target = std::fs::read_link(&entry_path)?;
            app_println!(
                mode,
                "\nSYMLINK: {} -> {}",
                entry_path.display(),
                target.display()
            );
            continue;
        }

        if meta.is_file() {
            print_file_preview(mode, &entry_path)?;
        }
    }

    Ok(())
}

// Helper to print preview for a single file
//
// @param path: &Path - the file path to preview
// @return Result<()> - if print was successful
fn print_file_preview(mode: &RunMode, path: &Path) -> std::io::Result<()> {
    const MAX_PREVIEW_SIZE: u64 = 32 * 1024;
    let meta = std::fs::metadata(path)?;
    app_println!(mode, "\nFILE: {}", path.display());

    if meta.len() > MAX_PREVIEW_SIZE {
        app_println!(mode, "TOO LARGE ({} bytes)", meta.len());
        return Ok(());
    }

    let mut file = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    if buf.iter().any(|b| *b == 0) {
        app_println!(mode, "BINARY FILE");
        return Ok(());
    }

    match String::from_utf8(buf) {
        Ok(text) => {
            if text.is_empty() {
                app_println!(mode, "(empty)");
            } else {
                app_print!(mode, "{}", text);
                if !text.ends_with('\n') {
                    app_println!(mode, "");
                }
            }
        }
        Err(_) => app_println!(mode, "BINARY FILE"),
    }

    Ok(())
}

// Helper to get user choice on how to resolve a conflict
//
// @return SyncAction - the chosen action
fn prompt_action(mode: &RunMode) -> SyncAction {
    loop {
        app_print!(mode, "Action [r]eplace, [b]ackup, [s]kip: ");
        let mut stdout = std::io::stdout();
        let _ = std::io::Write::flush(&mut stdout);

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            continue;
        }

        let choice = input.trim().to_lowercase();
        match choice.as_str() {
            "r" | "replace" => return SyncAction::Replace,
            "b" | "backup" => return SyncAction::BackupReplace,
            "s" | "skip" => return SyncAction::Skip,
            _ => app_println!(mode, "Invalid choice. Use r, b, or s."),
        }
    }
}

// Helper to ask for a final confirmation before executing actions
//
// @return bool - true if confirmed, false otherwise
fn confirm_proceed(mode: &RunMode) -> bool {
    app_print!(mode, "\nProceed? [y/N]: ");
    let mut stdout = std::io::stdout();
    let _ = std::io::Write::flush(&mut stdout);
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return false;
    }

    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

// Helper to print a summary of planned or executed actions
//
// @param title: &str - the title of the summary section
// @param plan: &[PlanItem] - items to print
fn print_plan(mode: &RunMode, title: &str, plan: &[PlanItem]) {
    app_println!(mode, "\n{}", title);
    print_plan_section(mode, "ignored", plan, SyncAction::Ignore);
    print_plan_section(mode, "skipped", plan, SyncAction::Skip);
    print_plan_section(mode, "replaced", plan, SyncAction::Replace);
    print_plan_section(mode, "backup+replaced", plan, SyncAction::BackupReplace);
}

// Helper to print a summary for a specific action group
//
// @param label: &str - the label for the action group
// @param plan: &[PlanItem] - items to print
// @param action: SyncAction - action type to filter by
fn print_plan_section(mode: &RunMode, label: &str, plan: &[PlanItem], action: SyncAction) {
    let mut items = plan.iter().filter(|item| item.action == action).peekable();
    if items.peek().is_none() {
        return;
    }

    app_println!(mode, "\n{}:", label);
    for item in items {
        match &item.reason {
            Some(reason) => app_println!(mode, "- {} ({})", item.to.display(), reason),
            None => app_println!(mode, "- {}", item.to.display()),
        }
    }
}

fn is_empty_path(path: &Path, meta: &std::fs::Metadata) -> std::io::Result<bool> {
    if meta.is_file() {
        return Ok(meta.len() == 0);
    }

    if meta.is_dir() {
        return is_empty_dir_recursive(path);
    }

    if meta.file_type().is_symlink() {
        return Ok(false);
    }

    Ok(false)
}

fn is_empty_dir_recursive(path: &Path) -> std::io::Result<bool> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        let meta = std::fs::symlink_metadata(&entry_path)?;

        if meta.is_file() {
            if meta.len() > 0 {
                return Ok(false);
            }
            continue;
        }

        if meta.is_dir() {
            if !is_empty_dir_recursive(&entry_path)? {
                return Ok(false);
            }
            continue;
        }

        return Ok(false);
    }

    Ok(true)
}

fn help(mode: &RunMode) {
    app_println!(mode, "dbdm - dotfile link manager");
    app_println!(mode, "\nUsage:");
    app_println!(mode, "  dbdm <command> [--force]");
    app_println!(mode, "\nCommands:");
    app_println!(mode, "  check   Validate config and planned links");
    app_println!(mode, "  sync    Apply config links to the filesystem");
    app_println!(mode, "  help    Show this help message");
    app_println!(mode, "\nConfig:");
    app_println!(mode, "  Looks for dbdm.conf in the current directory.");
    app_println!(mode, "  Each line: 'link = <from> <to>'");
}
