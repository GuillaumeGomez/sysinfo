// Take a look at the license at the top of the repository in the LICENSE file.

use crate::common::Uid;
use crate::User;

use std::fmt::Debug;

/// Interacting with users.
///
/// ```no_run
/// use sysinfo::{Users, UsersExt};
///
/// let mut users = Users::new();
/// users.refresh_list();
/// for user in users.users() {
///     eprintln!("{user:?}");
/// }
/// ```
pub trait UsersExt: Debug {
    /// Creates a new [`Components`][crate::Components] type.
    ///
    /// ```no_run
    /// use sysinfo::{Users, UsersExt};
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// for user in users.users() {
    ///     eprintln!("{user:?}");
    /// }
    /// ```
    fn new() -> Self;

    /// Returns the users list.
    ///
    /// ```no_run
    /// use sysinfo::{Users, UsersExt};
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// for user in users.users() {
    ///     eprintln!("{user:?}");
    /// }
    /// ```
    fn users(&self) -> &[User];

    /// Returns the users list.
    ///
    /// ```no_run
    /// use sysinfo::{Users, UsersExt};
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// users.users_mut().sort_by(|user1, user2| {
    ///     user1.name().partial_cmp(user2.name()).unwrap()
    /// });
    /// ```
    fn users_mut(&mut self) -> &mut [User];

    /// Sort the users list with the provided callback.
    ///
    /// Internally, it is using the [`slice::sort_unstable_by`] function, so please refer to it
    /// for implementation details.
    ///
    /// You can do the same without this method by calling:
    ///
    /// ```no_run
    /// use sysinfo::{Users, UsersExt};
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// users.sort_by(|user1, user2| {
    ///     user1.name().partial_cmp(user2.name()).unwrap()
    /// });
    /// ```
    ///
    /// ⚠️ If you use [`UsersExt::refresh_list`], you will need to call this method to sort the
    /// users again.
    fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&User, &User) -> std::cmp::Ordering,
    {
        self.users_mut().sort_unstable_by(compare);
    }

    /// The user list will be emptied then completely recomputed.
    ///
    /// ```no_run
    /// use sysinfo::{Users, UsersExt};
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// ```
    fn refresh_list(&mut self);

    /// Returns the [`User`] matching the given `user_id`.
    ///
    /// **Important**: The user list must be filled before using this method, otherwise it will
    /// always return `None` (through the `refresh_*` methods).
    ///
    /// It is a shorthand for:
    ///
    /// ```ignore
    /// # use sysinfo::{Users, UsersExt};
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// users.users().find(|user| user.id() == user_id);
    /// ```
    ///
    /// Full example:
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System, Users, UsersExt};
    ///
    /// let mut s = System::new_all();
    /// let mut users = Users::new();
    ///
    /// users.refresh_list();
    ///
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     if let Some(user_id) = process.user_id() {
    ///         eprintln!("User for process 1337: {:?}", users.get_user_by_id(user_id));
    ///     }
    /// }
    /// ```
    fn get_user_by_id(&self, user_id: &Uid) -> Option<&User> {
        self.users().iter().find(|user| user.id() == user_id)
    }
}
