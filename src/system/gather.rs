use chrono::Local;
use os_info;
use std::env;
use whoami;
use uuid::Uuid;
use machine_uid;
use super::network::get_interfaces;

// Hardware ID generation
// Use from_u128 for const context
const HARDWARE_NAMESPACE: Uuid = Uuid::from_u128(0x6ba7b8109dad11d180b400c04fd430c8);
fn get_hardware_uuid() -> Result<Uuid, Box<dyn std::error::Error>> {
    let hwid = machine_uid::get()?;
    Ok(Uuid::new_v5(&HARDWARE_NAMESPACE, hwid.as_bytes()))
}

#[derive(Debug, serde::Serialize)]
pub struct SystemData {
    pub uuid: Uuid,
    pub hostname: String,
    pub os_type: String,
    pub os_version: String,
    pub interfaces: Vec<super::network::InterfaceInfo>,
    pub username: String,
    pub is_admin_user: bool,
    pub is_admin_process: bool,
    pub host_time: String,
}

pub fn system_data() -> SystemData {
    let info = os_info::get();
    let hostname = whoami::fallible::hostname().unwrap_or_else(|_| "Unknown".into());
    let username = whoami::username();
    let host_time = Local::now().format("%d/%m/%Y %H:%M:%S").to_string();
    let hwid = get_hardware_uuid().unwrap_or_else(|_| Uuid::nil());

    SystemData {
        uuid: hwid,
        hostname,
        os_type: info.os_type().to_string(),
        os_version: info.version().to_string(),
        interfaces: get_interfaces(),
        username,
        is_admin_user: false,
        is_admin_process: false,
        host_time,
    }
}