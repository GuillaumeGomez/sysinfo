// Take a look at the license at the top of the repository in the LICENSE file.

// Described in table 17 in the standard:
// https://www.dmtf.org/sites/default/files/standards/documents/DSP0134_3.6.0.pdf
#[repr(u8)]
#[derive(Copy, Clone)]
pub(crate) enum SMBIOSChassisType {
    #[allow(dead_code)]
    NotSpecified = 0,
    Other = 1,
    Unknown = 2,
    Desktop = 3,
    LowProfileDesktop = 4,
    PizzaBox = 5,
    MiniTower = 6,
    Tower = 7,
    Portable = 8,
    Laptop = 9,
    Notebook = 10,
    HandHeld = 11,
    DockingStation = 12,
    AllInOne = 13,
    SubNotebook = 14,
    SpaceSaving = 15,
    LunchBox = 16,
    MainServerChassis = 17,
    ExpansionChassis = 18,
    SubChassis = 19,
    BusExpansionChassis = 20,
    PeripheralChassis = 21,
    RAIDChassis = 22,
    RackMountChassis = 23,
    SealedCasePC = 24,
    MultiSystemChassis = 25,
    CompactPCI = 26,
    AdvancedTCA = 27,
    Blade = 28,
    BladeEnclosure = 29,
    Tablet = 30,
    Convertible = 31,
    Detachable = 32,
    IoTGateway = 33,
    EmbeddedPC = 34,
    MiniPC = 35,
    StickPC = 36,
}

impl TryFrom<u8> for SMBIOSChassisType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            1 => Self::Other,
            2 => Self::Unknown,
            3 => Self::Desktop,
            4 => Self::LowProfileDesktop,
            5 => Self::PizzaBox,
            6 => Self::MiniTower,
            7 => Self::Tower,
            8 => Self::Portable,
            9 => Self::Laptop,
            10 => Self::Notebook,
            11 => Self::HandHeld,
            12 => Self::DockingStation,
            13 => Self::AllInOne,
            14 => Self::SubNotebook,
            15 => Self::SpaceSaving,
            16 => Self::LunchBox,
            17 => Self::MainServerChassis,
            18 => Self::ExpansionChassis,
            19 => Self::SubChassis,
            20 => Self::BusExpansionChassis,
            21 => Self::PeripheralChassis,
            22 => Self::RAIDChassis,
            23 => Self::RackMountChassis,
            24 => Self::SealedCasePC,
            25 => Self::MultiSystemChassis,
            26 => Self::CompactPCI,
            27 => Self::AdvancedTCA,
            28 => Self::Blade,
            29 => Self::BladeEnclosure,
            30 => Self::Tablet,
            31 => Self::Convertible,
            32 => Self::Detachable,
            33 => Self::IoTGateway,
            34 => Self::EmbeddedPC,
            35 => Self::MiniPC,
            36 => Self::StickPC,
            _ => return Err(()),
        })
    }
}

impl TryInto<&'static str> for SMBIOSChassisType {
    type Error = ();

    fn try_into(self) -> Result<&'static str, Self::Error> {
        Ok(match self {
            Self::Other => "Other",
            Self::Unknown => "Unknown",
            Self::Desktop => "Desktop",
            Self::LowProfileDesktop => "Low Profile Desktop",
            Self::PizzaBox => "Pizza Box",
            Self::MiniTower => "Mini Tower",
            Self::Tower => "Tower",
            Self::Portable => "Portable",
            Self::Laptop => "Laptop",
            Self::Notebook => "Notebook",
            Self::HandHeld => "Hand Held",
            Self::DockingStation => "Docking Station",
            Self::AllInOne => "All in One",
            Self::SubNotebook => "Sub Notebook",
            Self::SpaceSaving => "Space-saving",
            Self::LunchBox => "Lunch Box",
            Self::MainServerChassis => "Main Server Chassis",
            Self::ExpansionChassis => "Expansion Chassis",
            Self::SubChassis => "SubChassis",
            Self::BusExpansionChassis => "Bus Expansion Chassis",
            Self::PeripheralChassis => "Peripheral Chassis",
            Self::RAIDChassis => "RAID Chassis",
            Self::RackMountChassis => "Rack Mount Chassis",
            Self::SealedCasePC => "Sealed-case PC",
            Self::MultiSystemChassis => "Multi-system chassis",
            Self::CompactPCI => "Compact PCI",
            Self::AdvancedTCA => "Advanced TCA",
            Self::Blade => "Blade",
            Self::BladeEnclosure => "Blade Enclosure",
            Self::Tablet => "Tablet",
            Self::Convertible => "Convertible",
            Self::Detachable => "Detachable",
            Self::IoTGateway => "IoT Gateway",
            Self::EmbeddedPC => "Embedded PC",
            Self::MiniPC => "Mini PC",
            Self::StickPC => "Stick PC",
            _ => return Err(()),
        })
    }
}
