// Take a look at the license at the top of the repository in the LICENSE file.

#[cfg(feature = "system")]
use std::ffi::OsStr;
#[cfg(any(
    feature = "disk",
    feature = "system",
    feature = "network",
    feature = "pci"
))]
use std::fs::File;
#[cfg(any(
    feature = "disk",
    feature = "system",
    feature = "network",
    feature = "pci"
))]
use std::io::Read;
#[cfg(any(feature = "disk", feature = "system"))]
use std::io::{self};
#[cfg(any(
    feature = "disk",
    feature = "system",
    feature = "network",
    feature = "pci"
))]
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

#[cfg(feature = "network")]
pub(crate) fn read<P: AsRef<Path>>(parent: P, path: &str, data: &mut [u8]) -> u64 {
    if let Ok(mut f) = File::open(parent.as_ref().join(path))
        && let Ok(size) = f.read(data)
    {
        let mut i = 0;
        let mut ret = 0;

        while i < size && i < data.len() && data[i] >= b'0' && data[i] <= b'9' {
            ret *= 10;
            ret += (data[i] - b'0') as u64;
            i += 1;
        }
        return ret;
    }
    0
}

#[cfg(feature = "network")]
pub(crate) fn read_signed<P: AsRef<Path>>(parent: P, path: &str, data: &mut [u8]) -> i64 {
    if let Ok(mut f) = File::open(parent.as_ref().join(path))
        && let Ok(size) = f.read(data)
    {
        let mut i = 0;
        let mut ret = 0;

        let negative = i < size && data[i] == b'-';
        if negative {
            i += 1;
        }

        while i < size && i < data.len() && data[i] >= b'0' && data[i] <= b'9' {
            ret *= 10;
            ret += (data[i] - b'0') as i64;
            i += 1;
        }
        return if negative { -ret } else { ret };
    }
    i64::MIN
}

#[cfg(feature = "pci")]
pub(crate) fn read_hex<P: AsRef<Path>>(parent: P, path: &str, data: &mut [u8]) -> u64 {
    if let Ok(mut f) = File::open(parent.as_ref().join(path))
        && let Ok(size) = f.read(data)
        && size >= 3
        && data.len() >= 3
        && let Ok(str) = str::from_utf8(&data[..size])
        && let Some(str) = str.strip_prefix("0x")
    {
        return u64::from_str_radix(str.trim(), 16).unwrap_or(0);
    }
    0
}

// `read_str` clears and refills the Vec, so its length becomes the length of
// the string just read. For example, reading "up\n" leaves the Vec length at 3.
// Keep this buffer separate from numeric read buffers, otherwise counters could
// be truncated to a few bytes.
#[allow(clippy::ptr_arg)]
#[cfg(feature = "network")]
pub(crate) fn read_str<'data, P: AsRef<Path>>(
    parent: P,
    path: &str,
    data: &'data mut Vec<u8>,
) -> &'data [u8] {
    data.clear();
    if let Ok(mut f) = File::open(parent.as_ref().join(path))
        && let Ok(size) = f.read_to_end(data)
    {
        &mut data[..size]
    } else {
        b""
    }
}
