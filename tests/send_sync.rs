//
// Sysinfo
//
// Copyright (c) 2020 Guillaume Gomez
//

extern crate sysinfo;

#[test]
fn test_send_sync() {
    fn is_send<T: Send>() {}
    fn is_sync<T: Sync>() {}

    is_send::<sysinfo::System>();
    is_sync::<sysinfo::System>();
}
