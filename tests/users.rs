// Take a look at the license at the top of the repository in the LICENSE file.

#![cfg(feature = "user")]

// This test is used to ensure that the users are not loaded by default.
//
// Calling users.groups() multiple times doesn't return the same output https://github.com/GuillaumeGomez/sysinfo/issues/1233
//
// Example:
// ---- test_users 1 stdout ----
// user: Administrator group:[Group { inner: GroupInner { id: Gid(0), name: "Administrators" } }]
// ---- test_users 2 stdout ----
// user: Administrator group:[]
#[test]
fn test_users() {
    use sysinfo::Users;

    let mut users = match Users::new() {
        Ok(u) => u,
        Err(error) => {
            std::assert_matches!(error, Error::Unsupported);
            return;
        }
    };
    assert_eq!(users.iter().count(), 0);
    users.refresh();
    assert!(users.iter().count() > 0);
    let count = users.first().unwrap().groups().iter().len();
    for _ in 1..10 {
        assert!(users.first().unwrap().groups().iter().len() == count)
    }
}

// This test ensures that there are actually groups listed, in particular for Windows.
#[test]
fn test_groups() {
    use sysinfo::Groups;

    let mut groups = match Groups::new() {
        Ok(g) => g,
        Err(error) => {
            std::assert_matches!(error, Error::Unsupported);
            return;
        }
    };
    groups.refresh();
    assert!(groups.list().len() > 1);
}
