// Take a look at the license at the top of the repository in the LICENSE file.

#[cfg(feature = "system")]
use std::ffi::OsStr;
#[cfg(any(feature = "disk", feature = "system"))]
use std::fs::File;
#[cfg(any(feature = "disk", feature = "system"))]
use std::io::{self, Read};
#[cfg(any(feature = "disk", feature = "system"))]
use std::path::Path;

#[cfg(feature = "system")]
pub(crate) fn get_all_data_from_file(file: &mut File, size: usize) -> io::Result<Vec<u8>> {
    use std::io::Seek;

    let mut buf = Vec::with_capacity(size);
    file.rewind()?;
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Reads the whole content of `file` (from its start) into `buf`, replacing its previous content.
#[cfg(feature = "system")]
pub(crate) fn read_file_into(file: &mut File, buf: &mut Vec<u8>) -> io::Result<()> {
    use std::io::Seek;

    buf.clear();
    file.rewind()?;
    file.read_to_end(buf)?;
    Ok(())
}

/// Reads the whole content of the file at `file_path` into `buf`, replacing its previous content.
#[cfg(feature = "system")]
pub(crate) fn read_path_into<P: AsRef<Path>>(file_path: P, buf: &mut Vec<u8>) -> io::Result<()> {
    let mut file = File::open(file_path.as_ref())?;
    buf.clear();
    file.read_to_end(buf)?;
    Ok(())
}

#[cfg(any(feature = "disk", feature = "system"))]
pub(crate) fn get_all_utf8_data<P: AsRef<Path>>(file_path: P, size: usize) -> io::Result<String> {
    let mut file = File::open(file_path.as_ref())?;
    let mut buf = String::with_capacity(size);
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

/// This type is used in `retrieve_all_new_process_info` because we have a "parent" path and
/// from it, we `pop`/`join` every time because it's more memory efficient than using `Path::join`.
#[cfg(feature = "system")]
pub(crate) struct PathHandler(pub(crate) std::path::PathBuf);

#[cfg(feature = "system")]
impl PathHandler {
    /// `buf` is reused to avoid allocating. Get it back with [`PathHandler::into_inner`].
    pub(crate) fn new<P: AsRef<Path>>(mut buf: std::path::PathBuf, path: P) -> Self {
        buf.as_mut_os_string().clear();
        buf.push(path);
        // `path` is the "parent" for all paths which will follow so we add a fake element at
        // the end since every `PathHandler::replace_and_join` call will first call `pop`
        // internally.
        buf.push("a");
        Self(buf)
    }

    pub(crate) fn into_inner(self) -> std::path::PathBuf {
        self.0
    }

    #[inline(always)]
    pub(crate) fn as_path(&self) -> &Path {
        &self.0
    }
}

#[cfg(feature = "system")]
pub(crate) trait PathPush {
    fn replace_and_join<S: AsRef<OsStr> + ?Sized>(&mut self, p: &S) -> &Path;
}

#[cfg(feature = "system")]
impl PathPush for PathHandler {
    #[inline(always)]
    fn replace_and_join<S: AsRef<OsStr> + ?Sized>(&mut self, p: &S) -> &Path {
        self.0.pop();
        self.0.push(p.as_ref());
        self.as_path()
    }
}

// This implementation allows to skip one allocation that is done in `PathHandler`.
#[cfg(feature = "system")]
impl PathPush for std::path::PathBuf {
    #[inline(always)]
    fn replace_and_join<S: AsRef<OsStr> + ?Sized>(&mut self, p: &S) -> &Path {
        self.push(p.as_ref());
        self.as_path()
    }
}

#[cfg(feature = "system")]
pub(crate) fn to_u64(v: &[u8]) -> u64 {
    let mut x = 0;

    for c in v {
        x *= 10;
        x += u64::from(c - b'0');
    }
    x
}

/// Converts a path to a NUL-terminated `Vec<u8>` suitable for use with C functions.
#[cfg(feature = "disk")]
pub(crate) fn to_cpath(path: &std::path::Path) -> Vec<u8> {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    let path_os: &OsStr = path.as_ref();
    let mut cpath = path_os.as_bytes().to_vec();
    cpath.push(0);
    cpath
}
