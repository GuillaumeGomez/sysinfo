// Take a look at the license at the top of the repository in the LICENSE file.

use std::fs::File;
use std::io::{self, Read, Seek};
use std::path::{Path, PathBuf};

use crate::sys::system::REMAINING_FILES;

pub(crate) fn get_all_data_from_file(file: &mut File, size: usize) -> io::Result<String> {
    let mut buf = String::with_capacity(size);
    file.rewind()?;
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

pub(crate) fn get_all_data<P: AsRef<Path>>(file_path: P, size: usize) -> io::Result<String> {
    let mut file = File::open(file_path.as_ref())?;
    get_all_data_from_file(&mut file, size)
}

#[allow(clippy::useless_conversion)]
pub(crate) fn realpath(path: &Path) -> std::path::PathBuf {
    match std::fs::read_link(path) {
        Ok(f) => f,
        Err(_e) => {
            sysinfo_debug!("failed to get real path for {:?}: {:?}", path, _e);
            PathBuf::new()
        }
    }
}

/// Type used to correctly handle the `REMAINING_FILES` global.
pub(crate) struct FileCounter(File);

impl FileCounter {
    pub(crate) fn new(f: File) -> Option<Self> {
        unsafe {
            if let Ok(ref mut x) = REMAINING_FILES.lock() {
                if **x > 0 {
                    **x -= 1;
                    return Some(Self(f));
                }
                // All file descriptors we were allowed are being used.
            }
        }
        None
    }
}

impl std::ops::Deref for FileCounter {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for FileCounter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for FileCounter {
    fn drop(&mut self) {
        unsafe {
            if let Ok(ref mut x) = crate::sys::system::REMAINING_FILES.lock() {
                **x += 1;
            }
        }
    }
}

/// This type is used in `retrieve_all_new_process_info` because we have a "parent" path and
/// from it, we `pop`/`join` every time because it's more memory efficient than using `Path::join`.
pub(crate) struct PathHandler(PathBuf);

impl PathHandler {
    pub(crate) fn new(path: &Path) -> Self {
        // `path` is the "parent" for all paths which will follow so we add a fake element at
        // the end since every `PathHandler::join` call will first call `pop` internally.
        Self(path.join("a"))
    }
}

pub(crate) trait PathPush {
    fn join(&mut self, p: &str) -> &Path;
}

impl PathPush for PathHandler {
    fn join(&mut self, p: &str) -> &Path {
        self.0.pop();
        self.0.push(p);
        self.0.as_path()
    }
}

// This implementation allows to skip one allocation that is done in `PathHandler`.
impl PathPush for PathBuf {
    fn join(&mut self, p: &str) -> &Path {
        self.push(p);
        self.as_path()
    }
}

pub(crate) fn to_u64(v: &[u8]) -> u64 {
    let mut x = 0;

    for c in v {
        x *= 10;
        x += u64::from(c - b'0');
    }
    x
}

/// Converts a path to a NUL-terminated `Vec<u8>` suitable for use with C functions.
pub(crate) fn to_cpath(path: &std::path::Path) -> Vec<u8> {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    let path_os: &OsStr = path.as_ref();
    let mut cpath = path_os.as_bytes().to_vec();
    cpath.push(0);
    cpath
}
