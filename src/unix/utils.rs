// Take a look at the license at the top of the repository in the LICENSE file.

#[cfg(feature = "user")]
pub(crate) fn cstr_to_rust(c: *const libc::c_char) -> Option<String> {
    cstr_to_rust_with_size(c, None)
}

#[cfg(any(feature = "disk", feature = "system", feature = "user"))]
#[allow(dead_code)]
pub(crate) fn cstr_to_rust_with_size(
    c: *const libc::c_char,
    size: Option<usize>,
) -> Option<String> {
    if c.is_null() {
        return None;
    }
    let (mut s, max) = match size {
        Some(len) => (Vec::with_capacity(len), len as isize),
        None => (Vec::new(), isize::MAX),
    };
    let mut i = 0;
    unsafe {
        loop {
            let value = *c.offset(i) as u8;
            if value == 0 {
                break;
            }
            s.push(value);
            i += 1;
            if i >= max {
                break;
            }
        }
        String::from_utf8(s).ok()
    }
}

#[cfg(all(
    feature = "system",
    not(any(
        target_os = "ios",
        all(target_os = "macos", feature = "apple-sandbox",)
    ))
))]
pub(crate) fn wait_process(pid: crate::Pid) -> Option<std::process::ExitStatus> {
    use std::os::unix::process::ExitStatusExt;

    let mut status = 0;
    // attempt waiting
    unsafe {
        if retry_eintr!(libc::waitpid(pid.0, &mut status, 0)) < 0 {
            // attempt failed (non-child process) so loop until process ends
            let duration = std::time::Duration::from_millis(10);
            while libc::kill(pid.0, 0) == 0 {
                std::thread::sleep(duration);
            }
        }
        Some(std::process::ExitStatus::from_raw(status))
    }
}

#[cfg(all(
    feature = "system",
    any(target_os = "linux", target_os = "android", target_os = "netbsd"),
))]
#[allow(clippy::useless_conversion)]
pub(crate) fn realpath<P: AsRef<std::path::Path>>(path: P) -> Option<std::path::PathBuf> {
    let path = path.as_ref();
    match std::fs::read_link(path) {
        Ok(path) => Some(path),
        Err(_e) => {
            sysinfo_debug!("failed to get real path for {:?}: {:?}", path, _e);
            None
        }
    }
}

