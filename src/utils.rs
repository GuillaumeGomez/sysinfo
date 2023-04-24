// Take a look at the license at the top of the repository in the LICENSE file.

/// Converts the value into a parallel iterator (if the multi-thread feature is enabled).
/// Uses the `rayon::iter::IntoParallelIterator` trait.
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
    not(all(target_os = "macos", feature = "apple-sandbox")),
    not(feature = "unknown-ci")
))]
pub(crate) fn into_iter<T>(val: T) -> T::Iter
where
    T: rayon::iter::IntoParallelIterator,
{
    val.into_par_iter()
}

/// Converts the value into a sequential iterator (if the multithread feature is disabled).
/// Uses the `std::iter::IntoIterator` trait.
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
    not(all(target_os = "macos", feature = "apple-sandbox"))
))]
pub(crate) fn into_iter<T>(val: T) -> T::IntoIter
where
    T: IntoIterator,
{
    val.into_iter()
}
