// Take a look at the license at the top of the repository in the LICENSE file.

#![cfg(feature = "component")]

use std::env::var;
use sysinfo::Error;

#[test]
fn test_components() {
    let mut c = match sysinfo::Components::new() {
        Ok(c) => c,
        Err(error) => {
            if !matches!(error, Error::Unsupported) {
                panic!("Failed to initialize `Components`");
            }
            return;
        }
    };
    assert!(c.is_empty());

    // Unfortunately, we can't get components in the CI...
    if cfg!(windows) || var("CI").is_ok() {
        return;
    }

    c.refresh(false);
    assert!(!c.is_empty());
}
