// Take a look at the license at the top of the repository in the LICENSE file.

/* convert a path to a NUL-terminated Vec<u8> suitable for use with C functions */
#[cfg(all(
    not(feature = "unknown-ci"),
    any(target_os = "linux", target_os = "android", target_vendor = "apple")
))]
pub(crate) fn to_cpath(path: &std::path::Path) -> Vec<u8> {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    let path_os: &OsStr = path.as_ref();
    let mut cpath = path_os.as_bytes().to_vec();
    cpath.push(0);
    cpath
}

/// Converts the value into a parallel iterator (if the multithread feature is enabled)
/// Uses the rayon::iter::IntoParallelIterator trait
#[cfg(all(
    all(
        any(
            target_os = "linux",
            target_os = "android",
            target_os = "macos",
            target_os = "windows",
            target_os = "freebsd",
        ),
        feature = "multithread"
    ),
    not(feature = "apple-sandbox"),
    not(feature = "unknown-ci")
))]
pub(crate) fn into_iter<T>(val: T) -> T::Iter
where
    T: rayon::iter::IntoParallelIterator,
{
    val.into_par_iter()
}

/// Converts the value into a sequential iterator (if the multithread feature is disabled)
/// Uses the std::iter::IntoIterator trait
#[cfg(all(
    all(
        any(
            target_os = "linux",
            target_os = "android",
            target_os = "macos",
            target_os = "windows",
            target_os = "freebsd",
        ),
        not(feature = "multithread")
    ),
    not(feature = "unknown-ci"),
    not(feature = "apple-sandbox")
))]
pub(crate) fn into_iter<T>(val: T) -> T::IntoIter
where
    T: IntoIterator,
{
    val.into_iter()
}
