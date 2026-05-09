/// Tauri commands
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub local_ip: String,
    pub port: u16,
    pub url: String,
}

#[tauri::command]
pub fn get_server_info() -> ServerInfo {
    let local_ip = local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let port = crate::DEFAULT_PORT;
    let url = format!("http://{}:{}", local_ip, port);

    ServerInfo {
        local_ip,
        port,
        url,
    }
}
