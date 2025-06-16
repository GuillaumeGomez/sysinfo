// Take a look at the license at the top of the repository in the LICENSE file.

#[repr(C, packed)]
pub(crate) struct SMBIOSBaseboardInformation {
    pub(crate) _type: u8,
    pub(crate) length: u8,
    pub(crate) _handle: u16,
    pub(crate) manufacturer: u8,
    pub(crate) product_name: u8,
    pub(crate) version: u8,
    pub(crate) serial_number: u8,
}
