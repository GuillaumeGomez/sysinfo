// Take a look at the license at the top of the repository in the LICENSE file.

use crate::ComponentExt;

use std::ptr::null_mut;

use winapi::shared::rpcdce::{
    RPC_C_AUTHN_LEVEL_CALL, RPC_C_AUTHN_LEVEL_DEFAULT, RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE,
    RPC_C_IMP_LEVEL_IMPERSONATE,
};
use winapi::shared::winerror::{FAILED, SUCCEEDED, S_FALSE, S_OK};
use winapi::shared::wtypesbase::CLSCTX_INPROC_SERVER;
use winapi::um::combaseapi::{
    CoCreateInstance, CoInitializeEx, CoInitializeSecurity, CoSetProxyBlanket, CoUninitialize,
};
use winapi::um::oaidl::VARIANT;
use winapi::um::objidl::EOAC_NONE;
use winapi::um::oleauto::{SysAllocString, SysFreeString, VariantClear};
use winapi::um::wbemcli::{
    CLSID_WbemLocator, IEnumWbemClassObject, IID_IWbemLocator, IWbemClassObject, IWbemLocator,
    IWbemServices, WBEM_FLAG_FORWARD_ONLY, WBEM_FLAG_NONSYSTEM_ONLY, WBEM_FLAG_RETURN_IMMEDIATELY,
};

#[doc = include_str!("../../md_doc/component.md")]
pub struct Component {
    temperature: f32,
    max: f32,
    critical: Option<f32>,
    label: String,
    connection: Option<Connection>,
}

impl Component {
    /// Creates a new `Component` with the given information.
    fn new() -> Option<Component> {
        let mut c = Connection::new()
            .and_then(|x| x.initialize_security())
            .and_then(|x| x.create_instance())
            .and_then(|x| x.connect_server())
            .and_then(|x| x.set_proxy_blanket())
            .and_then(|x| x.exec_query())?;

        c.temperature(true)
            .map(|(temperature, critical)| Component {
                temperature,
                label: "Computer".to_owned(),
                max: temperature,
                critical,
                connection: Some(c),
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
        if self.connection.is_none() {
            self.connection = Connection::new()
                .and_then(|x| x.initialize_security())
                .and_then(|x| x.create_instance())
                .and_then(|x| x.connect_server())
                .and_then(|x| x.set_proxy_blanket());
        }
        self.connection = if let Some(x) = self.connection.take() {
            x.exec_query()
        } else {
            None
        };
        if let Some(ref mut connection) = self.connection {
            if let Some((temperature, _)) = connection.temperature(false) {
                self.temperature = temperature;
                if self.temperature > self.max {
                    self.max = self.temperature;
                }
            }
        }
    }
}

pub(crate) fn get_components() -> Vec<Component> {
    match Component::new() {
        Some(c) => vec![c],
        None => Vec::new(),
    }
}

struct Instance(*mut IWbemLocator);

impl Drop for Instance {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                (*self.0).Release();
            }
        }
    }
}

struct ServerConnection(*mut IWbemServices);

impl Drop for ServerConnection {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                (*self.0).Release();
            }
        }
    }
}

struct Enumerator(*mut IEnumWbemClassObject);

impl Drop for Enumerator {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                (*self.0).Release();
            }
        }
    }
}

macro_rules! bstr {
    ($($x:expr),*) => {{
        let x: &[u16] = &[$($x as u16),*, 0];
        SysAllocString(x.as_ptr())
    }}
}

struct Connection {
    instance: Option<Instance>,
    server_connection: Option<ServerConnection>,
    enumerator: Option<Enumerator>,
    initialized: bool,
}

#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for Connection {}
unsafe impl Sync for Connection {}

impl Connection {
    #[allow(clippy::unnecessary_wraps)]
    fn new() -> Option<Connection> {
        unsafe {
            let val = CoInitializeEx(null_mut(), 0);
            Some(Connection {
                instance: None,
                server_connection: None,
                enumerator: None,
                initialized: val == S_OK || val == S_FALSE,
            })
        }
    }

    fn initialize_security(self) -> Option<Connection> {
        unsafe {
            if FAILED(CoInitializeSecurity(
                null_mut(),
                -1,
                null_mut(),
                null_mut(),
                RPC_C_AUTHN_LEVEL_DEFAULT,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                null_mut(),
                EOAC_NONE,
                null_mut(),
            )) {
                None
            } else {
                Some(self)
            }
        }
    }

    fn create_instance(mut self) -> Option<Connection> {
        let mut p_loc = null_mut();

        unsafe {
            if FAILED(CoCreateInstance(
                &CLSID_WbemLocator as *const _,
                null_mut(),
                CLSCTX_INPROC_SERVER,
                &IID_IWbemLocator as *const _,
                &mut p_loc as *mut _ as *mut _,
            )) {
                None
            } else {
                self.instance = Some(Instance(p_loc));
                Some(self)
            }
        }
    }

