/// Tauri commands
use serde::Serialize;
use std::sync::{Arc, Mutex};
use rand;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub local_ip: String,
    pub port: u16,
    pub url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NfcReaderStatus {
    pub connected: bool,
    pub reader_name: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NfcCardData {
    pub serial_number: String,
    pub data: Option<String>,
}

// Global NFC reader state
static NFC_READER: Mutex<Option<Arc<Mutex<NfcReader>>>> = Mutex::new(None);

struct NfcReader {
    connected: bool,
}

impl NfcReader {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // For now, simulate NFC reader detection
        // In a real implementation, this would use PC/SC or USB APIs
        // Return error to simulate no reader connected
        Err("No NFC reader detected".into())
    }

    fn connect_to_reader(&mut self, _reader_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.connected = true;
        Ok(())
    }

    fn read_card(&mut self) -> Result<NfcCardData, Box<dyn std::error::Error>> {
        if !self.connected {
            return Err("NFC reader not connected".into());
        }
        
        // Simulate card reading
        // In a real implementation, this would read from actual NFC hardware
        Ok(NfcCardData {
            serial_number: format!("{:02x}:{:02x}:{:02x}:{:02x}", 
                rand::random::<u8>(), 
                rand::random::<u8>(), 
                rand::random::<u8>(), 
                rand::random::<u8>()),
            data: None,
        })
    }

    fn wait_for_card(&mut self) -> Result<NfcCardData, Box<dyn std::error::Error>> {
        // Simulate waiting for card
        std::thread::sleep(std::time::Duration::from_millis(500));
        self.read_card()
    }
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

#[tauri::command]
pub fn check_nfc_reader() -> Result<NfcReaderStatus, String> {
    match NfcReader::new() {
        Ok(_reader) => {
            Ok(NfcReaderStatus {
                connected: _reader.connected,
                reader_name: Some("Simulated NFC Reader".to_string()),
                error: None,
            })
        }
        Err(e) => Ok(NfcReaderStatus {
            connected: false,
            reader_name: None,
            error: Some(format!("Failed to initialize NFC: {}", e)),
        }),
    }
}

#[tauri::command]
pub fn start_nfc_scanning() -> Result<String, String> {
    let mut reader = NfcReader::new().map_err(|e| e.to_string())?;
    
    reader.connect_to_reader("simulated_reader").map_err(|e| e.to_string())?;
    
    // Store reader globally
    {
        let mut global_reader = NFC_READER.lock().unwrap();
        *global_reader = Some(Arc::new(Mutex::new(reader)));
    }
    
    Ok("Simulated NFC Reader".to_string())
}

#[tauri::command]
pub fn stop_nfc_scanning() -> Result<(), String> {
    let mut global_reader = NFC_READER.lock().unwrap();
    *global_reader = None;
    Ok(())
}

#[tauri::command]
pub fn read_nfc_card() -> Result<NfcCardData, String> {
    let global_reader = NFC_READER.lock().unwrap();
    let reader = global_reader.as_ref()
        .ok_or_else(|| "NFC scanner not started".to_string())?
        .clone();
    
    let mut reader = reader.lock().unwrap();
    reader.wait_for_card().map_err(|e| e.to_string())
}
