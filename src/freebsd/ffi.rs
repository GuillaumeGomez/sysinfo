use libc;

pub type sigset_t = libc::c_ulong;
pub type dev_t = libc::c_int;
#[cfg(target_pointer_width = "32")]
pub type vm_size_t = libc::c_uint32_t;
#[cfg(target_pointer_width = "64")]
pub type vm_size_t = libc::c_uint64_t;
#[cfg(target_pointer_width = "32")]
pub type segsz_t = libc::c_int32_t;
#[cfg(target_pointer_width = "64")]
pub type segsz_t = libc::c_int64_t;
pub type fixpt_t = libc::c_uint32_t;
pub type lwpid_t = libc::c_int32_t;

pub const WMESGLEN: usize = 8;
pub const LOCKNAMELEN: usize = 8;
pub const KI_NGROUPS: usize = 16;
pub const TDNAMLEN: usize = 16;
pub const LOGNAMELEN: usize = 17;
pub const COMMLEN: usize = 19;
pub const KI_EMULNAMELEN: usize = 16;
pub const LOGINCLASSLEN: usize = 17;
pub const KI_NSPARE_INT: usize = 4;
pub const KI_NSPARE_PTR: usize = 6;
pub const KI_NSPARE_LONG: usize = 12;

pub struct pargs {
    pub ar_ref: ::c_uint,
    pub ar_length: ::c_uint,
    pub ar_args: [::c_uchar; 1],
}

pub struct rusage {
    pub ru_utime: libc::timeval,
    pub ru_stime: libc::timeval,
    pub ru_maxrss: libc::c_long,
    pub ru_ixrss: libc::c_long,
    pub ru_idrss: libc::c_long,
    pub ru_isrss: libc::c_long,
    pub ru_minflt: libc::c_long,
    pub ru_majflt: libc::c_long,
    pub ru_nswap: libc::c_long,
    pub ru_inblock: libc::c_long,
    pub ru_oublock: libc::c_long,
    pub ru_msgsnd: libc::c_long,
    pub ru_msgrcv: libc::c_long,
    pub ru_nsignals: libc::c_long,
    pub ru_nvcsw: libc::c_long,
    pub ru_nivcsw: libc::c_long,
}

pub struct priority {
    pub pri_class: libc::c_uchar,
    pub pri_level: libc::c_uchar,
    pub pri_native: libc::c_uchar,
    pub pri_user: libc::c_uchar,
}

pub struct kinfo_proc {
    pub ki_structsize: libc::c_int,
    pub ki_layout: libc::c_int,
    pub ki_args: *mut pargs,
    pub ki_paddr: *mut libc::c_void, // struct proc -> http://code.metager.de/source/xref/freebsd/sys/sys/proc.h#507
    pub ki_addr: *mut libc::c_void, // struct user -> http://code.metager.de/source/xref/freebsd/sys/sys/user.h#238
    pub ki_tracep: *mut libc::c_void, // struct vnode -> http://code.metager.de/source/xref/freebsd/sys/sys/vnode.h#98
    pub ki_textvp: *mut libc::c_void, // struct vnode
    pub ki_fd: *mut libc::c_void, // struct filedesc -> http://code.metager.de/source/xref/freebsd/sys/sys/filedesc.h#77
    pub ki_vmspace: *mut libc::c_void, // struct vmspace -> http://code.metager.de/source/xref/freebsd/sys/vm/vm_map.h#234
    pub ki_wchan: *mut libc::c_void,
    pub ki_pid: libc::pid_t,
    pub ki_ppid: libc::pid_t,
    pub ki_pgid: libc::pid_t,
    pub ki_tpgid: libc::pid_t,
    pub ki_sid: libc::pid_t,
    pub ki_tsid: libc::pid_t,
    pub ki_jobc: libc::c_short,
    pub ki_spare_short1: libc::c_short,
    pub ki_tdev: libc::dev_t,
    pub ki_siglist: sigset_t,
    pub ki_sigmask: sigset_t,
    pub ki_sigignore: sigset_t,
    pub ki_sigcatch: sigset_t,
    pub ki_uid: libc::uid_t,
    pub ki_ruid: libc::uid_t,
    pub ki_svuid: libc::uid_t,
    pub ki_rgid: libc::gid_t,
    pub ki_svgid: libc::gid_t,
    pub ki_ngroups: libc::c_short,
    pub ki_spare_short2: libc::c_short,
    pub ki_groups: [libc::gid_t; KI_NGROUPS],
    pub ki_size: vm_size_t,
    pub ki_rssize: segsz_t,
    pub ki_swrss: segsz_t,
    pub ki_tsize: segsz_t,
    pub ki_dsize: segsz_t,
    pub ki_ssize: segsz_t,
    pub ki_xstat: libc::c_ushort,
    pub ki_acflag: libc::c_ushort,
    pub ki_pctcpu: fixpt_t,
    pub ki_estcpu: libc::c_uint,
    pub ki_slptime: libc::c_uint,
    pub ki_swtime: libc::c_uint,
    pub ki_cow: libc::c_uint,
    pub ki_runtime: libc::uint64_t,
    pub ki_start: libc::timeval,
    pub ki_childtime: libc::timeval,
    pub ki_flag: libc::c_long,
    pub ki_kiflag: libc::c_long,
    pub ki_traceflag: libc::c_int,
    pub ki_stat: libc::c_char,
    pub ki_nice: libc::c_char,
    pub ki_lock: libc::c_char,
    pub ki_rqindex: libc::c_char,
    pub ki_oncpu_old: libc::c_uchar,
    pub ki_lastcpu_old: libc::c_uchar,
    pub ki_tdname: [libc::c_char; TDNAMLEN + 1],
    pub ki_wmesg: [libc::c_char; WMESGLEN + 1],
    pub ki_login: [libc::c_char; LOGNAMELEN + 1],
    pub ki_lockname: [libc::c_char; LOCKNAMELEN + 1],
    pub ki_comm: [libc::c_char; COMMLEN + 1],
    pub ki_emul: [libc::c_char; KI_EMULNAMELEN + 1],
    pub ki_loginclass: [libc::c_char; LOGINCLASSLEN + 1],
    pub ki_sparestrings: [libc::c_char; 50],
    pub ki_spareints: [libc::c_int; KI_NSPARE_INT],
    pub ki_oncpu: libc::c_int,
    pub ki_lastcpu: libc::c_int,
    pub ki_tracer: libc::c_int,
    pub ki_flag2: libc::c_int,
    pub ki_fibnum: libc::c_int,
    pub ki_cr_flags: libc::c_uint,
    pub ki_jid: libc::c_int,
    pub ki_numthreads: libc::c_int,
    pub ki_tid: lwpid_t,
    pub ki_pri: priority,
    pub ki_rusage: rusage,
    pub ki_rusage_ch: rusage,
    pub ki_pcb: *mut libc::c_void, // struct pcb -> http://code.metager.de/source/xref/freebsd/sys/sparc64/include/pcb.h#43
    pub ki_kstack: *mut libc::c_void,
    pub ki_udata: *mut libc::c_void,
    pub ki_tdaddr: *mut libc::c_void, // struct thread -> http://code.metager.de/source/xref/freebsd/sys/sys/proc.h#209
    pub ki_spareptrs: [*mut libc::c_void; KI_NSPARE_PTR],
    pub ki_sparelongs: [libc::c_long; KI_NSPARE_LONG],
    pub ki_sflag: libc::c_long,
    pub ki_tdflags: libc::c_long,
}
