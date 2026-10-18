// Take a look at the license at the top of the repository in the LICENSE file.

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
