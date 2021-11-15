// Take a look at the license at the top of the repository in the LICENSE file.

use std::fs::{read_dir, File};
use std::io::Read;
use std::path::Path;

fn read_dirs<P: AsRef<Path>>(dir: P, errors: &mut u32) {
    for entry in read_dir(dir).expect("read_dir failed") {
        let entry = entry.expect("entry failed");
        let path = entry.path();
        if path.is_dir() {
            read_dirs(path, errors);
        } else {
            if !check_file_license(&path) {
                *errors += 1;
            }
        }
    }
}

fn read_file<P: AsRef<Path>>(p: P) -> String {
    let mut f = File::open(p).expect("read_file::open failed");
    let mut content =
        String::with_capacity(f.metadata().map(|m| m.len() as usize + 1).unwrap_or(0));
    f.read_to_string(&mut content)
        .expect("read_file::read_to_end failed");
    content
}

fn show_error(p: &Path, err: &str) {
    eprintln!("=> [{}]: {}", p.display(), err);
}

fn check_file_license(p: &Path) -> bool {
    let content = read_file(p);
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

#[test]
fn check_license_headers() {
    let mut errors = 0;
    for folder in ["src", "tests", "examples"] {
        read_dirs(folder, &mut errors);
    }
    assert!(errors == 0);
}
