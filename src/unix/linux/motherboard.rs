// Take a look at the license at the top of the repository in the LICENSE file.

use crate::Error;
use std::fs::{read, read_dir, read_to_string};
use std::path::Path;

pub(crate) struct MotherboardInner;

impl MotherboardInner {
    pub(crate) fn new() -> Result<Self, Error> {
        Ok(Self)
    }

    pub(crate) fn asset_tag(&self) -> Option<String> {
        read_to_string("/sys/devices/virtual/dmi/id/board_asset_tag")
            .ok()
            .map(|s| s.trim().to_owned())
    }

    pub(crate) fn name(&self) -> Option<String> {
        read_to_string("/sys/devices/virtual/dmi/id/board_name")
            .ok()
            .or_else(|| {
                read_to_string("/proc/device-tree/board")
                    .ok()
                    .or_else(|| Some(parse_device_tree_compatible()?.1))
            })
            .map(|s| s.trim().to_owned())
    }

    pub(crate) fn vendor_name(&self) -> Option<String> {
        read_to_string("/sys/devices/virtual/dmi/id/board_vendor")
            .ok()
            .or_else(|| Some(parse_device_tree_compatible()?.0))
            .map(|s| s.trim().to_owned())
    }

    pub(crate) fn version(&self) -> Option<String> {
        read_to_string("/sys/devices/virtual/dmi/id/board_version")
            .ok()
            .map(|s| s.trim().to_owned())
    }

    pub(crate) fn serial_number(&self) -> Option<String> {
        read_to_string("/sys/devices/virtual/dmi/id/board_serial")
            .ok()
            .map(|s| s.trim().to_owned())
    }

    pub(crate) fn temperatures(&self) -> Vec<f32> {
        read_motherboard_temperatures()
    }
}

fn read_motherboard_temperatures() -> Vec<f32> {
    let hwmon_root = Path::new("/sys/class/hwmon");
    let Ok(entries) = read_dir(hwmon_root) else {
        return Vec::new();
    };

    let mut temps = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = read_to_string(path.join("name")).ok();
        if name.as_deref().map(|n| n.trim()) != Some("acpitz") {
            continue;
        }

        let Ok(sensors) = read_dir(&path) else {
            continue;
        };
        for sensor in sensors.flatten() {
            let fname = sensor.file_name();
            let fname_str = fname.to_string_lossy();
            if fname_str.starts_with("temp")
                && fname_str.ends_with("_input")
                && let Ok(raw) = read_to_string(sensor.path())
                && let Ok(milli) = raw.trim().parse::<i32>()
            {
                temps.push(milli as f32 / 1000.0);
            }
        }
    }

    temps
}

// Parses the first entry of the file `/proc/device-tree/compatible`, to extract the vendor and
// motherboard name. This file contains several `\0` separated strings; the first one include the
// vendor and the motherboard name, separated by a comma.
//
// According to the specification: https://github.com/devicetree-org/devicetree-specification
// a compatible string must contain only one comma.
fn parse_device_tree_compatible() -> Option<(String, String)> {
    let bytes = read("/proc/device-tree/compatible").ok()?;
    let first_line = bytes.split(|&b| b == 0).next()?;
    std::str::from_utf8(first_line)
        .ok()?
        .split_once(',')
        .map(|(a, b)| (a.to_owned(), b.to_owned()))
}
