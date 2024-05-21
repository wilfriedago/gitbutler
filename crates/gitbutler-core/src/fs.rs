use std::path::{Path, PathBuf};

use anyhow::Result;
use bstr::BString;
use gix::dir::walk::EmissionMode;
use walkdir::WalkDir;

// Returns an ordered list of relative paths for files inside a directory recursively.
pub fn list_files<P: AsRef<Path>>(dir_path: P, ignore_prefixes: &[P]) -> Result<Vec<PathBuf>> {
    let mut files = vec![];
    let dir_path = dir_path.as_ref();
    if !dir_path.exists() {
        return Ok(files);
    }
    for entry in WalkDir::new(dir_path) {
        let entry = entry?;
        if !entry.file_type().is_dir() {
            let path = entry.path();
            let path = path.strip_prefix(dir_path)?;
            let path = path.to_path_buf();
            if ignore_prefixes
                .iter()
                .any(|prefix| path.starts_with(prefix.as_ref()))
            {
                continue;
            }
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

// Return an iterator of worktree-relative slash-separated paths for files inside the `worktree_dir`, recursively.
// Fails if the `worktree_dir` isn't a valid git repository.
pub fn iter_worktree_files(
    worktree_dir: impl AsRef<Path>,
) -> Result<impl Iterator<Item = BString>> {
    let repo = gix::open(worktree_dir.as_ref())?;
    let index = repo.index_or_empty()?;
    let disabled_interrupt_handling = Default::default();
    let options = repo
        .dirwalk_options()?
        .emit_tracked(true)
        .emit_untracked(EmissionMode::Matching);
    Ok(repo
        .dirwalk_iter(index, None::<&str>, disabled_interrupt_handling, options)?
        .filter_map(Result::ok)
        .map(|e| e.entry.rela_path))
}

/// Write a single file so that the write either fully succeeds, or fully fails,
/// assuming the containing directory already exists.
pub(crate) fn write<P: AsRef<Path>>(
    file_path: P,
    contents: impl AsRef<[u8]>,
) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        Ok(std::fs::write(file_path, contents)?)
    }

    #[cfg(not(windows))]
    {
        let mut temp_file = gix::tempfile::new(
            file_path.as_ref().parent().unwrap(),
            ContainingDirectory::Exists,
            AutoRemove::Tempfile,
        )?;
        temp_file.write_all(contents.as_ref())?;
        Ok(persist_tempfile(temp_file, file_path)?)
    }
}

/// Write a single file so that the write either fully succeeds, or fully fails,
/// and create all leading directories.
pub(crate) fn create_dirs_then_write<P: AsRef<Path>>(
    file_path: P,
    contents: impl AsRef<[u8]>,
) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let dir = file_path.as_ref().parent().unwrap();
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(file_path, contents)
    }

    #[cfg(not(windows))]
    {
        let mut temp_file = gix::tempfile::new(
            file_path.as_ref().parent().unwrap(),
            ContainingDirectory::CreateAllRaceProof(Retries::default()),
            AutoRemove::Tempfile,
        )?;
        temp_file.write_all(contents.as_ref())?;
        persist_tempfile(temp_file, file_path)
    }
}

#[allow(dead_code)]
fn persist_tempfile(
    tempfile: gix::tempfile::Handle<gix::tempfile::handle::Writable>,
    to_path: impl AsRef<Path>,
) -> std::io::Result<()> {
    match tempfile.persist(to_path) {
        Ok(Some(_opened_file)) => {
            // EXPERIMENT: Does this fix #3601?
            #[cfg(windows)]
            _opened_file.sync_all()?;
            Ok(())
        }
        Ok(None) => unreachable!(
            "BUG: a signal has caused the tempfile to be removed, but we didn't install a handler"
        ),
        Err(err) => Err(err.error),
    }
}