    fn connect_server(mut self) -> Option<Connection> {
        let mut p_svc = null_mut();

        if let Some(ref instance) = self.instance {
            unsafe {
                // "root\WMI"
                let s = bstr!('r', 'o', 'o', 't', '\\', 'W', 'M', 'I');
                let res = (*instance.0).ConnectServer(
                    s,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    0,
                    null_mut(),
                    null_mut(),
                    &mut p_svc as *mut _,
                );
                SysFreeString(s);
                if FAILED(res) {
                    return None;
                }
            }
        } else {
            return None;
        }
        self.server_connection = Some(ServerConnection(p_svc));
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
                    return None;
                }
            }
        } else {
            return None;
        }
        Some(self)
    }

    fn exec_query(mut self) -> Option<Connection> {
        let mut p_enumerator = null_mut();

        if let Some(ref server_connection) = self.server_connection {
            unsafe {
                // "WQL"
                let s = bstr!('W', 'Q', 'L'); // query kind
                                              // "SELECT * FROM MSAcpi_ThermalZoneTemperature"
                let query = bstr!(
                    'S', 'E', 'L', 'E', 'C', 'T', ' ', '*', ' ', 'F', 'R', 'O', 'M', ' ', 'M', 'S',
                    'A', 'c', 'p', 'i', '_', 'T', 'h', 'e', 'r', 'm', 'a', 'l', 'Z', 'o', 'n', 'e',
                    'T', 'e', 'm', 'p', 'e', 'r', 'a', 't', 'u', 'r', 'e'
                );
                let hres = (*server_connection.0).ExecQuery(
                    s,
                    query,
                    (WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY) as _,
                    null_mut(),
                    &mut p_enumerator as *mut _,
                );
                SysFreeString(s);
                SysFreeString(query);
                if FAILED(hres) {
                    return None;
                }
            }
        } else {
            return None;
        }
        self.enumerator = Some(Enumerator(p_enumerator));
        Some(self)
    }

    fn temperature(&mut self, get_critical: bool) -> Option<(f32, Option<f32>)> {
        use winapi::um::wbemcli::WBEM_INFINITE;

        let p_enum = match self.enumerator.take() {
            Some(x) => x,
            None => {
                return None;
            }
        };
        let mut p_obj: *mut IWbemClassObject = null_mut();
        let mut nb_returned = 0;

        unsafe {
            (*p_enum.0).Next(
                WBEM_INFINITE as _, // Time out
                1,                  // One object
                &mut p_obj as *mut _,
                &mut nb_returned,
            );

            if nb_returned == 0 {
                return None; // not enough rights I suppose...
            }

            (*p_obj).BeginEnumeration(WBEM_FLAG_NONSYSTEM_ONLY as _);

            let mut p_val = std::mem::MaybeUninit::<VARIANT>::uninit();
            // "CurrentTemperature"
            let temp = bstr!(
                'C', 'u', 'r', 'r', 'e', 'n', 't', 'T', 'e', 'm', 'p', 'e', 'r', 'a', 't', 'u',
                'r', 'e'
            );
            let res = (*p_obj).Get(temp, 0, p_val.as_mut_ptr(), null_mut(), null_mut());
            let mut p_val = p_val.assume_init();

            SysFreeString(temp);
            VariantClear(&mut p_val as *mut _ as *mut _);

            let temp = if SUCCEEDED(res) {
                // temperature is given in tenth of degrees Kelvin
                (p_val.n1.decVal().Lo64 / 10) as f32 - 273.15
            } else {
                (*p_obj).Release();
                return None;
            };

            let mut critical = None;
            if get_critical {
                // "CriticalPoint"
                let crit = bstr!(
                    'C', 'r', 'i', 't', 'i', 'c', 'a', 'l', 'T', 'r', 'i', 'p', 'P', 'o', 'i', 'n',
                    't'
                );
                let res = (*p_obj).Get(crit, 0, &mut p_val, null_mut(), null_mut());

                SysFreeString(crit);
                VariantClear(&mut p_val as *mut _ as *mut _);

                if SUCCEEDED(res) {
                    // temperature is given in tenth of degrees Kelvin
                    critical = Some((p_val.n1.decVal().Lo64 / 10) as f32 - 273.15);
                }
            }
            (*p_obj).Release();
            Some((temp, critical))
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        // Those three calls are here to enforce that they get dropped in the good order.
        self.enumerator.take();
        self.server_connection.take();
        self.instance.take();
        if self.initialized {
            unsafe {
                CoUninitialize();
            }
        }
    }
}
