// Take a look at the license at the top of the repository in the LICENSE file.

use crate::common::{Gid, MacAddr, Uid};
use crate::{Component, Group, User};

use std::fmt::Debug;

/// Contains all the methods of the [`Cpu`][crate::Cpu] struct.
///
/// ```no_run
/// use sysinfo::{CpuExt, System, RefreshKind, CpuRefreshKind};
///
/// let mut s = System::new_with_specifics(
///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
/// );
///
/// // Wait a bit because CPU usage is based on diff.
/// std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
/// // Refresh CPUs again.
/// s.refresh_cpu();
///
/// for cpu in s.cpus() {
///     println!("{}%", cpu.cpu_usage());
/// }
/// ```
pub trait CpuExt: Debug {
    /// Returns this CPU's usage.
    ///
    /// Note: You'll need to refresh it at least twice (diff between the first and the second is
    /// how CPU usage is computed) at first if you want to have a non-zero value.
    ///
    /// ```no_run
    /// use sysinfo::{CpuExt, System, RefreshKind, CpuRefreshKind};
    ///
    /// let mut s = System::new_with_specifics(
    ///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
    /// );
    ///
    /// // Wait a bit because CPU usage is based on diff.
    /// std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    /// // Refresh CPUs again.
    /// s.refresh_cpu();
    ///
    /// for cpu in s.cpus() {
    ///     println!("{}%", cpu.cpu_usage());
    /// }
    /// ```
    fn cpu_usage(&self) -> f32;

    /// Returns this CPU's name.
    ///
    /// ```no_run
    /// use sysinfo::{CpuExt, System, RefreshKind, CpuRefreshKind};
    ///
    /// let s = System::new_with_specifics(
    ///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
    /// );
    /// for cpu in s.cpus() {
    ///     println!("{}", cpu.name());
    /// }
    /// ```
    fn name(&self) -> &str;

    /// Returns the CPU's vendor id.
    ///
    /// ```no_run
    /// use sysinfo::{CpuExt, System, RefreshKind, CpuRefreshKind};
    ///
    /// let s = System::new_with_specifics(
    ///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
    /// );
    /// for cpu in s.cpus() {
    ///     println!("{}", cpu.vendor_id());
    /// }
    /// ```
    fn vendor_id(&self) -> &str;

    /// Returns the CPU's brand.
    ///
    /// ```no_run
    /// use sysinfo::{CpuExt, System, RefreshKind, CpuRefreshKind};
    ///
    /// let s = System::new_with_specifics(
    ///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
    /// );
    /// for cpu in s.cpus() {
    ///     println!("{}", cpu.brand());
    /// }
    /// ```
    fn brand(&self) -> &str;

    /// Returns the CPU's frequency.
    ///
    /// ```no_run
    /// use sysinfo::{CpuExt, System, RefreshKind, CpuRefreshKind};
    ///
    /// let s = System::new_with_specifics(
    ///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
    /// );
    /// for cpu in s.cpus() {
    ///     println!("{}", cpu.frequency());
    /// }
    /// ```
    fn frequency(&self) -> u64;
}

/// Getting volume of received and transmitted data.
///
/// ```no_run
/// use sysinfo::{Networks, NetworkExt};
///
/// let mut networks = Networks::new();
/// networks.refresh_list();
/// for (interface_name, network) in &networks {
///     println!("[{interface_name}] {network:?}");
/// }
/// ```
pub trait NetworkExt: Debug {
    /// Returns the number of received bytes since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("in: {} B", network.received());
    /// }
    /// ```
    fn received(&self) -> u64;

    /// Returns the total number of received bytes.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("in: {} B", network.total_received());
    /// }
    /// ```
    fn total_received(&self) -> u64;

    /// Returns the number of transmitted bytes since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("out: {} B", network.transmitted());
    /// }
    /// ```
    fn transmitted(&self) -> u64;

    /// Returns the total number of transmitted bytes.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("out: {} B", network.total_transmitted());
    /// }
    /// ```
    fn total_transmitted(&self) -> u64;

