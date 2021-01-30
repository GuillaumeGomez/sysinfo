//
// Sysinfo
//
// Copyright (c) 2020 Guillaume Gomez
//

#[test]
fn test_send_sync() {
    fn is_send<T: Send>() {}
    fn is_sync<T: Sync>() {}

    is_send::<sysinfo::System>();
    is_sync::<sysinfo::System>();
}
