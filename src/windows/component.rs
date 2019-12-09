//
// Sysinfo
//
// Copyright (c) 2018 Guillaume Gomez
//

use ComponentExt;

/// Struct containing a component information (temperature and name for the moment).
pub struct Component {
    temperature: f32,
    max: f32,
    critical: Option<f32>,
    label: String,
}

impl Component {
    /// Creates a new `Component` with the given information.
    pub fn new(label: String, max: Option<f32>, critical: Option<f32>) -> Component {
        Component {
            temperature: 0f32,
            label: label,
            max: max.unwrap_or(0.0),
            critical: critical,
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

pub fn get_components() -> Vec<Component> {
    Connection::new()
        .and_then(|x| x.initialize_security())
        .and_then(|x| x.create_instance())
        .and_then(|x| x.connect_server())
        .and_then(|x| x.set_proxy_blanket())
        .and_then(|x| x.exec_query())
        .and_then(|x| {
            x.iterate();
            Some(())
        });
    Vec::new()
}

use std::ptr::null_mut;

use winapi::shared::rpcdce::{RPC_C_AUTHN_LEVEL_DEFAULT, RPC_C_IMP_LEVEL_IMPERSONATE, RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE, RPC_C_AUTHN_LEVEL_CALL};
use winapi::shared::winerror::FAILED;
use winapi::shared::wtypesbase::CLSCTX_INPROC_SERVER;
use winapi::um::combaseapi::{CoCreateInstance, CoInitializeEx, CoUninitialize, CoInitializeSecurity, CoSetProxyBlanket};
use winapi::um::objbase::COINIT_MULTITHREADED;
use winapi::um::objidl::EOAC_NONE;
use winapi::um::oleauto::SysAllocString;
use winapi::um::oleauto::VariantClear;
use winapi::um::wbemcli::{CLSID_WbemLocator, IEnumWbemClassObject, IID_IWbemLocator, IWbemLocator, IWbemServices, WBEM_S_NO_ERROR, WBEM_FLAG_NONSYSTEM_ONLY, WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_FLAG_FORWARD_ONLY};
use winapi::shared::wtypes::BSTR;
use winapi::um::oleauto::SysFreeString;
use winapi::um::wbemcli::IWbemClassObject;
 use winapi::um::oaidl::VARIANT_n3;

struct Instance(*mut IWbemLocator);

impl Drop for Instance {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { (*self.0).Release(); }
        }
    }
}

struct ServerConnection(*mut IWbemServices);

impl Drop for ServerConnection {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { (*self.0).Release(); }
        }
    }
}

struct Enumerator(*mut IEnumWbemClassObject);

impl Drop for Enumerator {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { (*self.0).Release(); }
        }
    }
}

macro_rules! bstr {
    ($($x:expr),*) => {{
        let x: &[u16] = &[$($x as u16),*];
        SysAllocString(x.as_ptr())
    }}
}

struct Connection {
    instance: Option<Instance>,
    server_connection: Option<ServerConnection>,
    enumerator: Option<Enumerator>,
}

impl Connection {
    fn new() -> Option<Connection> {
        let x = unsafe { CoInitializeEx(null_mut(), 0) };
        if FAILED(x) {
            eprintln!("1 => {}", x);
            None
        } else {
            Some(Connection {
                instance: None,
                server_connection: None,
                enumerator: None,
            })
        }
    }

    fn initialize_security(self) -> Option<Connection> {
        if FAILED(unsafe {
            CoInitializeSecurity(
                null_mut(),
                -1,
                null_mut(),
                null_mut(),
                RPC_C_AUTHN_LEVEL_DEFAULT,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                null_mut(),
                EOAC_NONE,
                null_mut(),
            )
        }) {
            eprintln!("2");
            None
        } else {
            Some(self)
        }
    }

    fn create_instance(mut self) -> Option<Connection> {
        let mut pLoc = null_mut();

        if FAILED(unsafe {
            CoCreateInstance(
                *(&CLSID_WbemLocator as *const _ as *const usize) as *mut _,
                null_mut(),
                CLSCTX_INPROC_SERVER,
                *(&IID_IWbemLocator as *const _ as *const usize) as *mut _,
                &mut pLoc as *mut _ as *mut _,
            )
        }) {
            eprintln!("3");
            None
        } else {
            self.instance = Some(Instance(pLoc));
            Some(self)
        }
    }

