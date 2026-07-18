use std::collections::BTreeSet;

use anyhow::{Result, bail};

use super::{HostConfig, validate_scout_read_path};

pub const ALLOWED_READ_COMMANDS: &[&str] = &[
    "cat", "head", "tail", "grep", "rg", "ls", "tree", "wc", "uniq", "diff", "stat", "file", "du",
    "df", "pwd", "hostname", "uptime", "whoami",
];

pub const EXEC_DENYLIST: &[&str] = &[
    "sh", "bash", "zsh", "dash", "sudo", "su", "doas", "python", "python3", "perl", "ruby", "node",
    "lua", "php", "curl", "wget", "nc", "ncat", "socat", "rm", "dd", "mkfs", "cp", "mv", "chmod",
    "chown", "docker", "podman", "kubectl", "kill", "pkill", "env", "xargs", "awk", "sed", "vi",
    "vim", "nano", "cargo", "rustc", "apt", "apk", "dnf",
];
pub fn validate_command(command: &str, host_allowlist: &[String]) -> Result<()> {
    if command.is_empty()
        || !command
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        bail!("command name is invalid");
    }
    let deny: BTreeSet<&str> = EXEC_DENYLIST.iter().copied().collect();
    if deny.contains(command) {
        bail!("command is denied");
    }
    let allowed: BTreeSet<&str> = ALLOWED_READ_COMMANDS.iter().copied().collect();
    if allowed.contains(command) || host_allowlist.iter().any(|c| c == command) {
        return Ok(());
    }
    bail!("command is not allowlisted");
}

/// Validate the typed argv policy for a read-only Scout command.
///
/// The executable allowlist is only the first boundary. Options that execute a
/// helper or load arbitrary configuration are denied, and every filesystem
/// operand is checked against the host's read roots.
pub fn validate_command_args(host: &HostConfig, command: &str, args: &[&str]) -> Result<()> {
    validate_command(command, &host.exec_allowlist)?;
    if !ALLOWED_READ_COMMANDS.contains(&command) {
        bail!("custom scout commands require a registered typed argument policy");
    }
    if args.iter().any(|arg| arg.contains('\0')) {
        bail!("command arguments must not contain NUL bytes");
    }

    for index in command_filesystem_operand_indices(command, args)? {
        validate_scout_read_path(host, args[index])?;
    }
    Ok(())
}

pub(crate) fn command_filesystem_operand_indices(
    command: &str,
    args: &[&str],
) -> Result<Vec<usize>> {
    let (flags, value_flags): (&[&str], &[&str]) = match command {
        "cat" => (
            &[
                "-A",
                "-b",
                "-E",
                "-n",
                "-s",
                "-T",
                "-v",
                "--number",
                "--number-nonblank",
                "--show-all",
                "--show-ends",
                "--show-tabs",
                "--show-nonprinting",
                "--squeeze-blank",
            ],
            &[],
        ),
        "head" => (
            &["-q", "-v", "--quiet", "--silent", "--verbose"],
            &["-c", "--bytes", "-n", "--lines"],
        ),
        "tail" => (
            &[
                "-f",
                "-F",
                "-q",
                "-v",
                "--follow",
                "--quiet",
                "--silent",
                "--verbose",
            ],
            &["-c", "--bytes", "-n", "--lines", "-s", "--sleep-interval"],
        ),
        "ls" => (
            &[
                "-1",
                "-A",
                "-a",
                "-d",
                "-h",
                "-l",
                "-R",
                "--all",
                "--almost-all",
                "--directory",
                "--human-readable",
                "--recursive",
            ],
            &[],
        ),
        "tree" => (&["-a", "-d", "-f", "-i", "--noreport"], &["-L"]),
        "stat" => (
            &[
                "-f",
                "-L",
                "-t",
                "--dereference",
                "--file-system",
                "--terse",
            ],
            &["-c", "--format", "--printf"],
        ),
        "file" => (
            &["-b", "-L", "-z", "--brief", "--dereference", "--uncompress"],
            &[],
        ),
        "du" => (
            &[
                "-a",
                "-h",
                "-s",
                "-x",
                "--all",
                "--human-readable",
                "--summarize",
                "--one-file-system",
            ],
            &["-d", "--max-depth"],
        ),
        "diff" => (
            &[
                "-a",
                "-b",
                "-B",
                "-i",
                "-q",
                "-s",
                "-u",
                "-w",
                "--brief",
                "--ignore-all-space",
                "--ignore-blank-lines",
                "--ignore-case",
                "--report-identical-files",
                "--text",
                "--unified",
            ],
            &[],
        ),
        "wc" => (
            &[
                "-c", "-l", "-m", "-w", "--bytes", "--chars", "--lines", "--words",
            ],
            &[],
        ),
        "uniq" => (
            &[
                "-c",
                "-d",
                "-i",
                "-u",
                "--count",
                "--ignore-case",
                "--repeated",
                "--unique",
            ],
            &[
                "-f",
                "--skip-fields",
                "-s",
                "--skip-chars",
                "-w",
                "--check-chars",
            ],
        ),
        "grep" | "rg" => return grep_like_operand_indices(command, args),
        "df" | "pwd" | "hostname" | "uptime" | "whoami" => {
            if args.is_empty() {
                return Ok(Vec::new());
            }
            bail!("{command} does not accept scout arguments");
        }
        _ => bail!("no typed argument policy is registered for {command}"),
    };
    parse_path_operands(command, args, flags, value_flags)
}

