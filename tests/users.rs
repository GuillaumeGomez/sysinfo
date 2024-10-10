// Take a look at the license at the top of the repository in the LICENSE file.

// This test is used to ensure that the users are not loaded by default.

// users.groupps()  multiple times doesn't return the same output https://github.com/GuillaumeGomez/sysinfo/issues/1233
// Example:
// ---- test_users 1 stdout ----
// user: Administrator group:[Group { inner: GroupInner { id: Gid(0), name: "Administrators" } }]
// ---- test_users 2 stdout ----
// user: Administrator group:[]
// ....
#[cfg(feature = "user")]
#[test]
fn test_users() {
    use sysinfo::Users;

    if sysinfo::IS_SUPPORTED_SYSTEM {
        let mut users = Users::new();
        assert_eq!(users.iter().count(), 0);
        users.refresh_list();
        assert!(users.iter().count() > 0);
        //my first gruoup is Administrator
        let count = users.first().unwrap().groups().iter().len();
        for _ in 1..10 {
            // println!(
            //     "user: {} group:{:?}",
            //     users.first().unwrap().name(),
            //     users.first().unwrap().groups()
            // );
            assert!(users.first().unwrap().groups().iter().len() == count)
        }
    }
}