    /// Returns the number of incoming packets since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("in: {}", network.packets_received());
    /// }
    /// ```
    fn packets_received(&self) -> u64;

    /// Returns the total number of incoming packets.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("in: {}", network.total_packets_received());
    /// }
    /// ```
    fn total_packets_received(&self) -> u64;

    /// Returns the number of outcoming packets since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("out: {}", network.packets_transmitted());
    /// }
    /// ```
    fn packets_transmitted(&self) -> u64;

    /// Returns the total number of outcoming packets.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("out: {}", network.total_packets_transmitted());
    /// }
    /// ```
    fn total_packets_transmitted(&self) -> u64;

    /// Returns the number of incoming errors since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("in: {}", network.errors_on_received());
    /// }
    /// ```
    fn errors_on_received(&self) -> u64;

    /// Returns the total number of incoming errors.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("in: {}", network.total_errors_on_received());
    /// }
    /// ```
    fn total_errors_on_received(&self) -> u64;

    /// Returns the number of outcoming errors since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("out: {}", network.errors_on_transmitted());
    /// }
    /// ```
    fn errors_on_transmitted(&self) -> u64;

    /// Returns the total number of outcoming errors.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("out: {}", network.total_errors_on_transmitted());
    /// }
    /// ```
    fn total_errors_on_transmitted(&self) -> u64;

    /// Returns the MAC address associated to current interface.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("MAC address: {}", network.mac_address());
    /// }
    /// ```
    fn mac_address(&self) -> MacAddr;
}

/// Interacting with components.
///
/// ```no_run
/// use sysinfo::{Components, ComponentsExt};
///
/// let mut components = Components::new();
/// components.refresh_list();
/// for component in components.iter() {
///     eprintln!("{component:?}");
/// }
/// ```
pub trait ComponentsExt: Debug {
    /// Creates a new [`Components`][crate::Components] type.
    ///
    /// ```no_run
    /// use sysinfo::{Components, ComponentsExt};
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// for component in components.iter() {
    ///     eprintln!("{component:?}");
    /// }
    /// ```
    fn new() -> Self;

    /// Returns the components list.
    ///
    /// ```no_run
    /// use sysinfo::{Components, ComponentsExt};
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// for component in components.components() {
    ///     eprintln!("{component:?}");
    /// }
    /// ```
    fn components(&self) -> &[Component];

    /// Returns the components list.
    ///
    /// ```no_run
    /// use sysinfo::{Components, ComponentsExt};
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// for component in components.components_mut() {
    ///     component.refresh();
    ///     eprintln!("{component:?}");
    /// }
    /// ```
    fn components_mut(&mut self) -> &mut [Component];

    /// Sort the components list with the provided callback.
    ///
    /// Internally, it is using the [`slice::sort_unstable_by`] function, so please refer to it
    /// for implementation details.
    ///
    /// You can do the same without this method by calling:
    ///
    /// ```no_run
    /// use sysinfo::{Components, ComponentsExt};
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// components.sort_by(|component1, component2| {
    ///     component2.label().partial_cmp(component2.label()).unwrap()
    /// });
    /// ```
    ///
    /// ⚠️ If you use [`ComponentsExt::refresh_list`], you will need to call this method to sort the
    /// components again.
    fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&Component, &Component) -> std::cmp::Ordering,
    {
        self.components_mut().sort_unstable_by(compare);
    }

    /// Refreshes the listed components' information.
    ///
    /// ⚠️ If a component is added or removed, this method won't take it into account. Use
    /// [`ComponentsExt::refresh_list`] instead.
    ///
    /// ⚠️ If you didn't call [`ComponentsExt::refresh_list`] beforehand, this method will do
    /// nothing as the component list will be empty.
    ///
    /// ```no_run
    /// use sysinfo::{Components, ComponentsExt};
    ///
    /// let mut components = Components::new();
    /// // We get the component list.
    /// components.refresh_list();
    /// // We wait some time...?
    /// components.refresh();
    /// ```
    fn refresh(&mut self) {
        for component in self.components_mut() {
            component.refresh();
        }
    }

    /// The component list will be emptied then completely recomputed.
    ///
    /// ```no_run
    /// use sysinfo::{Components, ComponentsExt};
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// ```
    fn refresh_list(&mut self);
}

