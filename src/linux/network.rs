//
// Sysinfo
//
// Copyright (c) 2019 Guillaume Gomez
//

use std::fs::File;
use std::io::{Error, ErrorKind, Read};

use std::collections::HashMap;
use NetworkExt;

pub struct Networks {
    interfaces: HashMap<String, NetworkData>,
}

macro_rules! old_and_new {
    ($ty_:ident, $name:ident) => {{
        $ty_.old_$name = $name;
        $ty_.$name = $name;
    }}
}

impl Networks {
    pub(crate) fn new() -> Self {
        Networks {
            interfaces: HashMap::new(),
        }
    }

    pub(crate) fn update(&mut self) {
        if let Ok(dir) = std::fs::read_dir("/sys/class/net/") {
            for entry in dir {
                if let Ok(entry) = entry {
                    let parent = entry.path().join("statistics");
                    let read = |path: &str| -> usize {
                        // TODO: check optimization here?
                        std::fs::read_to_string(parent.join(path))
                            .unwrap_or_default()
                            .trim()
                            .parse()
                            .unwrap_or_default()
                    };
                    let rx_bytes = read("rx_bytes");
                    let tx_bytes = read("tx_bytes");
                    let rx_packets = read("rx_packets");
                    let tx_packets = read("tx_packets");
                    let rx_errors = read("rx_errors");
                    let tx_errors = read("tx_errors");
                    let rx_compressed = read("rx_compressed");
                    let tx_compressed = read("tx_compressed");
                    let entry = format!("{}", entry.file_name());
                    let interface = self.interfaces.entry(&entry).or_insert(
                        NetworkData {
                            rx_bytes,
                            old_rx_bytes: rx_bytes,
                            tx_bytes,
                            old_tx_bytes: tx_bytes,
                            rx_packets,
                            old_rx_packets: rx_packets,
                            tx_packets,
                            old_tx_packets: tx_packets,
                            rx_errors,
                            old_rx_errors: rx_errors,
                            tx_errors,
                            old_tx_errors: tx_errors,
                            rx_compressed,
                            old_rx_compressed: rx_compressed,
                            tx_compressed,
                            old_tx_compressed: tx_compressed,
                        }
                    );
                    old_and_new!(interface, rx_bytes);
                    old_and_new!(interface, tx_bytes);
                    old_and_new!(interface, rx_packets);
                    old_and_new!(interface, tx_packets);
                    old_and_new!(interface, rx_errors);
                    old_and_new!(interface, tx_errors);
                    old_and_new!(interface, rx_compressed);
                    old_and_new!(interface, tx_compressed);
                }
            }
        }
    }
}

/// Contains network information.
#[derive(Debug)]
pub struct NetworkData {
    /// Total number of bytes received over interface.
    rx_bytes: usize,
    old_rx_bytes: usize,
    /// Total number of bytes transmitted over interface.
    tx_bytes: usize,
    old_tx_bytes: usize,
    /// Total number of packets received.
    rx_packets: usize,
    old_rx_packets: usize,
    /// Total number of packets transmitted.
    tx_packets: usize,
    old_tx_packets: usize,
    /// Shows the total number of packets received with error. This includes
    /// too-long-frames errors, ring-buffer overflow errors, CRC errors,
    /// frame alignment errors, fifo overruns, and missed packets.
    rx_errors: usize,
    old_rx_errors: usize,
    /// similar to `rx_errors`
    tx_errors: usize,
    old_tx_errors: usize,
    /// Indicates the number of compressed packets received by this
    /// network device. This value might only be relevant for interfaces
    /// that support packet compression (e.g: PPP).
    rx_compressed: usize,
    old_rx_compressed: usize,
    /// Indicates the number of transmitted compressed packets. Note
    /// this might only be relevant for devices that support
    /// compression (e.g: PPP).
    tx_compressed: usize,
    old_tx_compressed: usize,
}

impl NetworkExt for NetworkData {
    fn get_income(&self) -> u64 {
        self.current_in - self.old_in
    }

    fn get_outcome(&self) -> u64 {
        self.current_out - self.old_out
    }
}
