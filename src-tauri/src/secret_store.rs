const SERVICE_PREFIX: &str = "Cutdown";

#[cfg(windows)]
mod platform {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{ERROR_NOT_FOUND, FILETIME};
    use windows_sys::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
        CRED_TYPE_GENERIC,
    };

    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value).encode_wide().chain(Some(0)).collect()
    }

    pub fn write_secret(target: &str, value: &str) -> Result<(), String> {
        let target_name = wide(target);
        let mut blob = value.as_bytes().to_vec();
        let credential = CREDENTIALW {
            Flags: 0,
            Type: CRED_TYPE_GENERIC,
            TargetName: target_name.as_ptr() as *mut u16,
            Comment: null_mut(),
            LastWritten: FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            },
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: null_mut(),
            TargetAlias: null_mut(),
            UserName: null_mut(),
        };

        let ok = unsafe { CredWriteW(&credential, 0) };
        if ok == 0 {
            return Err(format!(
                "Failed to store secret in Windows Credential Manager: {}",
                std::io::Error::last_os_error()
            ));
        }

        Ok(())
    }

    pub fn read_secret(target: &str) -> Result<Option<String>, String> {
        let target_name = wide(target);
        let mut credential = null_mut();
        let ok = unsafe { CredReadW(target_name.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };

        if ok == 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(ERROR_NOT_FOUND as i32) {
                return Ok(None);
            }
            return Err(format!(
                "Failed to read secret from Windows Credential Manager: {err}"
            ));
        }

        let result = unsafe {
            let credential_ref = &*credential;
            let bytes = std::slice::from_raw_parts(
                credential_ref.CredentialBlob,
                credential_ref.CredentialBlobSize as usize,
            );
            let value = String::from_utf8(bytes.to_vec())
                .map_err(|err| format!("Stored secret is not valid UTF-8: {err}"));
            CredFree(credential as *const _);
            value
        }?;

        Ok(Some(result))
    }

    pub fn delete_secret(target: &str) -> Result<(), String> {
        let target_name = wide(target);
        let ok = unsafe { CredDeleteW(target_name.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if ok == 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(ERROR_NOT_FOUND as i32) {
                return Ok(());
            }
            return Err(format!(
                "Failed to delete secret from Windows Credential Manager: {err}"
            ));
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn write_secret(_target: &str, _value: &str) -> Result<(), String> {
        Err("OS secret storage is only implemented on Windows.".to_string())
    }

    pub fn read_secret(_target: &str) -> Result<Option<String>, String> {
        Ok(None)
    }

    pub fn delete_secret(_target: &str) -> Result<(), String> {
        Ok(())
    }
}

fn target_name(key: &str) -> String {
    format!("{SERVICE_PREFIX}/{key}")
}

pub fn write_secret(key: &str, value: &str) -> Result<(), String> {
    platform::write_secret(&target_name(key), value)
}

pub fn read_secret(key: &str) -> Result<Option<String>, String> {
    platform::read_secret(&target_name(key))
}

pub fn delete_secret(key: &str) -> Result<(), String> {
    platform::delete_secret(&target_name(key))
}
