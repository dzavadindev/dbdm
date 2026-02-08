use crate::config_parser::Config;
use std::io::Read;
use std::path::{Path, PathBuf};

mod config_parser;

fn main() {
    // Grab current dir
    let mut pwd = match std::env::current_dir() {
        Ok(path) => path,
        Err(err) => {
            println!("Could not parse the {}", err.to_string());
            return;
        }
    };

    // Check for presence of dbdm.conf
    pwd.push("dbdm.conf");
    if !pwd.exists() {
        let mut path_str = pwd.clone();
        path_str.pop();
        println!(
            "dbdm.conf doesn exist in {}",
            path_str.to_str().expect("Can't parse dir path")
        );
        return;
    }

    // Parse the config
    let config = match config_parser::read_config(&pwd) {
        Ok(res) => res,
        Err(err) => {
            println!("Error in config:\n\n{}", err);
            return;
        }
    };

    // Handle the command
    let command = std::env::args().nth(1).unwrap_or(String::from("help"));
    match command.as_str() {
        "check" => check(&config),
        "sync" => sync(&config),
        "help" => help(),
        _ => help(),
    }
}

// One of the command handlers
// Allows to check if the current state of the system matches
// the desired state that is specified in the provided config
//
// @param config: &Config - the parsed config state
fn check(config: &Config) {
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
            println!(
                "\x1b[32m{} -> {}\x1b[0m",
                from_full.display(),
                to_full.display()
            );
        } else {
            println!(
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
fn sync(config: &Config) {
    let force = std::env::args().any(|arg| arg == "--force");

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

                // Account for the flag
                let action = if force {
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

                if !force {
                    pending_indices.push(idx);
                }
            }

            // It was an invalid link
            Err(_) => {
                plan.push(PlanItem {
                    from,
                    to,
                    action: SyncAction::Skip,
                    reason: Some("path does not exist".to_string()),
                });
            }
        }
    }

    for idx in pending_indices {
        let item = &plan[idx];
        println!("\nConflict at: {}", item.to.display());
        if let Err(err) = print_preview(&item.to) {
            println!("Preview error: {}", err);
        }

        let action = prompt_action();
        plan[idx].action = action;
    }

    print_plan("Planned actions", &plan);
    if !confirm_proceed() {
        println!("Aborted.");
        return;
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

    print_plan("Outcome", &executed);
    if !errors.is_empty() {
        println!("\nErrors:");
        for err in errors {
            println!("- {}", err);
        }
    }
}

// Helper to make an absolute path out of a Path
//
// @param path: &Path - the path to canonicalize
// @return PathBuf - the canonicalized path or the initial Path converted to PathBuf
fn canonicalize_or_fallback(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

// Helper to resolve a symlink target into an absolute path
//
// `read_link` can return a relative target, which is interpreted relative to the
// symlinks parent directory. This helper normalizes that into a concrete path
// so it can be compared reliably with the expected target.
//
// @param link_path: &Path - the path to the symlink
// @param target: &Path - the raw target path read from the symlink
// @return PathBuf - the resolved target path
fn resolve_symlink_target(link_path: &Path, target: &Path) -> PathBuf {
    if target.is_relative() {
        link_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(target)
    } else {
        target.to_path_buf()
    }
}

// Helper to print out a preview of what the utility is going to do
//
// @param path: &Path - the path to the symlink
// @return Result<()> - if print was successful
fn print_preview(path: &Path) -> std::io::Result<()> {
    let meta = std::fs::symlink_metadata(path)?;

    if meta.file_type().is_symlink() {
        let target = std::fs::read_link(path)?;
        println!("SYMLINK: {} -> {}", path.display(), target.display());
        return Ok(());
    }

    if meta.is_file() {
        print_file_preview(path)?;
        return Ok(());
    }

    if meta.is_dir() {
        print_dir_preview(path)?;
    }

    Ok(())
}

// Helper to print preview for all files in a directory recursively
//
// @param path: &Path - the directory path to traverse
// @return Result<()> - if print was successful
fn print_dir_preview(path: &Path) -> std::io::Result<()> {
    println!("\nDIRECTORY: {}", path.display());
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        let meta = std::fs::symlink_metadata(&entry_path)?;

        if meta.is_dir() {
            print_dir_preview(&entry_path)?;
            continue;
        }

        if meta.file_type().is_symlink() {
            let target = std::fs::read_link(&entry_path)?;
            println!(
                "\nSYMLINK: {} -> {}",
                entry_path.display(),
                target.display()
            );
            continue;
        }

        if meta.is_file() {
            print_file_preview(&entry_path)?;
        }
    }

    Ok(())
}

// Helper to print preview for a single file
//
// @param path: &Path - the file path to preview
// @return Result<()> - if print was successful
fn print_file_preview(path: &Path) -> std::io::Result<()> {
    const MAX_PREVIEW_SIZE: u64 = 32 * 1024;
    let meta = std::fs::metadata(path)?;
    println!("\nFILE: {}", path.display());

    if meta.len() > MAX_PREVIEW_SIZE {
        println!("TOO LARGE ({} bytes)", meta.len());
        return Ok(());
    }

    let mut file = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    if buf.iter().any(|b| *b == 0) {
        println!("BINARY FILE");
        return Ok(());
    }

    match String::from_utf8(buf) {
        Ok(text) => {
            if text.is_empty() {
                println!("(empty)");
            } else {
                print!("{}", text);
                if !text.ends_with('\n') {
                    println!();
                }
            }
        }
        Err(_) => println!("BINARY FILE"),
    }

    Ok(())
}

// Helper to get user choice on how to resolve a conflict
//
// @return SyncAction - the chosen action
fn prompt_action() -> SyncAction {
    loop {
        print!("Action [r]eplace, [b]ackup, [s]kip: ");
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
            _ => println!("Invalid choice. Use r, b, or s."),
        }
    }
}

