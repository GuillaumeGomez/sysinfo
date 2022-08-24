// Take a look at the license at the top of the repository in the LICENSE file.

#[test]
fn test_send_sync() {
    fn is_send<T: Send>() {}
    fn is_sync<T: Sync>() {}

    is_send::<sysinfo::System>();
    is_sync::<sysinfo::System>();
}
