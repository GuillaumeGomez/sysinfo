// Take a look at the license at the top of the repository in the LICENSE file.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use crate::sys::system::REMAINING_FILES;

pub(crate) fn get_all_data_from_file(file: &mut File, size: usize) -> io::Result<String> {
    let mut buf = String::with_capacity(size);
    file.seek(SeekFrom::Start(0))?;
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

pub(crate) fn get_all_data<P: AsRef<Path>>(file_path: P, size: usize) -> io::Result<String> {
    let mut file = File::open(file_path.as_ref())?;
    get_all_data_from_file(&mut file, size)
}

#[allow(clippy::useless_conversion)]
pub(crate) fn realpath(original: &Path) -> std::path::PathBuf {
    use libc::{lstat, stat, S_IFLNK, S_IFMT};
    use std::fs;
    use std::mem::MaybeUninit;
    use std::path::PathBuf;

    fn and(x: u32, y: u32) -> u32 {
        x & y
    }

    // let ori = Path::new(original.to_str().unwrap());
    // Right now lstat on windows doesn't work quite well
    // if cfg!(windows) {
    //     return PathBuf::from(ori);
    // }
    let result = PathBuf::from(original);
    let mut result_s = result.to_str().unwrap_or("").as_bytes().to_vec();
    result_s.push(0);
    let mut buf = MaybeUninit::<stat>::uninit();
    unsafe {
        let res = lstat(result_s.as_ptr() as *const _, buf.as_mut_ptr());
        if res < 0 {
            PathBuf::new()
        } else {
            let buf = buf.assume_init();
            if and(buf.st_mode.into(), S_IFMT.into()) != S_IFLNK.into() {
                PathBuf::new()
            } else {
                match fs::read_link(&result) {
                    Ok(f) => f,
                    Err(_) => PathBuf::new(),
                }
            }
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
