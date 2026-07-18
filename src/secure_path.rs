//! Descriptor-bound Scout filesystem access.

use std::fs::File;
use std::os::fd::{AsRawFd, OwnedFd};
use std::path::Path;

use anyhow::{Context, Result, bail};
use rustix::fs::{Mode, OFlags, ResolveFlags, open, openat2};

use crate::synapse::{HostConfig, scout_allowed_read_roots, validate_scout_read_path};

#[cfg(test)]
#[path = "secure_path_tests.rs"]
mod tests;

pub(crate) struct BoundPath {
    file: File,
}

impl BoundPath {
    pub(crate) fn file(&self) -> &File {
        &self.file
    }

    pub(crate) fn into_file(self) -> File {
        self.file
    }

    pub(crate) fn proc_path(&self) -> String {
        format!("/proc/self/fd/{}", self.file.as_raw_fd())
    }
}

pub(crate) fn bind_read_path(host: &HostConfig, path: &str) -> Result<BoundPath> {
    validate_scout_read_path(host, path)?;
    let root = matching_root(host, path)
        .ok_or_else(|| anyhow::anyhow!("path is outside configured scout read roots"))?;
    let root_relative = root.trim_start_matches('/');
    let target_relative = Path::new(path)
        .strip_prefix(&root)
        .context("path is outside selected scout read root")?;

    let slash: OwnedFd = open(
        "/",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    let root_fd = openat2(
        &slash,
        if root_relative.is_empty() {
            "."
        } else {
            root_relative
        },
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )?;
    let target = if target_relative.as_os_str().is_empty() {
        Path::new(".")
    } else {
        target_relative
    };
    let fd = openat2(
        &root_fd,
        target,
        OFlags::RDONLY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .with_context(|| format!("securely open scout path {path}"))?;
    Ok(BoundPath { file: fd.into() })
}

pub(crate) fn root_and_relative(host: &HostConfig, path: &str) -> Result<(String, String)> {
    validate_scout_read_path(host, path)?;
    let root = matching_root(host, path)
        .ok_or_else(|| anyhow::anyhow!("path is outside configured scout read roots"))?;
    let relative = Path::new(path)
        .strip_prefix(&root)
        .context("path is outside selected scout read root")?
        .to_string_lossy()
        .trim_start_matches('/')
        .to_owned();
    if relative.split('/').any(|part| part == "..") {
        bail!("path traversal is not allowed");
    }
    Ok((root, relative))
}

fn matching_root(host: &HostConfig, path: &str) -> Option<String> {
    scout_allowed_read_roots(host)
        .into_iter()
        .filter(|root| {
            root == "/"
                || path == root
                || path
                    .strip_prefix(root)
                    .is_some_and(|rest| rest.starts_with('/'))
        })
        .max_by_key(String::len)
}
