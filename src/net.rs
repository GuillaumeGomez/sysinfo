use std::collections::HashMap;

/// NICLoad represents the network interface card load informations
#[derive(Debug)]
pub struct NICLoad {
    /// a total number of bytes received over interface.
    pub rx_bytes: usize,
    /// a total number of bytes transmitted over interface.
    pub tx_bytes: usize,
    /// a total number of packets received.
    pub rx_packets: usize,
    /// a total number of packets transmitted.
    pub tx_packets: usize,
    /// shows a total number of packets received with error. This includes
    /// too-long-frames errors, ring-buffer overflow errors, CRC errors,
    /// frame alignment errors, fifo overruns, and missed packets.
    pub rx_errors: usize,
    /// similar to `rx_errors`
    pub tx_errors: usize,
    /// Indicates the number of compressed packets received by this
    ///	network device. This value might only be relevant for interfaces
    ///	that support packet compression (e.g: PPP).
    pub rx_compressed: usize,
    /// Indicates the number of transmitted compressed packets. Note
    ///	this might only be relevant for devices that support
    ///	compression (e.g: PPP).
    pub tx_compressed: usize,
}

impl NICLoad {
    /// Returns the current network interfaces card statistics
    ///
    /// # Notes
    ///
    /// Current don't support non-unix operating system
    #[cfg(not(unix))]
    pub fn current() -> HashMap<String, NICLoad> {
        HashMap::new()
    }

    /// Returns the current network interfaces card statistics
    #[cfg(unix)]
    pub fn current() -> HashMap<String, NICLoad> {
        let mut result = HashMap::new();
        if let Ok(dir) = std::fs::read_dir("/sys/class/net/") {
            for entry in dir {
                if let Ok(entry) = entry {
                    let parent = entry.path().join("statistics");
                    let read = |path: &str| -> usize {
                        std::fs::read_to_string(parent.join(path))
                            .unwrap_or_default()
                            .trim()
                            .parse()
                            .unwrap_or_default()
                    };
                    let load = NICLoad {
                        rx_bytes: read("rx_bytes"),
                        tx_bytes: read("tx_bytes"),
                        rx_packets: read("rx_packets"),
                        tx_packets: read("tx_packets"),
                        rx_errors: read("rx_errors"),
                        tx_errors: read("tx_errors"),
                        rx_compressed: read("rx_compressed"),
                        tx_compressed: read("tx_compressed"),
                    };
                    result.insert(format!("{:?}", entry.file_name()), load);
                }
            }
        }
        result
    }
}
