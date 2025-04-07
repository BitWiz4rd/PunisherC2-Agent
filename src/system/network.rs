use get_if_addrs::{get_if_addrs, IfAddr};

#[derive(Debug, serde::Serialize)]
pub struct InterfaceInfo {
    pub ip_type: String,
    pub ip_addr: String,
    pub netmask: String,
    pub broadcast: Option<String>,
}

pub fn get_interfaces() -> Vec<InterfaceInfo> {
    let mut interfaces_list = Vec::new();

    if let Ok(ifaces) = get_if_addrs() {
        for iface in ifaces {
            if iface.addr.is_loopback() {
                continue;
            }
            match iface.addr {
                IfAddr::V4(v4) => interfaces_list.push(InterfaceInfo {
                    ip_type: "v4".to_string(),
                    ip_addr: v4.ip.to_string(),
                    netmask: v4.netmask.to_string(),
                    broadcast: v4.broadcast.map(|b| b.to_string()),
                }),
                IfAddr::V6(v6) => interfaces_list.push(InterfaceInfo {
                    ip_type: "v6".to_string(),
                    ip_addr: v6.ip.to_string(),
                    netmask: v6.netmask.to_string(),
                    broadcast: v6.broadcast.map(|b| b.to_string()),
                }),
            }
        }
    }
    interfaces_list
}
