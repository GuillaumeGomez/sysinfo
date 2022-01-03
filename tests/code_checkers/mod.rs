// Take a look at the license at the top of the repository in the LICENSE file.

mod headers;
mod utils;

use std::path::Path;

const CHECKS: &[(fn(&str, &Path) -> bool, &[&str])] =
    &[(headers::check_license_header, &["src", "tests", "examples"])];

fn handle_tests(nb_errors: &mut usize, nb_run: &mut usize) {
    utils::read_dirs(
        &["benches", "examples", "src", "tests"],
        &mut |p: &Path, c: &str| {
            if let Some(first) = p.iter().next().and_then(|first| first.to_str()) {
                for (check, filter) in CHECKS {
                    if filter.contains(&first) {
                        *nb_run += 1;
                        if !check(c, p) {
                            *nb_errors += 1;
                        }
                    }
                }
            }
        },
    );
}

#[test]
fn code_checks() {
    let mut nb_errors = 0;
    let mut nb_run = 0;

    handle_tests(&mut nb_errors, &mut nb_run);

    assert_eq!(nb_errors, 0);
    assert_ne!(nb_run, 0);
}
