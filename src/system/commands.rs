use std::process::Command as ProcessCommand;

#[derive(Debug, serde::Serialize)]
pub struct ExecutedCommand {
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub result: String,
}

#[cfg(target_os = "windows")]
pub fn execute_command(command: &str) -> ExecutedCommand {
    let output_result = ProcessCommand::new("cmd.exe").args(["/C", command]).output();
    let (stdout, stderr, result) = 
        match output_result {
            Ok(output) => (
                String::from_utf8_lossy(&output.stdout).to_string(),
                String::from_utf8_lossy(&output.stderr).to_string(),
                "Ok".to_string(),
            ),
            Err(e) => (
                String::new(),
                format!("{}", e),
                "Err".to_string()),
    };

    return ExecutedCommand {
        command: command.to_string(),
        stdout,
        stderr,
        result,
    };
}

#[cfg(not(target_os = "windows"))]
pub fn execute_command(command: &str) -> ExecutedCommand {
    let output_result = ProcessCommand::new("sh").arg("-c").arg(command).output();
    let (stdout, stderr, result) = match output_result {
        Ok(output) => (
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
            "Ok".to_string(),
        ),
        Err(e) => (String::new(), String::new(), format!("Error: {}", e)),
    };

    ExecutedCommand {
        command: command.to_string(),
        stdout,
        stderr,
        result,
    }
}
