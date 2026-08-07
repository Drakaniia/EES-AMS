/// Count how many EXCEL.EXE processes are currently running without
/// terminating them. Used by the quit-and-verify logic to detect whether
/// Excel actually exited after a successful `Quit()` call.
#[cfg(target_os = "windows")]
pub(crate) fn count_excel_processes() -> u32 {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    let mut count: u32 = 0;

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    let snapshot = match snapshot {
        Ok(handle) => handle,
        Err(_) => return 0,
    };

    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };

    let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry).is_ok() };

    while has_entry {
        let exe_name = PCWSTR::from_raw(entry.szExeFile.as_ptr());
        let exe_name = unsafe { exe_name.to_string() }.unwrap_or_default();
        if exe_name.eq_ignore_ascii_case("EXCEL.EXE") {
            count += 1;
        }
        has_entry = unsafe { Process32NextW(snapshot, &mut entry).is_ok() };
    }

    unsafe {
        let _ = CloseHandle(HANDLE(snapshot.0));
    }

    count
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn count_excel_processes() -> u32 {
    0
}

/// Kill all running EXCEL.EXE processes by terminating them via the Windows
/// ToolHelp API. Returns the number of processes that were terminated.
///
/// This is also used by the `ExcelSession` quit-and-verify prevention logic
/// (Phase 2) to force-kill any Excel processes that failed to exit gracefully.
#[cfg(target_os = "windows")]
pub(crate) fn kill_excel_processes() -> u32 {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

    let mut killed: u32 = 0;

    // SAFETY: CreateToolhelp32Snapshot is a well-known Win32 API with no
    // memory-safety concerns beyond the handle lifecycle.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    let snapshot = match snapshot {
        Ok(handle) => handle,
        Err(_) => {
            log::warn!("failed to create process snapshot, cannot enumerate Excel processes");
            return 0;
        }
    };

    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };

    // SAFETY: Process32FirstW / Process32NextW operate on the snapshot handle.
    // The entry struct is properly sized via dwSize.
    let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry).is_ok() };

    while has_entry {
        let exe_name = PCWSTR::from_raw(entry.szExeFile.as_ptr());
        let exe_name = unsafe { exe_name.to_string() }.unwrap_or_default();

        if exe_name.eq_ignore_ascii_case("EXCEL.EXE") {
            // SAFETY: OpenProcess with PROCESS_TERMINATE on a known PID.
            // The handle is closed immediately after TerminateProcess.
            match unsafe { OpenProcess(PROCESS_TERMINATE, false, entry.th32ProcessID) } {
                Ok(process_handle) if !process_handle.is_invalid() => {
                    // TerminateProcess with exit code 0 — Excel has no
                    // unsaved-work recovery to preserve (per spec).
                    let terminated = unsafe { TerminateProcess(process_handle, 0) };
                    if terminated.is_ok() {
                        killed += 1;
                    } else {
                        log::warn!(
                            "failed to terminate EXCEL.EXE (PID {}): {:?}",
                            entry.th32ProcessID,
                            terminated
                        );
                    }
                    unsafe {
                        let _ = CloseHandle(process_handle);
                    }
                }
                Ok(_) => {
                    // Invalid handle — skip
                }
                Err(err) => {
                    log::warn!(
                        "could not open EXCEL.EXE process (PID {}): {:?}",
                        entry.th32ProcessID,
                        err
                    );
                }
            }
        }

        has_entry = unsafe { Process32NextW(snapshot, &mut entry).is_ok() };
    }

    unsafe {
        let _ = CloseHandle(HANDLE(snapshot.0));
    }

    killed
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn kill_excel_processes() -> u32 {
    0
}
