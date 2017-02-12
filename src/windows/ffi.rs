//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

//use winapi::UNICODE_STRING;
//use winapi::basetsd::ULONG_PTR;
use winapi::minwindef::{BOOL, /*BYTE,*/ DWORD, LPDWORD/*, ULONG*/};
use winapi::winnt::{BOOLEAN, LARGE_INTEGER, LPCSTR/*, PVOID*/};
use winapi::HANDLE;

use libc::{c_int, c_double, c_void};

extern "C" {
    pub fn PdhOpenQuery(data_source: LPCSTR, user_data: *mut c_void,
                        query: *mut PDH_HQUERY) -> PDH_STATUS;
    pub fn CreateEvent(event_attributes: *mut c_void, manual_reset: BOOL, initial_state: BOOL,
                       name: LPCSTR) -> HANDLE;
    pub fn PdhAddCounter(query: PDH_HQUERY, full_counter_path: LPCTSTR, user_data: *mut c_void,
                         counter: *mut PDH_HCOUNTER) -> PDH_STATUS;
    pub fn PdhCollectQueryData(query: PDH_HQUERY) -> PDH_STATUS;
   pub  fn PdhCollectQueryDataEx(query: PDH_HQUERY, time_interval_secs: DWORD,
                                 data_event: HANDLE) -> PDH_STATUS;
    pub fn PdhGetFormattedCounterArray(counter: PDH_HCOUNTER,
                                       format: DWORD,
                                       buffer_size: LPDWORD,
                                       buffer_count: LPDWORD,
                                       buffer: *mut PDH_FMT_COUNTERVALUE) -> PDH_STATUS;
}

#[allow(non_camel_case_types)]
pub type STORAGE_PROPERTY_ID = c_int;
#[allow(non_camel_case_types)]
pub type STORAGE_QUERY_TYPE = c_int;
/*#[allow(non_camel_case_types)]
pub type PRTL_USER_PROCESS_PARAMETERS = *mut RTL_USER_PROCESS_PARAMETERS;
#[allow(non_camel_case_types)]
pub type PPEB = *mut PEB;*/
// https://msdn.microsoft.com/en-us/windows/aa365231(v=vs.80)
#[allow(non_camel_case_types)]
pub type MEDIA_TYPE = DWORD;
#[allow(non_camel_case_types)]
pub type PDH_HCOUNTER = *mut c_void;
#[allow(non_camel_case_types)]
pub type PDH_HQUERY = *mut c_void;
#[allow(non_camel_case_types)]
pub type PDH_STATUS = DWORD;
#[allow(non_camel_case_types)]
pub type LPCTSTR = LPCSTR;

// https://msdn.microsoft.com/en-us/library/ff800839(v=vs.85).aspx
#[allow(non_upper_case_globals)]
pub const StorageDeviceTrimProperty: STORAGE_PROPERTY_ID = 8;
// https://msdn.microsoft.com/en-us/library/windows/desktop/ff800840(v=vs.85).aspx
#[allow(non_upper_case_globals)]
pub const PropertyStandardQuery: STORAGE_QUERY_TYPE = 0;
// https://github.com/maxux/librtinfo/blob/master/deprecated/windows/pdh.h#L289
pub const PDH_FMT_DOUBLE: DWORD = 0x00000200;

#[allow(non_snake_case)]
#[repr(C)]
pub struct STORAGE_PROPERTY_QUERY {
    pub PropertyId: STORAGE_PROPERTY_ID,
    pub QueryType: STORAGE_QUERY_TYPE,
    pub AdditionalParameters: [BOOL; 1],
}

#[allow(non_snake_case)]
#[repr(C)]
pub struct DEVICE_TRIM_DESCRIPTOR {
    pub Version: DWORD,
    pub Size: DWORD,
    pub TrimEnabled: BOOLEAN,
}

/*#[allow(non_snake_case)]
#[repr(C)]
pub struct GET_LENGTH_INFORMATION {
    pub Length: LARGE_INTEGER,
}*/

#[allow(non_snake_case)]
#[repr(C)]
pub struct DISK_GEOMETRY {
    pub Cylinders: LARGE_INTEGER,
    pub MediaType: MEDIA_TYPE,
    pub TracksPerCylinder: DWORD,
    pub SectorsPerTrack: DWORD,
    pub BytesPerSector: DWORD,
}

#[allow(non_snake_case)]
#[repr(C)]
pub struct PDH_FMT_COUNTERVALUE {
    pub CStatus: DWORD,
    pub doubleValue: c_double,
}

/*#[allow(non_snake_case)]
#[repr(C)]
pub struct RTL_USER_PROCESS_PARAMETERS {
    pub Reserved1: [BYTE; 16],
    pub Reserved2: [PVOID; 10],
    pub ImagePathName: UNICODE_STRING,
    pub CommandLine: UNICODE_STRING,
}

#[allow(non_snake_case)]
#[repr(C)]
pub struct PEB {
    pub Reserved1: [BYTE; 2],
    pub BeingDebugged: BYTE,
    pub Reserved2: [BYTE; 1],
    pub Reserved3: [BYTE; 2],
    pub Ldr: *mut c_void,
    pub ProcessParameters: PRTL_USER_PROCESS_PARAMETERS,
    pub Reserved4: [BYTE; 104],
    pub Reserved5: [PVOID; 52],
    pub PostProcessInitRoutine: *mut c_void,
    pub Reserved6: [BYTE; 128],
    pub Reserved7: [PVOID; 1],
    pub SessionId: ULONG,
}

#[allow(non_snake_case)]
#[repr(C)]
pub struct PROCESS_BASIC_INFORMATION {
    pub Reserved1: PVOID,
    pub PebBaseAddress: PPEB,
    pub Reserved2: [PVOID; 2],
    pub UniqueProcessId: ULONG_PTR,
    pub Reserved3: PVOID,
}*/
