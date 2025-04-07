use serde::Deserialize;
use crate::system::{gather::SystemData, commands::ExecutedCommand};

#[derive(Debug, Deserialize)]
pub struct CheckPendingCommandHTTPResponse {
    pub pending_commands: Vec<String>,
}
