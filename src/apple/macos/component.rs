// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::ffi;
use crate::ComponentExt;

use libc::{c_char, c_int, c_void};

use std::mem;

pub(crate) const COMPONENTS_TEMPERATURE_IDS: &[(&str, &[i8])] = &[
    ("PECI CPU", &['T' as i8, 'C' as i8, 'X' as i8, 'C' as i8]), // PECI CPU "TCXC"
    ("PECI CPU", &['T' as i8, 'C' as i8, 'X' as i8, 'c' as i8]), // PECI CPU "TCXc"
    (
        "CPU Proximity",
        &['T' as i8, 'C' as i8, '0' as i8, 'P' as i8],
    ), // CPU Proximity (heat spreader) "TC0P"
    ("GPU", &['T' as i8, 'G' as i8, '0' as i8, 'P' as i8]),      // GPU "TG0P"
    ("Battery", &['T' as i8, 'B' as i8, '0' as i8, 'T' as i8]),  // Battery "TB0T"
];

pub(crate) struct ComponentFFI {
    input_structure: ffi::KeyData_t,
    val: ffi::Val_t,
}

impl ComponentFFI {
    fn new(key: &[i8], con: ffi::io_connect_t) -> Option<ComponentFFI> {
        unsafe {
            get_key_size(con, key)
                .ok()
                .map(|(input_structure, val)| ComponentFFI {
                    input_structure,
                    val,
                })
        }
    }

    fn temperature(&self, con: ffi::io_connect_t) -> Option<f32> {
        get_temperature_inner(con, &self.input_structure, &self.val)
    }
}

#[doc = include_str!("../../../md_doc/component.md")]
pub struct Component {
    temperature: f32,
    max: f32,
    critical: Option<f32>,
    label: String,
    ffi_part: ComponentFFI,
    connection: ffi::io_connect_t,
}

impl Component {
    /// Creates a new `Component` with the given information.
    pub(crate) fn new(
        label: String,
        max: Option<f32>,
        critical: Option<f32>,
        key: &[i8],
        connection: ffi::io_connect_t,
    ) -> Option<Component> {
        let ffi_part = ComponentFFI::new(key, connection)?;
        ffi_part
            .temperature(connection)
            .map(|temperature| Component {
                temperature,
                label,
                max: max.unwrap_or(0.0),
                critical,
                ffi_part,
                connection,
            })
    }
}

impl ComponentExt for Component {
    fn temperature(&self) -> f32 {
        self.temperature
    }

    fn max(&self) -> f32 {
        self.max
    }

    fn critical(&self) -> Option<f32> {
        self.critical
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn refresh(&mut self) {
        if let Some(temp) = self.ffi_part.temperature(self.connection) {
            self.temperature = temp;
            if self.temperature > self.max {
                self.max = self.temperature;
            }
        }
    }
}

unsafe fn perform_call(
    conn: ffi::io_connect_t,
    index: c_int,
    input_structure: *const ffi::KeyData_t,
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
fn strtoul(s: &[i8]) -> u32 {
    unsafe {
        ((*s.get_unchecked(0) as u32) << (3u32 << 3))
            + ((*s.get_unchecked(1) as u32) << (2u32 << 3))
            + ((*s.get_unchecked(2) as u32) << (1u32 << 3))
            + (*s.get_unchecked(3) as u32)
    }
}

#[inline]
unsafe fn ultostr(s: *mut c_char, val: u32) {
    *s.offset(0) = ((val >> 24) % 128) as i8;
    *s.offset(1) = ((val >> 16) % 128) as i8;
    *s.offset(2) = ((val >> 8) % 128) as i8;
    *s.offset(3) = (val % 128) as i8;
    *s.offset(4) = 0;
}

unsafe fn get_key_size(
    con: ffi::io_connect_t,
    key: &[i8],
) -> Result<(ffi::KeyData_t, ffi::Val_t), i32> {
    let mut input_structure: ffi::KeyData_t = mem::zeroed::<ffi::KeyData_t>();
    let mut output_structure: ffi::KeyData_t = mem::zeroed::<ffi::KeyData_t>();
    let mut val: ffi::Val_t = mem::zeroed::<ffi::Val_t>();

    input_structure.key = strtoul(key);
    input_structure.data8 = ffi::SMC_CMD_READ_KEYINFO;

    let result = perform_call(
        con,
        ffi::KERNEL_INDEX_SMC,
        &input_structure,
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
    Ok((input_structure, val))
}

unsafe fn read_key(
    con: ffi::io_connect_t,
    input_structure: &ffi::KeyData_t,
    mut val: ffi::Val_t,
) -> Result<ffi::Val_t, i32> {
    let mut output_structure: ffi::KeyData_t = mem::zeroed::<ffi::KeyData_t>();

    match perform_call(
        con,
        ffi::KERNEL_INDEX_SMC,
        input_structure,
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

fn get_temperature_inner(
    con: ffi::io_connect_t,
    input_structure: &ffi::KeyData_t,
    original_val: &ffi::Val_t,
) -> Option<f32> {
    unsafe {
        if let Ok(val) = read_key(con, input_structure, (*original_val).clone()) {
            if val.data_size > 0
                && libc::strcmp(val.data_type.as_ptr(), b"sp78\0".as_ptr() as *const i8) == 0
            {
                // convert sp78 value to temperature
                let x = (i32::from(val.bytes[0]) << 6) + (i32::from(val.bytes[1]) >> 2);
                return Some(x as f32 / 64f32);
            }
        }
    }
    None
}

pub(crate) fn get_temperature(con: ffi::io_connect_t, key: &[i8]) -> Option<f32> {
    unsafe {
        let (input_structure, val) = get_key_size(con, key).ok()?;
        get_temperature_inner(con, &input_structure, &val)
    }
}
