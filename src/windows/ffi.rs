//
// Sysinfo
//
// Copyright (c) 2020 Guillaume Gomez
//

// TO BE REMOVED ONCE https://github.com/retep998/winapi-rs/pull/802 IS MERGED!!!

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use winapi::shared::basetsd::ULONG64;
use winapi::shared::guiddef::GUID;
use winapi::shared::ifdef::{NET_IFINDEX, NET_LUID};
use winapi::shared::minwindef::BYTE;
use winapi::shared::netioapi::NETIOAPI_API;
use winapi::shared::ntdef::{PVOID, UCHAR, ULONG, WCHAR};
use winapi::{ENUM, STRUCT};

const ANY_SIZE: usize = 1;

pub const IF_MAX_STRING_SIZE: usize = 256;
pub const IF_MAX_PHYS_ADDRESS_LENGTH: usize = 32;

pub type NET_IF_NETWORK_GUID = GUID;
pub type PMIB_IF_TABLE2 = *mut MIB_IF_TABLE2;
pub type PMIB_IF_ROW2 = *mut MIB_IF_ROW2;

macro_rules! BITFIELD {
    ($base:ident $field:ident: $fieldtype:ty [
        $($thing:ident $set_thing:ident[$r:expr],)+
    ]) => {
        impl $base {$(
            #[inline]
            pub fn $thing(&self) -> $fieldtype {
                let size = ::std::mem::size_of::<$fieldtype>() * 8;
                self.$field << (size - $r.end) >> (size - $r.end + $r.start)
            }
            #[inline]
            pub fn $set_thing(&mut self, val: $fieldtype) {
                let mask = ((1 << ($r.end - $r.start)) - 1) << $r.start;
                self.$field &= !mask;
                self.$field |= (val << $r.start) & mask;
            }
        )+}
    }
}

STRUCT! {struct MIB_IF_TABLE2 {
    NumEntries: ULONG,
    Table: [MIB_IF_ROW2; ANY_SIZE],
}}

ENUM! {enum NDIS_MEDIUM {
    NdisMedium802_3 = 0,
    NdisMedium802_5 = 1,
    NdisMediumFddi = 2,
    NdisMediumWan = 3,
    NdisMediumLocalTalk = 4,
    NdisMediumDix = 5, // defined for convenience, not a real medium
    NdisMediumArcnetRaw = 6,
    NdisMediumArcnet878_2 = 7,
    NdisMediumAtm = 8,
    NdisMediumWirelessWan = 9,
    NdisMediumIrda = 10,
    NdisMediumBpc = 11,
    NdisMediumCoWan = 12,
    NdisMedium1394 = 13,
    NdisMediumInfiniBand = 14,
    NdisMediumTunnel = 15,
    NdisMediumNative802_11 = 16,
    NdisMediumLoopback = 17,
    NdisMediumWiMAX = 18,
    NdisMediumIP = 19,
    NdisMediumMax = 20, // Not a real medium, defined as an upper-bound
}}

ENUM! {enum TUNNEL_TYPE {
    TUNNEL_TYPE_NONE = 0,
    TUNNEL_TYPE_OTHER = 1,
    TUNNEL_TYPE_DIRECT = 2,
    TUNNEL_TYPE_6TO4 = 11,
    TUNNEL_TYPE_ISATAP = 13,
    TUNNEL_TYPE_TEREDO = 14,
    TUNNEL_TYPE_IPHTTPS = 15,
}}

ENUM! {enum NDIS_PHYSICAL_MEDIUM {
    NdisPhysicalMediumUnspecified = 0,
    NdisPhysicalMediumWirelessLan = 1,
    NdisPhysicalMediumCableModem = 2,
    NdisPhysicalMediumPhoneLine = 3,
    NdisPhysicalMediumPowerLine = 4,
    NdisPhysicalMediumDSL = 5, // includes ADSL and UADSL (G.Lite)
    NdisPhysicalMediumFibreChannel = 6,
    NdisPhysicalMedium1394 = 7,
    NdisPhysicalMediumWirelessWan = 8,
    NdisPhysicalMediumNative802_11 = 9,
    NdisPhysicalMediumBluetooth = 10,
    NdisPhysicalMediumInfiniband = 11,
    NdisPhysicalMediumWiMax = 12,
    NdisPhysicalMediumUWB = 13,
    NdisPhysicalMedium802_3 = 14,
    NdisPhysicalMedium802_5 = 15,
    NdisPhysicalMediumIrda = 16,
    NdisPhysicalMediumWiredWAN = 17,
    NdisPhysicalMediumWiredCoWan = 18,
    NdisPhysicalMediumOther = 19,
    NdisPhysicalMediumMax = 20, // Not a real physical type, defined as an upper-bound
}}

ENUM! {enum NET_IF_ACCESS_TYPE {
    NET_IF_ACCESS_LOOPBACK = 1,
    NET_IF_ACCESS_BROADCAST = 2,
    NET_IF_ACCESS_POINT_TO_POINT = 3,
    NET_IF_ACCESS_POINT_TO_MULTI_POINT = 4,
    NET_IF_ACCESS_MAXIMUM = 5,
}}

ENUM! {enum NET_IF_DIRECTION_TYPE {
    NET_IF_DIRECTION_SENDRECEIVE = 0,
    NET_IF_DIRECTION_SENDONLY = 1,
    NET_IF_DIRECTION_RECEIVEONLY = 2,
    NET_IF_DIRECTION_MAXIMUM = 3,
}}

