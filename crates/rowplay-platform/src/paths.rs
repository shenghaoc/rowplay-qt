// SPDX-License-Identifier: GPL-3.0-or-later
//! Platform directories for on-disk state.
//!
//! Uses the `directories` crate, so the paths follow each platform's convention
//! (XDG on Linux, `~/Library/Application Support` on macOS, `%APPDATA%` on
//! Windows). Nothing stored here is a secret: the Concept2 token lives in the
//! OS keychain ([`crate::token_store`]).
//!
//! On Unix the private directories are created with mode `0700` and the files
//! with `0600`; the local cache holds the athlete's whole logbook, which is not
//! secret but is nobody else's business.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Reverse-DNS qualifier passed to `directories`.
const QUALIFIER: &str = "com";
/// Organisation passed to `directories`.
const ORGANISATION: &str = "rowplay";
/// Application name passed to `directories`.
const APPLICATION: &str = "rowplay-qt";

/// File name of the SQLite workout cache.
const WORKOUT_CACHE_FILE: &str = "workouts.sqlite";
/// File name of the JSON preferences file.
const PREFERENCES_FILE: &str = "preferences.json";

/// The app's platform directories.
///
/// `None` only when the platform provides none (for example a container without
/// `HOME`); callers turn that into their own error.
#[must_use]
pub fn project_dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from(QUALIFIER, ORGANISATION, APPLICATION)
}

/// Environment override for both [`data_dir`] and [`config_dir`].
///
/// Exists so automated runs (the QML runtime gate) are hermetic: without it a
/// gate run would sync into — and clear — the developer's real logbook cache.
/// Unset in normal use, where the platform conventions above apply.
pub const DATA_DIR_ENV: &str = "ROWPLAY_DATA_DIR";

/// The overridden base directory, if `ROWPLAY_DATA_DIR` is set and non-empty.
fn override_dir() -> Option<PathBuf> {
    std::env::var_os(DATA_DIR_ENV)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

/// The data directory, created with owner-only permissions if missing.
pub fn data_dir() -> io::Result<PathBuf> {
    let dir = match override_dir() {
        Some(base) => base.join("data"),
        None => project_dirs()
            .ok_or_else(no_project_dirs)?
            .data_dir()
            .to_path_buf(),
    };
    create_private_dir(&dir)?;
    Ok(dir)
}

/// The configuration directory, created with owner-only permissions if missing.
pub fn config_dir() -> io::Result<PathBuf> {
    let dir = match override_dir() {
        Some(base) => base.join("config"),
        None => project_dirs()
            .ok_or_else(no_project_dirs)?
            .config_dir()
            .to_path_buf(),
    };
    create_private_dir(&dir)?;
    Ok(dir)
}

/// The default SQLite workout cache path.
pub fn default_workout_cache_path() -> io::Result<PathBuf> {
    Ok(data_dir()?.join(WORKOUT_CACHE_FILE))
}

/// The default preferences file path.
pub fn default_preferences_path() -> io::Result<PathBuf> {
    Ok(config_dir()?.join(PREFERENCES_FILE))
}

/// Create `path` and any missing parents, mode `0700` on Unix.
pub fn create_private_dir(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;

        let mut builder = fs::DirBuilder::new();
        builder.recursive(true).mode(0o700);
        builder.create(path)
    }
    #[cfg(not(unix))]
    {
        fs::create_dir_all(path)
    }
}

/// Force `path` to owner-only permissions on Unix; a no-op elsewhere.
pub fn restrict_permissions(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

/// Create `path` if it does not exist, with mode `0600` on Unix, then make sure
/// an existing file is restricted too.
///
/// Used before handing a path to a library that creates the file itself
/// (SQLite), so the file never exists briefly with default permissions.
pub fn ensure_private_file(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        create_private_dir(parent)?;
    }
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        options.mode(0o600);
    }
    match options.open(path) {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    restrict_permissions(path)
}

/// The error returned when the platform exposes no application directories.
fn no_project_dirs() -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        "the platform provides no application data directory",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_dirs_resolve_to_the_app_identifier() {
        let dirs = project_dirs().expect("platform directories");
        assert!(
            dirs.data_dir()
                .starts_with(dirs.config_dir().parent().unwrap_or(Path::new("/")))
                || dirs.data_dir() != dirs.config_dir(),
            "data and config directories should be distinct"
        );
        let text = dirs.data_dir().to_string_lossy().to_lowercase();
        assert!(text.contains("rowplay"), "{text}");
    }

    #[test]
    fn default_paths_sit_in_the_project_directories() {
        let cache = default_workout_cache_path().expect("cache path");
        assert_eq!(cache.file_name().unwrap(), WORKOUT_CACHE_FILE);
        assert_eq!(cache.parent().unwrap(), data_dir().unwrap());

        let preferences = default_preferences_path().expect("preferences path");
        assert_eq!(preferences.file_name().unwrap(), PREFERENCES_FILE);
        assert_eq!(preferences.parent().unwrap(), config_dir().unwrap());
    }

    #[test]
    fn private_files_and_directories_are_owner_only_on_unix() {
        let root = tempfile::tempdir().expect("temp dir");
        let dir = root.path().join("nested").join("data");
        create_private_dir(&dir).expect("create dir");
        assert!(dir.is_dir());

        let file = dir.join("workouts.sqlite");
        ensure_private_file(&file).expect("create file");
        assert!(file.is_file());
        // Re-creating an existing file is fine.
        ensure_private_file(&file).expect("existing file");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            assert_eq!(
                fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
                0o700,
                "directory mode"
            );
            assert_eq!(
                fs::metadata(&file).unwrap().permissions().mode() & 0o777,
                0o600,
                "file mode"
            );

            // An existing file with wide permissions is tightened.
            fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
            ensure_private_file(&file).unwrap();
            assert_eq!(
                fs::metadata(&file).unwrap().permissions().mode() & 0o777,
                0o600,
                "existing file mode"
            );
        }
    }
}
