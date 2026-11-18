// Take a look at the license at the top of the repository in the LICENSE file.

// This file must hold only one test. The test compares `Process::open_files` for this very
// process with a direct count of its descriptors, and that only works if nothing else in the
// process opens or closes files meanwhile. Each file in `tests/` is built as its own program,
// so a test that is alone in its file is alone in its process.

#![cfg(all(feature = "system", unix))]

use sysinfo::{ProcessesToUpdate, System};

/// The number of file descriptors this process has open, counted by asking about each
/// descriptor number in turn. Slow, but it does not depend on how sysinfo counts.
fn count_open_descriptors() -> usize {
    // Descriptor numbers stay below the open-file limit of the process, which `sysconf`
    // reports. That limit can be "none" or in the millions. This test process only has a
    // few descriptors, so stop at 65536 to keep the walk short.
    const CAP: libc::c_long = 1 << 16;
    let limit = match unsafe { libc::sysconf(libc::_SC_OPEN_MAX) } {
        n if n > 0 => n.min(CAP),
        _ => CAP,
    };
    (0..limit as libc::c_int)
        // `F_GETFD` only reads the descriptor's flags, and fails with `EBADF` if no such
        // descriptor is open.
        .filter(|&fd| unsafe { libc::fcntl(fd, libc::F_GETFD) } != -1)
        .count()
}

/// What the FreeBSD kernel lists for this process under `kern.proc.filedesc`: one line per
/// record, with its descriptor number, type and path. Shown when the count is wrong.
#[cfg(target_os = "freebsd")]
fn describe_kernel_file_table() -> String {
    let mib = [
        libc::CTL_KERN,
        libc::KERN_PROC,
        libc::KERN_PROC_FILEDESC,
        std::process::id() as libc::c_int,
    ];
    let mut len: libc::size_t = 0;
    let mut out = String::new();
    unsafe {
        if libc::sysctl(
            mib.as_ptr(),
            mib.len() as _,
            std::ptr::null_mut(),
            &mut len,
            std::ptr::null_mut(),
            0,
        ) != 0
        {
            return "kern.proc.filedesc: the size query failed".to_string();
        }
        // Room for descriptors opened between the two calls, as libutil's `kinfo_getfile` does.
        len = len * 4 / 3;
        let mut buf = vec![0u8; len];
        if libc::sysctl(
            mib.as_ptr(),
            mib.len() as _,
            buf.as_mut_ptr() as _,
            &mut len,
            std::ptr::null_mut(),
            0,
        ) != 0
        {
            return "kern.proc.filedesc: the read failed".to_string();
        }
        let mut offset = 0;
        while offset < len {
            let record = buf.as_ptr().add(offset) as *const libc::kinfo_file;
            let size = std::ptr::addr_of!((*record).kf_structsize).read_unaligned();
            if size <= 0 {
                break;
            }
            let fd = std::ptr::addr_of!((*record).kf_fd).read_unaligned();
            let kind = std::ptr::addr_of!((*record).kf_type).read_unaligned();
            let path = std::ffi::CStr::from_ptr(std::ptr::addr_of!((*record).kf_path) as _);
            out.push_str(&format!(
                "  kf_fd={fd:>3} kf_type={kind} path={:?}\n",
                path.to_string_lossy()
            ));
            offset += size as usize;
        }
    }
    out
}

#[cfg(not(target_os = "freebsd"))]
fn describe_kernel_file_table() -> String {
    String::new()
}

// `Process::open_files` must report the number of file descriptors the process has open, no
// more and no less.
//
// Where the count comes from listing `/proc/<pid>/fd`, the handle on that directory is a
// descriptor of this process too for the moment of the count, so one more is expected.
#[test]
fn open_files_counts_the_open_descriptors() {
    if cfg!(feature = "apple-sandbox") {
        return;
    }
    let Ok(pid) = sysinfo::get_current_pid() else {
        return;
    };
    let Ok(mut s) = System::new() else { return };
    let probe = usize::from(cfg!(any(
        target_os = "linux",
        target_os = "android",
        target_os = "netbsd"
    )));
    // Refresh once before counting. sysinfo may open handles of its own on the first refresh
    // and keep them for the life of `System`: on FreeBSD, `kvm_openfiles` opens `/dev/null`
    // twice. Those are descriptors of this process as well, so they must be open already
    // when the direct count is taken.
    s.refresh_processes(ProcessesToUpdate::Some(&[pid]), false);
    let expected = count_open_descriptors() + probe;
    s.refresh_processes(ProcessesToUpdate::Some(&[pid]), false);
    let Some(reported) = s.process(pid).and_then(|p| p.open_files()) else {
        // Not supported on this system.
        return;
    };
    assert_eq!(
        reported,
        expected,
        "`open_files` differs from a direct count of the open descriptors\n{}",
        describe_kernel_file_table(),
    );
}