//  Based off NetBSD `x86_cpu_topology()` implementation
//  https://github.com/NetBSD/src/blob/trunk/sys/arch/x86/x86/cpu_topology.c
#[cfg(all(
    feature = "system",
    any(target_os = "netbsd", target_os = "freebsd"),
    any(target_arch = "x86", target_arch = "x86_64")
))]
fn x86_cpu_package_id() -> Result<u32, crate::Error> {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{__cpuid, __cpuid_count};
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{__cpuid, __cpuid_count};

    unsafe fn add_u32(v: &mut Vec<u8>, i: u32) {
        let i = &i as *const u32 as *const u8;
        unsafe {
            v.push(*i);
            v.push(*i.offset(1));
            v.push(*i.offset(2));
            v.push(*i.offset(3));
        }
    }

    let max_cpuid = __cpuid(0).eax;
    let max_ext_cpuid = __cpuid(0x80000000).eax;

    //  check vendor
    enum Vendor {
        Intel,
        AMD,
    }

    let vendor_res = __cpuid(0);
    let mut x = Vec::with_capacity(3 * std::mem::size_of::<u32>());
    unsafe {
        add_u32(&mut x, vendor_res.ebx);
        add_u32(&mut x, vendor_res.edx);
        add_u32(&mut x, vendor_res.ecx);
    }
    let mut pos = 0;
    for e in x.iter() {
        if *e == 0 {
            break;
        }
        pos += 1;
    }
    let vendor_id = std::str::from_utf8(&x[..pos]).map_err(|_| crate::Error::Unsupported)?;
    let vendor = match vendor_id {
        "GenuineIntel" => Vendor::Intel,
        "AuthenticAMD" => Vendor::AMD,
        _ => Err(crate::Error::Unsupported)?,
    };

    //  processor info
    if max_cpuid < 1 {
        return Err(crate::Error::Unsupported);
    }

    let leaf1 = __cpuid(1);

    let signature = leaf1.eax;
    let features_edx = leaf1.edx;

    let apic_id = leaf1.ebx >> 24;

    let base_family = (signature >> 8) & 0x0f;
    let ext_family = (signature >> 20) & 0xff;

    let cpu_family = if base_family == 0x0f {
        base_family + ext_family
    } else {
        base_family
    };

    match vendor {
        Vendor::Intel if cpu_family < 6 => {
            //  package_id = apic_id
            return Ok(apic_id);
        }
        Vendor::AMD if cpu_family < 0xf => {
            //  package_id = apic_id
            return Ok(apic_id);
        }
        _ => {}
    }

    let mut package_id = apic_id;
    let mut core_bits = 0u32;

    //  check for HTT support

    //  NetBSD source:
    //  if ((ci->ci_feat_val[0] & CPUID_HTT) != 0) {
    //      x86_cpuid(1, descs);
    //      lp_max = __SHIFTOUT(descs[1], CPUID_HTT_CORES);
    //  } else {
    //      lp_max = 1;
    //  }
    let lp_max = if (features_edx & 0x10000000) != 0 {
        (leaf1.ebx >> 16) & 0xff
    } else {
        1
    };

    let core_max = match vendor {
        Vendor::Intel => {
            if max_cpuid >= 4 {
                let leaf4 = __cpuid_count(4, 0);
                //  EAX[31:26] = number of cores/package - 1
                ((leaf4.eax >> 26) & 0x3f) + 1
            } else {
                1
            }
        }
        Vendor::AMD => {
            //  In a case of AMD, HTT flag means CMP support.
            if (features_edx & 0x10000000) == 0 {
                1
            } else if cpu_family < 0x10 || max_ext_cpuid < 0x80000008 {
                //  Legacy Method, LPs represent Cores.
                lp_max
            } else {
                let leaf80000008 = __cpuid(0x80000008);
                // ECX[7:0] = number of physical threads in processor - 1
                let core_max = (leaf80000008.ecx & 0xff) + 1;

                // ECX[15:12] = APIC ID size
                let apic_id_size = (leaf80000008.ecx >> 12) & 0x0f;

                if apic_id_size != 0 {
                    core_bits = apic_id_size;
                }

                core_max
            }
        }
    };

    debug_assert!(lp_max >= core_max);
    let mut smt_bits = ((lp_max / core_max) - 1).ilog2() + 1;
    if core_bits == 0 {
        core_bits = (core_max - 1).ilog2() + 1;
    }

    //  Family 0xf and 0x10 processors may have different structure of APIC ID.
    if matches!(vendor, Vendor::AMD) && cpu_family < 0x11 {
        //  Needs to read a specific CPU register

        //  NetBSD source:
        //  const uint64_t reg = rdmsr(MSR_NB_CFG);
        //  if ((reg & NB_CFG_INITAPICCPUIDLO) == 0) {
        //      /*
        //       * 0xf:  { CoreId, NodeId[2:0] }
        //       * 0x10: { CoreId[1:0], 000b, NodeId[2:0] }
        //       */
        //      const u_int node_id = apic_id & __BITS(0, 2);
        //      apic_id = (cpu_family == 0xf) ?
        //      (apic_id >> core_bits) | (node_id << core_bits) :
        //      (apic_id >> 5) | (node_id << 2);
        //  }

        return Err(crate::Error::Unsupported);
    }

    //  Family 0x17 and above support SMT
    if matches!(vendor, Vendor::AMD) && cpu_family >= 0x17 {
        let leaf8000001e = __cpuid(0x8000001e);

        // EBX[15:8] = threads per core - 1
        let threads = ((leaf8000001e.ebx >> 8) & 0xff) + 1;

        debug_assert!(smt_bits == 0);
        smt_bits = threads.ilog2();
        debug_assert!(smt_bits <= core_bits);
        core_bits -= smt_bits;
    }

    if smt_bits + core_bits != 0 {
        if smt_bits + core_bits < u32::BITS {
            package_id = apic_id >> (smt_bits + core_bits);
        } else {
            package_id = 0;
        }
    }

    Ok(package_id)
}
