//
// Sysinfo
//
// Copyright (c) 2018 Guillaume Gomez
//

use libc::{c_char, c_int, c_void};
use std::mem;
use sys::ffi;
use ComponentExt;

pub(crate) const COMPONENTS_TEMPERATURE_IDS: &[(&str, &[i8])] = &[
    ("CPU", &['T' as i8, 'C' as i8, '0' as i8, 'P' as i8, 0]), // CPU "TC0P"
    ("GPU", &['T' as i8, 'G' as i8, '0' as i8, 'P' as i8, 0]), // GPU "TG0P"
    ("Battery", &['T' as i8, 'B' as i8, '0' as i8, 'T' as i8, 0]), // Battery "TB0T"
];

/// Struct containing a component information (temperature and name for the moment).
pub struct Component {
    temperature: f32,
    max: f32,
    critical: Option<f32>,
    label: String,
    key: Vec<i8>,
}

impl Component {
    /// Creates a new `Component` with the given information.
    pub(crate) fn new(
        label: String,
        max: Option<f32>,
        critical: Option<f32>,
        key: Vec<i8>,
        con: ffi::io_connect_t,
    ) -> Option<Component> {
        get_temperature(con, &key).map(|temperature| Component {
            temperature,
            label,
            max: max.unwrap_or(0.0),
            critical,
            key,
        })
    }

    pub(crate) fn update(&mut self, con: ffi::io_connect_t) {
        if let Some(temp) = get_temperature(con, &self.key) {
            self.temperature = temp;
            if self.temperature > self.max {
                self.max = self.temperature;
            }
        }
    }
}

impl ComponentExt for Component {
    fn get_temperature(&self) -> f32 {
        self.temperature
    }

    fn get_max(&self) -> f32 {
        self.max
    }

    fn get_critical(&self) -> Option<f32> {
        self.critical
    }

    fn get_label(&self) -> &str {
        &self.label
    }
}

unsafe fn perform_call(
    conn: ffi::io_connect_t,
    index: c_int,
    input_structure: *mut ffi::KeyData_t,
    output_structure: *mut ffi::KeyData_t,
) -> i32 {
    let mut structure_output_size = mem::size_of::<ffi::KeyData_t>();

    ffi::IOConnectCallStructMethod(
        conn,
        index as u32,
        input_structure,
        mem::size_of::<ffi::KeyData_t>(),
        output_structure,
        &mut structure_output_size,
    )
}

// Adapted from https://github.com/lavoiesl/osx-cpu-temp/blob/master/smc.c#L28
#[inline]
unsafe fn strtoul(s: *const c_char) -> u32 {
    ((*s.offset(0) as u32) << (3u32 << 3))
        + ((*s.offset(1) as u32) << (2u32 << 3))
        + ((*s.offset(2) as u32) << (1u32 << 3))
        + ((*s.offset(3) as u32) << (0u32 << 3))
}

#[inline]
unsafe fn ultostr(s: *mut c_char, val: u32) {
    *s.offset(0) = ((val >> 24) % 128) as i8;
    *s.offset(1) = ((val >> 16) % 128) as i8;
    *s.offset(2) = ((val >> 8) % 128) as i8;
    *s.offset(3) = (val % 128) as i8;
    *s.offset(4) = 0;
}

unsafe fn read_key(con: ffi::io_connect_t, key: *const c_char) -> Result<ffi::Val_t, i32> {
    let mut input_structure: ffi::KeyData_t = mem::zeroed::<ffi::KeyData_t>();
    let mut output_structure: ffi::KeyData_t = mem::zeroed::<ffi::KeyData_t>();
    let mut val: ffi::Val_t = mem::zeroed::<ffi::Val_t>();

    input_structure.key = strtoul(key);
    input_structure.data8 = ffi::SMC_CMD_READ_KEYINFO;

    let result = perform_call(
        con,
        ffi::KERNEL_INDEX_SMC,
        &mut input_structure,
        &mut output_structure,
    );
    if result != ffi::KIO_RETURN_SUCCESS {
        return Err(result);
    }

    val.data_size = output_structure.key_info.data_size;
    ultostr(
        val.data_type.as_mut_ptr(),
        output_structure.key_info.data_type,
    );
    input_structure.key_info.data_size = val.data_size;
    input_structure.data8 = ffi::SMC_CMD_READ_BYTES;

    match perform_call(
        con,
        ffi::KERNEL_INDEX_SMC,
        &mut input_structure,
        &mut output_structure,
    ) {
        ffi::KIO_RETURN_SUCCESS => {
            libc::memcpy(
                val.bytes.as_mut_ptr() as *mut c_void,
                output_structure.bytes.as_mut_ptr() as *mut c_void,
                mem::size_of::<[u8; 32]>(),
            );
            Ok(val)
        }
        result => Err(result),
    }
}

pub(crate) fn get_temperature(con: ffi::io_connect_t, v: &[i8]) -> Option<f32> {
    if let Ok(val) = unsafe { read_key(con, v.as_ptr()) } {
        if val.data_size > 0
            && unsafe { libc::strcmp(val.data_type.as_ptr(), b"sp78\0".as_ptr() as *const i8) } == 0
        {
            // convert sp78 value to temperature
            let x = (i32::from(val.bytes[0]) << 6) + (i32::from(val.bytes[1]) >> 2);
            return Some(x as f32 / 64f32);
        }
    }
    None
}