    fn connect_server(mut self) -> Option<Connection> {
        let mut pSvc = null_mut();

        if let Some(ref instance) = self.instance {
            unsafe {
                let s = bstr!('R', 'O', 'O', 'T', '\\', 'W', 'M', 'I');
                let res = (*instance.0).ConnectServer(
                    s,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    0,
                    null_mut(),
                    null_mut(),
                    &mut pSvc as *mut _,
                );
                SysFreeString(s);
                if FAILED(res) {
                    eprintln!("4");
                    return None;
                }
            }
        } else {
            eprintln!("5");
            return None;
        }
        self.server_connection = Some(ServerConnection(pSvc));
        Some(self)
    }

    fn set_proxy_blanket(self) -> Option<Connection> {
        if let Some(ref server_connection) = self.server_connection {
            unsafe {
                if FAILED(CoSetProxyBlanket(
                   server_connection.0 as *mut _,
                   RPC_C_AUTHN_WINNT,
                   RPC_C_AUTHZ_NONE,
                   null_mut(),
                   RPC_C_AUTHN_LEVEL_CALL,
                   RPC_C_IMP_LEVEL_IMPERSONATE,
                   null_mut(),
                   EOAC_NONE,
                )) {
                    eprintln!("6");
                    return None;
                }
            }
        } else {
            eprintln!("7");
            return None;
        }
        Some(self)
    }

    fn exec_query(mut self) -> Option<Connection> {
        let mut pEnumerator = null_mut();

        if let Some(ref server_connection) = self.server_connection {
            unsafe {
                let s = bstr!('W', 'Q', 'L'); // query kind
                let query = bstr!('S','E','L','E','C','T',' ','*',' ','F','R','O','M',' ','M','S','A','c','p','i','_','T','h','e','r','m','a','l','Z','o','n','e','T','e','m','p','e','r','a','t','u','r','e');
                let hres = (*server_connection.0).ExecQuery(
                    s,
                    query,
                    (WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY) as _,
                    null_mut(),
                    &mut pEnumerator as *mut _,
                );
                SysFreeString(s);
                SysFreeString(query);
                if FAILED(hres) {
                    eprintln!("8");
                    return None;
                }
            }
        } else {
            eprintln!("9");
            return None;
        }
        self.enumerator = Some(Enumerator(pEnumerator));
        Some(self)
    }

    fn iterate(mut self) {
        let pEnum = match self.enumerator.take() {
            Some(x) => x,
            None => {
                eprintln!("10");
                return;
            }
        };
        loop {
            let mut pObj: *mut IWbemClassObject = null_mut();
            let mut uReturned = 0;

            let hRes = unsafe {
                (*pEnum.0).Next(
                    0,                  // Time out
                    1,                  // One object
                    &mut pObj as *mut _,
                    &mut uReturned,
                )
            };

            if uReturned == 0 {
                eprintln!("and done!");
                break;
            }

            unsafe {
                (*pObj).BeginEnumeration(WBEM_FLAG_NONSYSTEM_ONLY as _);

                let mut pVal: VARIANT_n3 = ::std::mem::uninitialized();
                let mut pstrName: BSTR = null_mut();

                while (*pObj).Next(0, &mut pstrName, &mut pVal as *mut _ as *mut _, null_mut(), null_mut()) == WBEM_S_NO_ERROR as _ {
                    {
                        let mut i = 0;
                        while (*pstrName.offset(i)) != 0 {
                            i += 1;
                        }
                        let bytes = ::std::slice::from_raw_parts(pstrName as *const u16, i as usize + 1);
                        i = 0;
                        while (*pVal.bstrVal().offset(i)) != 0 {
                            i += 1;
                        }
                        let bytes2 = ::std::slice::from_raw_parts(*pVal.bstrVal() as *const u16, i as usize + 1);
                        println!("==> {}::{}", String::from_utf16_lossy(bytes), String::from_utf16_lossy(bytes2));
                    }
                    SysFreeString(pstrName);
                    VariantClear(&mut pVal as *mut _ as *mut _);
                }
            }

            unsafe { (*pObj).Release(); }
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        unsafe { CoUninitialize(); }
    }
}
