// Take a look at the license at the top of the repository in the LICENSE file.

use super::utils::show_error;
use std::path::Path;

pub fn check_license_header(content: &str, p: &Path) -> bool {
    let mut lines = content.lines();
    let next = lines.next();
    let header = "// Take a look at the license at the top of the repository in the LICENSE file.";

    match next {
        Some(s) if s == header => {
            let next = lines.next();
            match next {
                Some("") => true,
                Some(s) => {
                    show_error(
                        p,
                        &format!("Expected empty line after license header, found `{}`", s),
                    );
                    false
                }
                None => {
                    show_error(p, "This file should very likely not exist...");
                    false
                }
            }
        }
        Some(s) => {
            show_error(
                p,
                &format!(
                    "Expected license header at the top of the file (`{}`), found: `{}`",
                    header, s
                ),
            );
            false
        }
        None => {
            show_error(p, "This (empty?) file should very likely not exist...");
            false
        }
    }
}
