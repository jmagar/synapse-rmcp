//! Unified CLI color policy.

use std::io::IsTerminal;
use std::sync::atomic::{AtomicU8, Ordering};

const COLOR_AUTO: u8 = 0;
const COLOR_ALWAYS: u8 = 1;
const COLOR_NEVER: u8 = 2;

static COLOR_OVERRIDE: AtomicU8 = AtomicU8::new(COLOR_AUTO);

#[cfg(test)]
#[path = "color_policy_tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

pub fn install_color_choice(choice: ColorChoice) {
    let value = match choice {
        ColorChoice::Auto => COLOR_AUTO,
        ColorChoice::Always => COLOR_ALWAYS,
        ColorChoice::Never => COLOR_NEVER,
    };
    COLOR_OVERRIDE.store(value, Ordering::Relaxed);
}

pub fn resolve(is_terminal: bool) -> bool {
    match COLOR_OVERRIDE.load(Ordering::Relaxed) {
        COLOR_ALWAYS => true,
        COLOR_NEVER => false,
        _ => {
            if std::env::var_os("NO_COLOR").is_some() {
                return false;
            }
            if env_flag("FORCE_COLOR") || env_flag("CLICOLOR_FORCE") {
                return true;
            }
            is_terminal
        }
    }
}

pub fn enabled_stdout() -> bool {
    resolve(std::io::stdout().is_terminal())
}

pub fn enabled_stderr() -> bool {
    resolve(std::io::stderr().is_terminal())
}

fn env_flag(var: &str) -> bool {
    std::env::var_os(var)
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false)
}
