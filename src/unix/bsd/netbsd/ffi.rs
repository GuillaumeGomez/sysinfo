// Take a look at the license at the top of the repository in the LICENSE file.

#![allow(non_camel_case_types, dead_code)]

use libc::{c_char, c_int, c_ulong, c_void, kinfo_proc2, size_t};

pub(crate) const SIDL: u64 = 1;
pub(crate) const SACTIVE: u64 = 2;
pub(crate) const SDYING: u64 = 3;
pub(crate) const SSTOP: u64 = 4;
pub(crate) const SZOMB: u64 = 5;
pub(crate) const SDEAD: u64 = 6;

pub(crate) const LSIDL: i8 = 1;
pub(crate) const LSRUN: i8 = 2;
pub(crate) const LSSLEEP: i8 = 3;
pub(crate) const LSSTOP: i8 = 4;
pub(crate) const LSZOMB: i8 = 5;
pub(crate) const LSONPROC: i8 = 7;
pub(crate) const LSSUSPENDED: i8 = 8;

pub(crate) const KVM_NO_FILES: c_int = c_int::MAX;
pub(crate) const _POSIX2_LINE_MAX: usize = 2048;

pub(crate) const L_SYSTEM: u64 = 0x00000200;
pub(crate) const P_SYSTEM: u64 = L_SYSTEM;

pub(crate) const CTL_KERN: c_int = 1;
pub(crate) const CTL_VM: c_int = 2;
pub(crate) const CTL_VFS: c_int = 3;
pub(crate) const CTL_NET: c_int = 4;
pub(crate) const CTL_DEBUG: c_int = 5;
pub(crate) const CTL_HW: c_int = 6;

pub(crate) const HW_MACHINE: c_int = 1;
pub(crate) const HW_MODEL: c_int = 2;
pub(crate) const HW_NCPUONLINE: c_int = 16;

pub(crate) const CP_USER: usize = 0;
pub(crate) const CP_NICE: usize = 1;
pub(crate) const CP_SYS: usize = 2;
pub(crate) const CP_INTR: usize = 3;
pub(crate) const CP_IDLE: usize = 4;
pub(crate) const CPUSTATES: usize = 5;

pub(crate) const VM_UVMEXP2: c_int = 5;

pub(crate) type kvm_t = c_void;

pub(crate) struct uvmexp_sysctl {
    pub(crate) pagesize: i64,
    pub(crate) pagemask: i64,
    pub(crate) pageshift: i64,
    pub(crate) npages: i64,
    pub(crate) free: i64,
    pub(crate) active: i64,
    pub(crate) inactive: i64,
    pub(crate) paging: i64,
    pub(crate) wired: i64,
    pub(crate) zeropages: i64,
    pub(crate) reserve_pagedaemon: i64,
    pub(crate) reserve_kernel: i64,
    pub(crate) freemin: i64,
    pub(crate) freetarg: i64,
    pub(crate) inactarg: i64,
    pub(crate) wiredmax: i64,
    pub(crate) nswapdev: i64,
    pub(crate) swpages: i64,
    pub(crate) swpginuse: i64,
    pub(crate) swpgonly: i64,
    pub(crate) nswget: i64,
    pub(crate) unused1: i64,
    pub(crate) cpuhit: i64,
    pub(crate) cpumiss: i64,
    pub(crate) faults: i64,
    pub(crate) traps: i64,
    pub(crate) intrs: i64,
    pub(crate) swtch: i64,
    pub(crate) softs: i64,
    pub(crate) syscalls: i64,
    pub(crate) pageins: i64,
    pub(crate) swapins: i64,
    pub(crate) swapouts: i64,
    pub(crate) pgswapin: i64,
    pub(crate) pgswapout: i64,
    pub(crate) forks: i64,
    pub(crate) forks_ppwait: i64,
    pub(crate) forks_sharevm: i64,
    pub(crate) pga_zerohit: i64,
    pub(crate) pga_zeromiss: i64,
    pub(crate) zeroaborts: i64,
    pub(crate) fltnoram: i64,
    pub(crate) fltnoanon: i64,
    pub(crate) fltpgwait: i64,
    pub(crate) fltpgrele: i64,
    pub(crate) fltrelck: i64,
    pub(crate) fltrelckok: i64,
    pub(crate) fltanget: i64,
    pub(crate) fltanretry: i64,
    pub(crate) fltamcopy: i64,
    pub(crate) fltnamap: i64,
    pub(crate) fltnomap: i64,
    pub(crate) fltlget: i64,
    pub(crate) fltget: i64,
    pub(crate) flt_anon: i64,
    pub(crate) flt_acow: i64,
    pub(crate) flt_obj: i64,
    pub(crate) flt_prcopy: i64,
    pub(crate) flt_przero: i64,
    pub(crate) pdwoke: i64,
    pub(crate) pdrevs: i64,
    pub(crate) unused4: i64,
    pub(crate) pdfreed: i64,
    pub(crate) pdscans: i64,
    pub(crate) pdanscan: i64,
    pub(crate) pdobscan: i64,
    pub(crate) pdreact: i64,
    pub(crate) pdbusy: i64,
    pub(crate) pdpageouts: i64,
    pub(crate) pdpending: i64,
    pub(crate) pddeact: i64,
    pub(crate) anonpages: i64,
    pub(crate) filepages: i64,
    pub(crate) execpages: i64,
    pub(crate) colorhit: i64,
    pub(crate) colormiss: i64,
    pub(crate) ncolors: i64,
    pub(crate) bootpages: i64,
    pub(crate) poolpages: i64,
    pub(crate) countsyncone: i64,
    pub(crate) countsyncall: i64,
    pub(crate) anonunknown: i64,
    pub(crate) anonclean: i64,
    pub(crate) anondirty: i64,
    pub(crate) fileunknown: i64,
    pub(crate) fileclean: i64,
    pub(crate) filedirty: i64,
    pub(crate) fltup: i64,
    pub(crate) fltnoup: i64,
}

unsafe extern "C" {
    pub(crate) fn getifnum() -> c_int;
}

#[link(name = "kvm")]
unsafe extern "C" {
    pub(crate) fn kvm_close(kd: *mut kvm_t);
    pub(crate) fn kvm_openfiles(
        a: *const c_char,
        b: *const c_char,
        c: *const c_char,
        kind: c_int,
        error: *mut c_char,
    ) -> *mut kvm_t;
    pub(crate) fn kvm_getlwps(
        kd: *mut kvm_t,
        pid: c_int,
        procaddr: c_ulong,
        elemsize: size_t,
        count: *mut c_int,
    ) -> *mut libc::kinfo_lwp;
    pub(crate) fn kvm_getargv2(
        kd: *mut kvm_t,
        p: *const kinfo_proc2,
        nchr: c_int,
    ) -> *mut *mut c_char;
    pub(crate) fn kvm_getenvv2(
        kd: *mut kvm_t,
        p: *const kinfo_proc2,
        nchr: c_int,
    ) -> *mut *mut c_char;
    pub(crate) fn kvm_getproc2(
        kd: *mut kvm_t,
        op: c_int,
        arg: c_int,
        elemsize: size_t,
        cnt: *mut c_int,
    ) -> *mut kinfo_proc2;
}
