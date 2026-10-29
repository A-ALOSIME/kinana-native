// Windows Native OS integration helpers

const KINANA_CRED_TARGET: &str = "kinana_vault_key\0";
const KINANA_RECOVERY_CRED_TARGET: &str = "kinana_recovery_code\0";

/// Native Windows Clipboard helper with security formats.
/// Excludes copied passwords from Windows 10/11 Clipboard History (Win + V)
/// and prevents syncing to Microsoft Cloud Clipboard.
pub unsafe fn copy_to_windows_clipboard_secure(text: &str) -> Result<(), String> {
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
    };
    use windows_sys::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, GMEM_ZEROINIT,
    };

    if OpenClipboard(std::ptr::null_mut()) == 0 {
        return Err("Failed to open Windows clipboard".to_string());
    }

    EmptyClipboard();

    // 1. Exclude from Windows 10/11 Clipboard History (Win + V)
    let hist_fmt_name: Vec<u16> = "CanIncludeInClipboardHistory\0".encode_utf16().collect();
    let cf_history = RegisterClipboardFormatW(hist_fmt_name.as_ptr());
    if cf_history != 0 {
        let hmem = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, std::mem::size_of::<u32>());
        if !hmem.is_null() {
            SetClipboardData(cf_history, hmem as _);
        }
    }

    // 2. Prevent uploading/syncing to Microsoft Cloud Clipboard
    let cloud_fmt_name: Vec<u16> = "CanUploadToCloudClipboard\0".encode_utf16().collect();
    let cf_cloud = RegisterClipboardFormatW(cloud_fmt_name.as_ptr());
    if cf_cloud != 0 {
        let hmem = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, std::mem::size_of::<u32>());
        if !hmem.is_null() {
            SetClipboardData(cf_cloud, hmem as _);
        }
    }

    // 3. Write plaintext to clipboard as CF_UNICODETEXT (ID 13)
    let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let byte_len = utf16.len() * std::mem::size_of::<u16>();
    let hmem_text = GlobalAlloc(GMEM_MOVEABLE, byte_len);
    if hmem_text.is_null() {
        CloseClipboard();
        return Err("Failed to allocate global memory for clipboard text".to_string());
    }

    let ptr = GlobalLock(hmem_text) as *mut u16;
    if ptr.is_null() {
        CloseClipboard();
        return Err("Failed to lock global memory for clipboard text".to_string());
    }
    std::ptr::copy_nonoverlapping(utf16.as_ptr(), ptr, utf16.len());
    GlobalUnlock(hmem_text);

    SetClipboardData(13 /* CF_UNICODETEXT */, hmem_text as _);
    CloseClipboard();
    Ok(())
}

/// Clears Windows clipboard
pub unsafe fn clear_windows_clipboard() {
    use windows_sys::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard};
    if OpenClipboard(std::ptr::null_mut()) != 0 {
        EmptyClipboard();
        CloseClipboard();
    }
}

// =========================================================================
// WINDOWS CREDENTIAL MANAGER HELPERS (WinCred API)
// =========================================================================

pub unsafe fn store_vault_key_wincred(key: &str) -> Result<(), String> {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::Security::Credentials::{
        CredWriteW, CREDENTIALW, CRED_PERSIST_ENTERPRISE, CRED_TYPE_GENERIC,
    };

    let target_name: Vec<u16> = KINANA_CRED_TARGET.encode_utf16().collect();
    let user_name: Vec<u16> = "kinana_user\0".encode_utf16().collect();
    let secret_bytes = key.as_bytes();

    let cred = CREDENTIALW {
        Flags: 0,
        Type: CRED_TYPE_GENERIC,
        TargetName: target_name.as_ptr() as *mut u16,
        Comment: std::ptr::null_mut(),
        LastWritten: std::mem::zeroed(),
        CredentialBlobSize: secret_bytes.len() as u32,
        CredentialBlob: secret_bytes.as_ptr() as *mut u8,
        Persist: CRED_PERSIST_ENTERPRISE,
        AttributeCount: 0,
        Attributes: std::ptr::null_mut(),
        TargetAlias: std::ptr::null_mut(),
        UserName: user_name.as_ptr() as *mut u16,
    };

    if CredWriteW(&cred, 0) == 0 {
        let err = GetLastError();
        return Err(format!("CredWriteW failed with code {}", err));
    }

    Ok(())
}