// Helper to ask for a final confirmation before executing actions
//
// @return bool - true if confirmed, false otherwise
fn confirm_proceed() -> bool {
    print!("\nProceed? [y/N]: ");
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
fn print_plan(title: &str, plan: &[PlanItem]) {
    println!("\n{}", title);
    print_plan_section("ignored", plan, SyncAction::Ignore);
    print_plan_section("skipped", plan, SyncAction::Skip);
    print_plan_section("replaced", plan, SyncAction::Replace);
    print_plan_section("backup+replaced", plan, SyncAction::BackupReplace);
}

// Helper to print a summary for a specific action group
//
// @param label: &str - the label for the action group
// @param plan: &[PlanItem] - items to print
// @param action: SyncAction - action type to filter by
fn print_plan_section(label: &str, plan: &[PlanItem], action: SyncAction) {
    let mut items = plan.iter().filter(|item| item.action == action).peekable();
    if items.peek().is_none() {
        return;
    }

    println!("\n{}:", label);
    for item in items {
        match &item.reason {
            Some(reason) => println!("- {} ({})", item.to.display(), reason),
            None => println!("- {}", item.to.display()),
        }
    }
}

// Helper to remove existing target and create a symlink
//
// @param from: &Path - the source path for the symlink
// @param to: &Path - the destination path for the symlink
// @return Result<()> - if replacement was successful
fn replace_link(from: &Path, to: &Path) -> std::io::Result<()> {
    remove_existing(to)?;
    std::os::unix::fs::symlink(from, to)
}

// Helper to backup an existing target and create a symlink
//
// @param from: &Path - the source path for the symlink
// @param to: &Path - the destination path to backup and replace
// @return Result<()> - if backup and replacement were successful
fn backup_and_replace(from: &Path, to: &Path) -> std::io::Result<()> {
    let backup_dir = match std::fs::metadata(from) {
        Ok(meta) if meta.is_dir() => from.to_path_buf(),
        _ => from
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| from.to_path_buf()),
    };

    std::fs::create_dir_all(&backup_dir)?;
    let base_name = to
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "backup".to_string());
    let backup_path = unique_backup_path(&backup_dir, &base_name);

    std::fs::rename(to, &backup_path)?;
    std::os::unix::fs::symlink(from, to)
}

// Helper to create a unique backup path with a numeric suffix
//
// @param dir: &Path - the directory where backup should be created
// @param name: &str - the base name of the file being backed up
// @return PathBuf - the unique backup path
fn unique_backup_path(dir: &Path, name: &str) -> PathBuf {
    let base = format!("{}.bak.dbdm", name);
    let mut path = dir.join(&base);
    let mut counter = 1;
    while path.exists() {
        let candidate = format!("{}.{}", base, counter);
        path = dir.join(candidate);
        counter += 1;
    }
    path
}

// Helper to remove existing path whether file, directory, or symlink
//
// @param path: &Path - the path to remove
// @return Result<()> - if removal was successful
fn remove_existing(path: &Path) -> std::io::Result<()> {
    let meta = std::fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || meta.is_file() {
        std::fs::remove_file(path)
    } else {
        std::fs::remove_dir_all(path)
    }
}

fn help() {
    println!("dbdm - dotfile link manager");
    println!("\nUsage:");
    println!("  dbdm <command> [--force]");
    println!("\nCommands:");
    println!("  check   Validate config and planned links");
    println!("  sync    Apply config links to the filesystem");
    println!("  help    Show this help message");
    println!("\nConfig:");
    println!("  Looks for dbdm.conf in the current directory.");
    println!("  Each line: 'link = <from> <to>'");
}
