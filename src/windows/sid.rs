// Take a look at the license at the top of the repository in the LICENSE file.

use std::{fmt::Display, str::FromStr};

use winapi::{
    shared::{
        sddl::{ConvertSidToStringSidW, ConvertStringSidToSidW},
        winerror::ERROR_INSUFFICIENT_BUFFER,
    },
    um::{
        errhandlingapi::GetLastError,
        securitybaseapi::{CopySid, GetLengthSid, IsValidSid},
        winbase::{LocalFree, LookupAccountSidW},
        winnt::{SidTypeUnknown, LPWSTR, PSID},
    },
};

use crate::sys::utils::to_str;

#[doc = include_str!("../../md_doc/sid.md")]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sid {
    sid: Vec<u8>,
}

impl Sid {
    /// Creates an `Sid` by making a copy of the given raw SID.
    pub(crate) unsafe fn from_psid(psid: PSID) -> Option<Self> {
        if psid.is_null() {
            return None;
        }

        if IsValidSid(psid) == 0 {
            return None;
        }

        let length = GetLengthSid(psid);

        let mut sid = vec![0; length as usize];

        if CopySid(length, sid.as_mut_ptr() as *mut _, psid) == 0 {
            sysinfo_debug!("CopySid failed: {:?}", GetLastError());
            return None;
        }

        // We are making assumptions about the SID internal structure,
        // and these only hold if the revision is 1
        // https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-sid
        // Namely:
        // 1. SIDs can be compared directly (memcmp).
        // 2. Following from this, to hash a SID we can just hash its bytes.
        // These are the basis for deriving PartialEq, Eq, and Hash.
        // And since we also need PartialOrd and Ord, we might as well derive them
        // too. The default implementation will be consistent with Eq,
        // and we don't care about the actual order, just that there is one.
        // So it should all work out.
        // Why bother with this? Because it makes the implementation that
        // much simpler :)
        assert_eq!(sid[0], 1, "Expected SID revision to be 1");

        Some(Self { sid })
    }

    /// Retrieves the account name of this SID.
    pub(crate) fn account_name(&self) -> Option<String> {
        unsafe {
            let mut name_len = 0;
            let mut domain_len = 0;
            let mut name_use = SidTypeUnknown;

            if LookupAccountSidW(
                std::ptr::null_mut(),
                self.sid.as_ptr() as *mut _,
                std::ptr::null_mut(),
                &mut name_len,
                std::ptr::null_mut(),
                &mut domain_len,
                &mut name_use,
            ) == 0
            {
                let error = GetLastError();
                if error != ERROR_INSUFFICIENT_BUFFER {
                    sysinfo_debug!("LookupAccountSidW failed: {:?}", error);
                    return None;
                }
            }

            let mut name = vec![0; name_len as usize];

            // Reset length to 0 since we're still passing a NULL pointer
            // for the domain.
            domain_len = 0;

            if LookupAccountSidW(
                std::ptr::null_mut(),
                self.sid.as_ptr() as *mut _,
                name.as_mut_ptr(),
                &mut name_len,
                std::ptr::null_mut(),
                &mut domain_len,
                &mut name_use,
            ) == 0
            {
                sysinfo_debug!("LookupAccountSidW failed: {:?}", GetLastError());
                return None;
            }

            Some(to_str(name.as_mut_ptr()))
        }
    }
}

impl Display for Sid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe fn convert_sid_to_string_sid(sid: PSID) -> Option<String> {
            let mut string_sid: LPWSTR = std::ptr::null_mut();
            if ConvertSidToStringSidW(sid, &mut string_sid) == 0 {
                sysinfo_debug!("ConvertSidToStringSidW failed: {:?}", GetLastError());
                return None;
            }
            let result = to_str(string_sid);
            LocalFree(string_sid as *mut _);
            Some(result)
        }

        let string_sid = unsafe { convert_sid_to_string_sid(self.sid.as_ptr() as *mut _) };
        let string_sid = string_sid.ok_or(std::fmt::Error)?;

        write!(f, "{string_sid}")
    }
}

impl FromStr for Sid {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        unsafe {
            let mut string_sid: Vec<u16> = s.encode_utf16().collect();
            string_sid.push(0);

            let mut psid: PSID = std::ptr::null_mut();
            if ConvertStringSidToSidW(string_sid.as_ptr(), &mut psid) == 0 {
                return Err(format!(
                    "ConvertStringSidToSidW failed: {:?}",
                    GetLastError()
                ));
            }
            let sid = Self::from_psid(psid);
            LocalFree(psid as *mut _);

            // Unwrapping because ConvertStringSidToSidW should've performed
            // all the necessary validations. If it returned an invalid SID,
            // we better fail fast.
            Ok(sid.unwrap())
        }
    }
}
