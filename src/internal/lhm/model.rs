//! JSON model mirroring `LhmNative.dll`'s snapshot payload. Not every field is
//! consumed by the UI yet, so dead-code warnings are expected.
#![allow(dead_code)]

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub ok: bool,
    #[serde(default)]
    pub error: Option<String>,
    pub timestamp_unix_ms: i64,
    #[serde(default)]
    pub cpu: Cpu,
    #[serde(default)]
    pub memory: Memory,
    #[serde(default)]
    pub gpus: Vec<Gpu>,
    #[serde(default)]
    pub disks: Vec<Disk>,
    #[serde(default)]
    pub nics: Vec<Nic>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cpu {
    pub usage_percent: Option<f32>,
    pub temperature_c: Option<f32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Memory {
    pub usage_percent: Option<f32>,
    pub temperature_c: Option<f32>,
    pub total_bytes: u64,
    pub used_bytes: u64,
    #[serde(default)]
    pub dimm_temperatures_c: Vec<f32>,
}

const GB: f64 = (1 << 30) as f64;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gpu {
    pub name: String,
    pub usage_percent: Option<f32>,
    pub temperature_c: Option<f32>,
    pub vram_total_bytes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Disk {
    pub name: String,
    pub activity_percent: Option<f32>,
    pub temperature_c: Option<f32>,
    pub read_bytes_per_sec: f64,
    pub write_bytes_per_sec: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Nic {
    pub name: String,
    pub ip: String,
    pub upload_bytes_per_sec: f64,
    pub download_bytes_per_sec: f64,
}