ENUM! {enum IF_OPER_STATUS {
    IfOperStatusUp = 1,
    IfOperStatusDown = 2,
    IfOperStatusTesting = 3,
    IfOperStatusUnknown = 4,
    IfOperStatusDormant = 5,
    IfOperStatusNotPresent = 6,
    IfOperStatusLowerLayerDown = 7,
}}

ENUM! {enum NET_IF_ADMIN_STATUS {
    NET_IF_ADMIN_STATUS_UP = 1,
    NET_IF_ADMIN_STATUS_DOWN = 2,
    NET_IF_ADMIN_STATUS_TESTING = 3,
}}

ENUM! {enum NET_IF_MEDIA_CONNECT_STATE {
    MediaConnectStateUnknown = 0,
    MediaConnectStateConnected = 1,
    MediaConnectStateDisconnected = 2,
}}

ENUM! {enum NET_IF_CONNECTION_TYPE {
    NET_IF_CONNECTION_DEDICATED = 1,
    NET_IF_CONNECTION_PASSIVE = 2,
    NET_IF_CONNECTION_DEMAND = 3,
    NET_IF_CONNECTION_MAXIMUM = 4,
}}

STRUCT! {struct MIB_IF_ROW2_InterfaceAndOperStatusFlags {
    bitfield: BYTE,
}}
BITFIELD! {MIB_IF_ROW2_InterfaceAndOperStatusFlags bitfield: BYTE [
    HardwareInterface set_HardwareInterface[0..1],
    FilterInterface set_FilterInterface[1..2],
    ConnectorPresent set_ConnectorPresent[2..3],
    NotAuthenticated set_NotAuthenticated[3..4],
    NotMediaConnected set_NotMediaConnected[4..5],
    Paused set_Paused[5..6],
    LowPower set_LowPower[6..7],
    EndPointInterface set_EndPointInterface[7..8],
]}

STRUCT! {struct MIB_IF_ROW2 {
    InterfaceLuid: NET_LUID,
    InterfaceIndex: NET_IFINDEX,
    InterfaceGuid: GUID,
    Alias: [WCHAR; IF_MAX_STRING_SIZE + 1],
    Description: [WCHAR; IF_MAX_STRING_SIZE + 1],
    PhysicalAddressLength: ULONG,
    PhysicalAddress: [UCHAR; IF_MAX_PHYS_ADDRESS_LENGTH],
    PermanentPhysicalAddress: [UCHAR; IF_MAX_PHYS_ADDRESS_LENGTH],
    Mtu: ULONG,
    Type: ULONG, // Interface Type.
    TunnelType: TUNNEL_TYPE, // Tunnel Type, if Type = IF_TUNNEL.
    MediaType: NDIS_MEDIUM,
    PhysicalMediumType: NDIS_PHYSICAL_MEDIUM,
    AccessType: NET_IF_ACCESS_TYPE,
    DirectionType: NET_IF_DIRECTION_TYPE,
    InterfaceAndOperStatusFlags: MIB_IF_ROW2_InterfaceAndOperStatusFlags,
    OperStatus: IF_OPER_STATUS,
    AdminStatus: NET_IF_ADMIN_STATUS,
    MediaConnectState: NET_IF_MEDIA_CONNECT_STATE,
    NetworkGuid: NET_IF_NETWORK_GUID,
    ConnectionType: NET_IF_CONNECTION_TYPE,
    TransmitLinkSpeed: ULONG64,
    ReceiveLinkSpeed: ULONG64,
    InOctets: ULONG64,
    InUcastPkts: ULONG64,
    InNUcastPkts: ULONG64,
    InDiscards: ULONG64,
    InErrors: ULONG64,
    InUnknownProtos: ULONG64,
    InUcastOctets: ULONG64,
    InMulticastOctets: ULONG64,
    InBroadcastOctets: ULONG64,
    OutOctets: ULONG64,
    OutUcastPkts: ULONG64,
    OutNUcastPkts: ULONG64,
    OutDiscards: ULONG64,
    OutErrors: ULONG64,
    OutUcastOctets: ULONG64,
    OutMulticastOctets: ULONG64,
    OutBroadcastOctets: ULONG64,
    OutQLen: ULONG64,
}}

// To be removed once https://github.com/retep998/winapi-rs/pull/872 is merged
use winapi::shared::lmcons::NET_API_STATUS;
use winapi::shared::minwindef::{DWORD, LPBYTE, LPDWORD};
use winapi::um::winnt::LPCWSTR;

extern "system" {
    pub fn GetIfTable2(Table: *mut PMIB_IF_TABLE2) -> NETIOAPI_API;
    pub fn GetIfEntry2(Row: PMIB_IF_ROW2) -> NETIOAPI_API;
    pub fn FreeMibTable(Memory: PVOID);
    // To be removed once https://github.com/retep998/winapi-rs/pull/872 is merged
    pub fn NetUserGetLocalGroups(
        servername: LPCWSTR,
        username: LPCWSTR,
        level: DWORD,
        flags: DWORD,
        bufptr: *mut LPBYTE,
        prefmaxlen: DWORD,
        entriesread: LPDWORD,
        totalentries: LPDWORD,
    ) -> NET_API_STATUS;
}
