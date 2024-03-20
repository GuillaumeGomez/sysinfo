// Take a look at the license at the top of the repository in the LICENSE file.

/// Converts the value into a parallel iterator if the `multithread` feature is enabled.
/// Uses the `rayon::iter::IntoParallelIterator` trait.
#[cfg(all(
    feature = "multithread",
    not(feature = "unknown-ci"),
    not(all(target_os = "macos", feature = "apple-sandbox")),
))]
#[allow(dead_code)]
pub(crate) fn into_iter<T>(val: T) -> T::Iter
where
    T: rayon::iter::IntoParallelIterator,
{
    val.into_par_iter()
}

/// Converts the value into a sequential iterator if the `multithread` feature is disabled.
/// Uses the `std::iter::IntoIterator` trait.
#[cfg(any(
    not(feature = "multithread"),
    feature = "unknown-ci",
    all(target_os = "macos", feature = "apple-sandbox")
))]
#[allow(dead_code)]
pub(crate) fn into_iter<T>(val: T) -> T::IntoIter
where
    T: IntoIterator,
{
    val.into_iter()
}

/// Converts the value into a parallel mutable iterator if the `multithread` feature is enabled.
/// Uses the `rayon::iter::IntoParallelRefMutIterator` trait.
#[cfg(all(
    feature = "multithread",
    not(feature = "unknown-ci"),
    not(all(target_os = "macos", feature = "apple-sandbox")),
))]
pub(crate) fn into_iter_mut<'a, T>(
    val: &'a mut T,
) -> <T as rayon::iter::IntoParallelRefMutIterator<'a>>::Iter
where
    T: rayon::iter::IntoParallelRefMutIterator<'a> + ?Sized,
{
    val.par_iter_mut()
}

/// Converts the value into a sequential mutable iterator if the `multithread` feature is disabled.
/// Uses the `std::iter::IntoIterator` trait.
#[cfg(any(
    not(feature = "multithread"),
    feature = "unknown-ci",
    all(target_os = "macos", feature = "apple-sandbox")
))]
pub(crate) fn into_iter_mut<T>(val: T) -> T::IntoIter
where
    T: IntoIterator,
{
    val.into_iter()
}