pub unsafe fn read_vault_key_wincred() -> Result<String, String> {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::Security::Credentials::{
        CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
    };

    let target_name: Vec<u16> = KINANA_CRED_TARGET.encode_utf16().collect();
    let mut cred_ptr: *mut CREDENTIALW = std::ptr::null_mut();

    if CredReadW(target_name.as_ptr(), CRED_TYPE_GENERIC, 0, &mut cred_ptr) == 0 {
        let err = GetLastError();
        return Err(format!("CredReadW failed with code {}", err));
    }

    if cred_ptr.is_null() {
        return Err("Credential pointer is null".to_string());
    }

    let cred = &*cred_ptr;
    let slice = std::slice::from_raw_parts(cred.CredentialBlob, cred.CredentialBlobSize as usize);
    let secret_string = String::from_utf8(slice.to_vec())
        .map_err(|e| format!("Invalid UTF-8 in credential: {}", e))?;

    CredFree(cred_ptr as *const _);
    Ok(secret_string)
}

pub unsafe fn delete_vault_key_wincred() -> Result<(), String> {
    use windows_sys::Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC};

    let target_name: Vec<u16> = KINANA_CRED_TARGET.encode_utf16().collect();
    let _ = CredDeleteW(target_name.as_ptr(), CRED_TYPE_GENERIC, 0);
    Ok(())
}

pub unsafe fn store_recovery_code_wincred(code: &str) -> Result<(), String> {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::Security::Credentials::{
        CredWriteW, CREDENTIALW, CRED_PERSIST_ENTERPRISE, CRED_TYPE_GENERIC,
    };

    let target_name: Vec<u16> = KINANA_RECOVERY_CRED_TARGET.encode_utf16().collect();
    let user_name: Vec<u16> = "kinana_user\0".encode_utf16().collect();
    let secret_bytes = code.as_bytes();

    let cred = CREDENTIALW {
        Flags: 0,
        Type: CRED_TYPE_GENERIC,
        TargetName: target_name.as_ptr() as *mut u16,
        Comment: std::ptr::null_mut(),
        LastWritten: std::mem::zeroed(),
        CredentialBlobSize: secret_bytes.len() as u32,
        CredentialBlob: secret_bytes.as_ptr() as *mut u8,
        Persist: CRED_PERSIST_ENTERPRISE,
        AttributeCount: 0,
        Attributes: std::ptr::null_mut(),
        TargetAlias: std::ptr::null_mut(),
        UserName: user_name.as_ptr() as *mut u16,
    };

    if CredWriteW(&cred, 0) == 0 {
        let err = GetLastError();
        return Err(format!("CredWriteW (recovery) failed with code {}", err));
    }

    Ok(())
}

pub unsafe fn read_recovery_code_wincred() -> Result<String, String> {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::Security::Credentials::{
        CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
    };

    let target_name: Vec<u16> = KINANA_RECOVERY_CRED_TARGET.encode_utf16().collect();
    let mut cred_ptr: *mut CREDENTIALW = std::ptr::null_mut();

    if CredReadW(target_name.as_ptr(), CRED_TYPE_GENERIC, 0, &mut cred_ptr) == 0 {
        let err = GetLastError();
        return Err(format!("CredReadW (recovery) failed with code {}", err));
    }

    if cred_ptr.is_null() {
        return Err("Recovery credential pointer is null".to_string());
    }

    let cred = &*cred_ptr;
    let slice = std::slice::from_raw_parts(cred.CredentialBlob, cred.CredentialBlobSize as usize);
    let secret_string = String::from_utf8(slice.to_vec())
        .map_err(|e| format!("Invalid UTF-8 in recovery credential: {}", e))?;

    CredFree(cred_ptr as *const _);
    Ok(secret_string)
}

// =========================================================================
// WORKING SET TRIMMING HELPER
// =========================================================================

pub fn trim_working_set_tree() {
    unsafe {
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};
        let handle = GetCurrentProcess();
        SetProcessWorkingSetSize(handle, usize::MAX, usize::MAX);
    }
}