/// Getting information for a user.
///
/// It is returned from [`UsersExt::users`].
///
/// ```no_run
/// use sysinfo::{Users, UsersExt, UserExt};
///
/// let mut users = Users::new();
/// users.refresh_list();
/// for user in users.users() {
///     println!("{} is in {} groups", user.name(), user.groups().len());
/// }
/// ```
pub trait UserExt: Debug + PartialEq + Eq + PartialOrd + Ord {
    /// Returns the ID of the user.
    ///
    /// ```no_run
    /// use sysinfo::{Users, UsersExt, UserExt};
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// for user in users.users() {
    ///     println!("{:?}", *user.id());
    /// }
    /// ```
    fn id(&self) -> &Uid;

    /// Returns the group ID of the user.
    ///
    /// ⚠️ This information is not set on Windows.  Windows doesn't have a `username` specific
    /// group assigned to the user. They do however have unique
    /// [Security Identifiers](https://docs.microsoft.com/en-us/windows/win32/secauthz/security-identifiers)
    /// made up of various [Components](https://docs.microsoft.com/en-us/windows/win32/secauthz/sid-components).
    /// Pieces of the SID may be a candidate for this field, but it doesn't map well to a single
    /// group ID.
    ///
    /// ```no_run
    /// use sysinfo::{Users, UsersExt, UserExt};
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// for user in users.users() {
    ///     println!("{}", *user.group_id());
    /// }
    /// ```
    fn group_id(&self) -> Gid;

    /// Returns the name of the user.
    ///
    /// ```no_run
    /// use sysinfo::{Users, UsersExt, UserExt};
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// for user in users.users() {
    ///     println!("{}", user.name());
    /// }
    /// ```
    fn name(&self) -> &str;

    /// Returns the groups of the user.
    ///
    /// ⚠️ This is computed every time this method is called.
    ///
    /// ```no_run
    /// use sysinfo::{Users, UsersExt, UserExt};
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// for user in users.users() {
    ///     println!("{} is in {:?}", user.name(), user.groups());
    /// }
    /// ```
    fn groups(&self) -> Vec<Group>;
}

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
    /// use sysinfo::{UserExt, Users, UsersExt};
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
    /// use sysinfo::{UserExt, Users, UsersExt};
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
    /// # use sysinfo::{UserExt, Users, UsersExt};
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

/// Getting information for a user group.
///
/// It is returned from [`UserExt::groups`].
///
/// ```no_run
/// use sysinfo::{GroupExt, UserExt, Users, UsersExt};
///
/// let mut users = Users::new();
///
/// for user in users.users() {
///     println!(
///         "user: (ID: {:?}, group ID: {:?}, name: {:?})",
///         user.id(),
///         user.group_id(),
///         user.name(),
///     );
///     for group in user.groups() {
///         println!("group: (ID: {:?}, name: {:?})", group.id(), group.name());
///     }
/// }
/// ```
pub trait GroupExt: Debug + PartialEq + Eq + PartialOrd + Ord {
    /// Returns the ID of the group.
    ///
    /// ⚠️ This information is not set on Windows.
    ///
    /// ```no_run
    /// use sysinfo::{GroupExt, UserExt, Users, UsersExt};
    ///
    /// let mut users = Users::new();
    ///
    /// for user in users.users() {
    ///     for group in user.groups() {
    ///         println!("{:?}", group.id());
    ///     }
    /// }
    /// ```
    fn id(&self) -> &Gid;

    /// Returns the name of the group.
    ///
    /// ```no_run
    /// use sysinfo::{GroupExt, UserExt, Users, UsersExt};
    ///
    /// let mut users = Users::new();
    ///
    /// for user in users.users() {
    ///     for group in user.groups() {
    ///         println!("{}", group.name());
    ///     }
    /// }
    /// ```
    fn name(&self) -> &str;
}
