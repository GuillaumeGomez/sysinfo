#![allow(clippy::too_many_arguments)]

use std::fs::{read_dir, File};
use std::io::Read;
use std::path::Path;

use crate::GpuExt;

#[doc = include_str!("../../md_doc/gpu.md")]
pub struct Gpu {
    name: String,
    gpu_usage: Option<f32>,
    vram_used: Option<u64>,
    vram_total: Option<u64>,
    freq: Option<u64>,
    freq_max: Option<u64>,
    vendor_id: String,
    brand: Option<String>,
}

impl Gpu {
    /// Find all the GPUs in the system.
    pub fn get_gpus() -> Vec<Self> {
        let mut gpus = Vec::new();
        if let Ok(dir) = read_dir(&Path::new("/sys/class/drm/")) {
            for entry in dir.flatten() {
                let entry = entry.path();
                if !entry.is_dir() {
                    continue;
                }
                let filename = entry.file_name().and_then(|x| x.to_str()).unwrap_or("");
                if filename.starts_with("card") && !filename.contains('-') {
                    let gpu: Option<Self> = match get_vendor_id(&entry.join("device")) {
                        Some(id) => match id.as_str() {
                            "8086" => Self::get_intel_gpu_info(&entry),
                            "10de" => Self::get_nvidia_gpu_info(&entry),
                            "1002" => Self::get_amd_gpu_info(&entry),
                            _ => None,
                        },
                        None => None,
                    };
                    if let Some(gpu_info) = gpu {
                        gpus.push(gpu_info);
                    }
                }
            }
        }
        gpus
    }

    fn get_intel_gpu_info(path: &Path) -> Option<Self> {
        let vendor_id = String::from("Intel");
        let name: String = path.file_name().unwrap().to_str().unwrap().to_string();

        let cur_freq: u64 = match File::open(path.join("gt_cur_freq_mhz")) {
            Ok(mut f) => {
                let mut s = String::new();
                f.read_to_string(&mut s).unwrap();
                s.trim().split('\n').next().unwrap().parse::<u64>().unwrap()
            }
            Err(_) => return None,
        };

        let max_freq: u64 = match File::open(path.join("gt_max_freq_mhz")) {
            Ok(mut f) => {
                let mut s = String::new();
                f.read_to_string(&mut s).unwrap();
                s.trim().split('\n').next().unwrap().parse::<u64>().unwrap()
            }
            Err(_) => return None,
        };

        let gpu = Gpu {
            name,
            gpu_usage: None,
            vram_used: None,
            vram_total: None,
            freq: Some(cur_freq),
            freq_max: Some(max_freq),
            vendor_id,
            brand: None,
        };

        Some(gpu)
    }

    fn get_amd_gpu_info(path: &Path) -> Option<Self> {
        let vendor_id = String::from("AMD");
        let name: String = path.file_name().unwrap().to_str().unwrap().to_string();

        let gpu_usage: f32 = match File::open(path.join("device/gpu_busy_percent")) {
            Ok(mut f) => {
                let mut s = String::new();
                f.read_to_string(&mut s).unwrap();
                s.trim().split('\n').next().unwrap().parse::<f32>().unwrap()
            }
            Err(_) => return None,
        };

        let vram_used: u64 = match File::open(path.join("device/mem_info_vram_used")) {
            Ok(mut f) => {
                let mut s = String::new();
                f.read_to_string(&mut s).unwrap();
                s.trim().split('\n').next().unwrap().parse::<u64>().unwrap()
            }
            Err(_) => return None,
        };

        let vram_total: u64 = match File::open(path.join("device/mem_info_vram_total")) {
            Ok(mut f) => {
                let mut s = String::new();
                f.read_to_string(&mut s).unwrap();
                s.trim().split('\n').next().unwrap().parse::<u64>().unwrap()
            }
            Err(_) => return None,
        };

        let gpu = Gpu {
            name,
            gpu_usage: Some(gpu_usage),
            vram_used: Some(vram_used / 1024),
            vram_total: Some(vram_total / 1024),
            freq: None,
            freq_max: None,
            vendor_id,
            brand: None,
        };

        Some(gpu)
    }

    fn get_nvidia_gpu_info(path: &Path) -> Option<Self> {
        let vendor_id = String::from("NVIDIA");
        let name: String = path.file_name().unwrap().to_str().unwrap().to_string();

        let gpu = Gpu {
            name,
            gpu_usage: None,
            vram_used: None,
            vram_total: None,
            freq: None,
            freq_max: None,
            vendor_id,
            brand: None,
        };

        Some(gpu)
    }
}

impl GpuExt for Gpu {
    fn name(&self) -> &str {
        &self.name
    }

    fn gpu_usage(&self) -> Option<f32> {
        self.gpu_usage
    }

    fn vram_used(&self) -> Option<u64> {
        self.vram_used
    }

    fn vram_total(&self) -> Option<u64> {
        self.vram_total
    }

    fn freq(&self) -> Option<u64> {
        self.freq
    }

    fn freq_max(&self) -> Option<u64> {
        self.freq_max
    }

    fn vendor_id(&self) -> String {
        self.vendor_id.clone()
    }

    fn brand(&self) -> Option<String> {
        self.brand.clone()
    }
}

fn get_vendor_id(path: &Path) -> Option<String> {
    let mut vendor_id: String = String::with_capacity(7);
    if let Ok(mut f) = File::open(path.join("vendor")) {
        if f.read_to_string(&mut vendor_id).is_ok() {
            return Some(String::from(&vendor_id[2..6]));
        }
    }
    None
}
