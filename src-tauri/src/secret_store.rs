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

/// Linux: the freedesktop Secret Service (KWallet, GNOME Keyring, KeePassXC…). When no
/// keyring daemon is running, secrets fall back to a 0600 file next to settings.json so
/// uploads still work on minimal desktops.
#[cfg(not(windows))]
mod platform {
    use std::collections::BTreeMap;
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    use std::path::PathBuf;

    const SERVICE: &str = "Cutdown";

    fn entry(target: &str) -> Result<keyring::Entry, keyring::Error> {
        keyring::Entry::new(SERVICE, target)
    }

    /// The keyring itself is unreachable (no daemon, locked and dismissed, no D-Bus session).
    fn keyring_unavailable(err: &keyring::Error) -> bool {
        matches!(
            err,
            keyring::Error::PlatformFailure(_) | keyring::Error::NoStorageAccess(_)
        )
    }

    pub fn write_secret(target: &str, value: &str) -> Result<(), String> {
        match entry(target).and_then(|entry| entry.set_password(value)) {
            Ok(()) => {
                // Don't leave an older copy in the fallback file.
                let _ = remove_file_secret(target);
                Ok(())
            }
            Err(err) if keyring_unavailable(&err) => {
                eprintln!("system keyring unavailable, storing secret in file: {err}");
                write_file_secret(target, value)
            }
            Err(err) => Err(format!("Failed to store secret in the system keyring: {err}")),
        }
    }

    pub fn read_secret(target: &str) -> Result<Option<String>, String> {
        match entry(target).and_then(|entry| entry.get_password()) {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(read_file_secrets().remove(target)),
            Err(err) if keyring_unavailable(&err) => Ok(read_file_secrets().remove(target)),
            Err(err) => Err(format!("Failed to read secret from the system keyring: {err}")),
        }
    }

    pub fn delete_secret(target: &str) -> Result<(), String> {
        let keyring_result = entry(target).and_then(|entry| entry.delete_credential());
        remove_file_secret(target)?;
        match keyring_result {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) if keyring_unavailable(&err) => Ok(()),
            Err(err) => Err(format!("Failed to delete secret from the system keyring: {err}")),
        }
    }

    fn secrets_file() -> Result<PathBuf, String> {
        Ok(crate::app_paths::config_dir()?.join("secrets.json"))
    }

    fn read_file_secrets() -> BTreeMap<String, String> {
        secrets_file()
            .ok()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    fn write_file_secrets(secrets: &BTreeMap<String, String>) -> Result<(), String> {
        let path = secrets_file()?;
        if secrets.is_empty() {
            return match std::fs::remove_file(&path) {
                Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
                    Err(format!("Failed to remove {}: {err}", path.display()))
                }
                _ => Ok(()),
            };
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("Failed to create settings directory: {err}"))?;
        }
        let raw = serde_json::to_string_pretty(secrets)
            .map_err(|err| format!("Failed to serialize secrets: {err}"))?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)
            .map_err(|err| format!("Failed to open {}: {err}", path.display()))?;
        file.write_all(raw.as_bytes())
            .map_err(|err| format!("Failed to write {}: {err}", path.display()))
    }

    fn write_file_secret(target: &str, value: &str) -> Result<(), String> {
        let mut secrets = read_file_secrets();
        secrets.insert(target.to_string(), value.to_string());
        write_file_secrets(&secrets)
    }

    fn remove_file_secret(target: &str) -> Result<(), String> {
        let mut secrets = read_file_secrets();
        if secrets.remove(target).is_some() {
            write_file_secrets(&secrets)?;
        }
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