fn parse_path_operands(
    command: &str,
    args: &[&str],
    flags: &[&str],
    value_flags: &[&str],
) -> Result<Vec<usize>> {
    let mut paths = Vec::new();
    let mut index = 0;
    let mut options = true;
    while index < args.len() {
        let arg = args[index];
        if options && arg == "--" {
            options = false;
        } else if options && arg.starts_with('-') {
            if arg.contains('=') || !flags.contains(&arg) && !value_flags.contains(&arg) {
                bail!("unsupported {command} option: {arg}");
            }
            if value_flags.contains(&arg) {
                index += 1;
                if index >= args.len() || args[index].starts_with('-') {
                    bail!("{command} option {arg} requires a value");
                }
            }
        } else {
            paths.push(index);
        }
        index += 1;
    }
    Ok(paths)
}

fn grep_like_operand_indices(command: &str, args: &[&str]) -> Result<Vec<usize>> {
    let flags = [
        "-F",
        "-H",
        "-I",
        "-i",
        "-l",
        "-n",
        "-v",
        "-w",
        "-x",
        "--fixed-strings",
        "--files-with-matches",
        "--ignore-case",
        "--line-number",
        "--invert-match",
        "--word-regexp",
        "--line-regexp",
    ];
    let value_flags = [
        "-A",
        "-B",
        "-C",
        "-e",
        "-g",
        "-m",
        "--after-context",
        "--before-context",
        "--context",
        "--glob",
        "--max-count",
        "--regexp",
    ];
    let mut paths = Vec::new();
    let mut index = 0;
    let mut options = true;
    let mut has_explicit_pattern = false;
    let mut positional_pattern_seen = false;
    while index < args.len() {
        let arg = args[index];
        if options && arg == "--" {
            options = false;
        } else if options && arg.starts_with('-') {
            if arg.contains('=') || !flags.contains(&arg) && !value_flags.contains(&arg) {
                bail!("unsupported {command} option: {arg}");
            }
            if value_flags.contains(&arg) {
                index += 1;
                if index >= args.len() {
                    bail!("{command} option {arg} requires a value");
                }
                has_explicit_pattern |= matches!(arg, "-e" | "--regexp");
            }
        } else if !has_explicit_pattern && !positional_pattern_seen {
            positional_pattern_seen = true;
        } else {
            paths.push(index);
        }
        index += 1;
    }
    if !has_explicit_pattern && !positional_pattern_seen {
        bail!("{command} requires a pattern");
    }
    Ok(paths)
}
