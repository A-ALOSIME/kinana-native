use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use kinana_core::{
    crypto::{decrypt_payload, derive_key, DerivedKey, VaultHeader, MIN_VAULT_FILE_SIZE},
    models::{Account, SecretString, Service, VaultSummary},
    storage::{VaultSession, VaultStorage},
};
use zeroize::Zeroize;

pub struct AppState {
    pub custom_dir: Option<PathBuf>,
    pub session: Mutex<Option<VaultSession>>,
    pub last_activity: Mutex<Instant>,
    pub auto_lock_timeout_secs: Mutex<Option<u64>>,
    pub current_master_password: Mutex<Option<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            custom_dir: None,
            session: Mutex::new(None),
            last_activity: Mutex::new(Instant::now()),
            auto_lock_timeout_secs: Mutex::new(Some(300)), // Default 5 minutes
            current_master_password: Mutex::new(None),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_custom_dir(path: PathBuf) -> Self {
        Self {
            custom_dir: Some(path),
            session: Mutex::new(None),
            last_activity: Mutex::new(Instant::now()),
            auto_lock_timeout_secs: Mutex::new(Some(300)),
            current_master_password: Mutex::new(None),
        }
    }

    pub fn get_app_dir(&self) -> Result<PathBuf, String> {
        if let Some(dir) = &self.custom_dir {
            if !dir.exists() {
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            return Ok(dir.clone());
        }

        let base = dirs::config_dir().ok_or_else(|| "Failed to find config directory".to_string())?;
        let path = base.join("com.kinana.vault");
        if !path.exists() {
            std::fs::create_dir_all(&path).map_err(|e| format!("Failed to create vault dir: {}", e))?;
        }
        Ok(path)
    }

    pub fn get_vault_path(&self) -> Result<PathBuf, String> {
        let dir = self.get_app_dir()?;
        Ok(dir.join("vault.dat"))
    }

    pub fn get_recovery_file_path(&self) -> Result<PathBuf, String> {
        let dir = self.get_app_dir()?;
        Ok(dir.join("recovery.dat"))
    }
}

pub fn unpack_account_notes(raw_notes: Option<&str>) -> (String, String, String) {
    let raw = match raw_notes {
        Some(s) if !s.is_empty() => s,
        _ => return (String::new(), String::new(), String::new()),
    };

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) {
        if val.is_object() {
            let email = val.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let rec = val.get("recovery_key").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let notes = val.get("notes").and_then(|v| v.as_str()).unwrap_or("").to_string();
            return (email, rec, notes);
        }
    }
    (String::new(), String::new(), raw.to_string())
}

pub fn pack_account_notes(email: &str, recovery_key: &str, notes: &str) -> Option<String> {
    if email.is_empty() && recovery_key.is_empty() && notes.is_empty() {
        return None;
    }
    let val = serde_json::json!({
        "email": email,
        "recovery_key": recovery_key,
        "notes": notes,
    });
    Some(val.to_string())
}

impl AppState {
    pub fn update_activity(&self) {
        if let Ok(mut last) = self.last_activity.lock() {
            *last = Instant::now();
        }
    }

    pub fn check_idle_timeout(&self) -> bool {
        let timeout_secs = match self.auto_lock_timeout_secs.lock() {
            Ok(guard) => match *guard {
                Some(s) => s,
                None => return false,
            },
            Err(_) => return false,
        };

        if let Ok(last) = self.last_activity.lock() {
            if last.elapsed().as_secs() >= timeout_secs {
                return true;
            }
        }
        false
    }

    pub fn vault_exists(&self) -> bool {
        self.get_vault_path().map(|p| p.exists()).unwrap_or(false)
    }

    pub fn is_unlocked(&self) -> bool {
        if let Ok(guard) = self.session.lock() {
            guard.is_some()
        } else {
            false
        }
    }

    pub fn create_vault(&self, password: &str) -> Result<VaultSummary, String> {
        let path = self.get_vault_path()?;
        let storage = VaultStorage::new(&path);

        if storage.exists() {
            return Err("Vault already exists. Use unlock instead.".to_string());
        }

        let mut session = storage
            .create_new_vault(password, None)
            .map_err(|e| format!("Failed to create vault: {}", e))?;

        storage
            .save(&mut session)
            .map_err(|e| format!("Failed to save new vault: {}", e))?;

        let summary = session.data.to_summary();

        // Store vault key to Windows Credentials by default
        let _ = unsafe { crate::win32::store_vault_key_wincred(password) };
        let _ = self.get_or_generate_recovery_key();

        if let Ok(mut guard) = self.session.lock() {
            *guard = Some(session);
        }
        if let Ok(mut pass_guard) = self.current_master_password.lock() {
            *pass_guard = Some(password.to_string());
        }

        self.update_activity();
        Ok(summary)
    }

    pub fn unlock_vault(&self, password: &str) -> Result<VaultSummary, String> {
        let path = self.get_vault_path()?;
        let storage = VaultStorage::new(&path);

        if !storage.exists() {
            return Err("Vault file does not exist".to_string());
        }

        let session = storage
            .unlock(password)
            .map_err(|e| format!("Password incorrect or corrupted vault: {}", e))?;

        let summary = session.data.to_summary();

        if let Ok(mut guard) = self.session.lock() {
            *guard = Some(session);
        }
        if let Ok(mut pass_guard) = self.current_master_password.lock() {
            *pass_guard = Some(password.to_string());
        }

        self.update_activity();
        Ok(summary)
    }

    pub fn auto_unlock_vault(&self) -> Result<Option<VaultSummary>, String> {
        if !self.vault_exists() {
            return Ok(None);
        }

        let saved_key = unsafe { crate::win32::read_vault_key_wincred() };
        match saved_key {
            Ok(key) if !key.is_empty() => {
                let summary = self.unlock_vault(&key)?;
                Ok(Some(summary))
            }
            _ => Ok(None),
        }
    }

    pub fn lock_vault(&self) {
        if let Ok(mut guard) = self.session.lock() {
            *guard = None;
        }
        if let Ok(mut pass_guard) = self.current_master_password.lock() {
            if let Some(mut p) = pass_guard.take() {
                p.zeroize();
            }
        }
        crate::win32::trim_working_set_tree();
    }

    pub fn get_vault_summary(&self) -> Result<VaultSummary, String> {
        let guard = self.session.lock().map_err(|_| "Mutex poisoned".to_string())?;
        match guard.as_ref() {
            Some(session) => Ok(session.data.to_summary()),
            None => Err("Vault is locked".to_string()),
        }
    }

    pub fn add_service(
        &self,
        name: &str,
        category: &str,
        icon_data: Option<String>,
        username: &str,
        password: &str,
        email: &str,
        recovery_key: &str,
        notes: &str,
        url: &str,
    ) -> Result<VaultSummary, String> {
        self.update_activity();
        let path = self.get_vault_path()?;
        let storage = VaultStorage::new(&path);
        let mut guard = self.session.lock().map_err(|_| "Mutex poisoned".to_string())?;
        let session = guard.as_mut().ok_or_else(|| "Vault is locked".to_string())?;

        let service_id = uuid::Uuid::new_v4().to_string();
        let mut accounts = Vec::new();

        if !username.is_empty() || !password.is_empty() || !email.is_empty() {
            let account_id = uuid::Uuid::new_v4().to_string();
            let packed = pack_account_notes(email, recovery_key, notes);
            accounts.push(Account {
                id: account_id,
                label: "الحساب الأساسي".to_string(),
                username: username.to_string(),
                password: SecretString::new(password.to_string()),
                notes: packed,
                url: if url.is_empty() { None } else { Some(url.to_string()) },
            });
        }

        let final_icon_data = if icon_data.is_some() {
            icon_data
        } else if !category.is_empty() {
            Some(format!("cat:{}", category))
        } else {
            None
        };

        session.data.services.push(Service {
            id: service_id,
            name: name.to_string(),
            icon_data: final_icon_data,
            accounts,
        });

        storage.save(session).map_err(|e| format!("Failed to save vault: {}", e))?;
        Ok(session.data.to_summary())
    }

    pub fn update_service(
        &self,
        service_id: &str,
        name: &str,
        category: &str,
    ) -> Result<VaultSummary, String> {
        self.update_activity();
        let path = self.get_vault_path()?;
        let storage = VaultStorage::new(&path);
        let mut guard = self.session.lock().map_err(|_| "Mutex poisoned".to_string())?;
        let session = guard.as_mut().ok_or_else(|| "Vault is locked".to_string())?;

        let svc = session
            .data
            .services
            .iter_mut()
            .find(|s| s.id == service_id)
            .ok_or_else(|| "Service not found".to_string())?;

        svc.name = name.to_string();
        if !category.is_empty() {
            svc.icon_data = Some(format!("cat:{}", category));
        }

        storage.save(session).map_err(|e| format!("Failed to save vault: {}", e))?;
        Ok(session.data.to_summary())
    }

    pub fn delete_service(&self, service_id: &str) -> Result<VaultSummary, String> {
        self.update_activity();
        let path = self.get_vault_path()?;
        let storage = VaultStorage::new(&path);
        let mut guard = self.session.lock().map_err(|_| "Mutex poisoned".to_string())?;
        let session = guard.as_mut().ok_or_else(|| "Vault is locked".to_string())?;

        let initial_len = session.data.services.len();
        session.data.services.retain(|s| s.id != service_id);

        if session.data.services.len() == initial_len {
            return Err("Service not found".to_string());
        }

        storage.save(session).map_err(|e| format!("Failed to save vault: {}", e))?;
        Ok(session.data.to_summary())
    }

    pub fn duplicate_service(&self, service_id: &str) -> Result<VaultSummary, String> {
        self.update_activity();
        let path = self.get_vault_path()?;
        let storage = VaultStorage::new(&path);
        let mut guard = self.session.lock().map_err(|_| "Mutex poisoned".to_string())?;
        let session = guard.as_mut().ok_or_else(|| "Vault is locked".to_string())?;

        let original = session
            .data
            .services
            .iter()
            .find(|s| s.id == service_id)
            .ok_or_else(|| "Service not found".to_string())?;

        let duplicated = Service {
            id: uuid::Uuid::new_v4().to_string(),
            name: format!("{} (نسخة)", original.name),
            icon_data: original.icon_data.clone(),
            accounts: original
                .accounts
                .iter()
                .map(|a| Account {
                    id: uuid::Uuid::new_v4().to_string(),
                    label: a.label.clone(),
                    username: a.username.clone(),
                    password: SecretString::new(a.password.as_str().to_string()),
                    notes: a.notes.clone(),
                    url: a.url.clone(),
                })
                .collect(),
        };

        session.data.services.push(duplicated);
        storage.save(session).map_err(|e| format!("Failed to save vault: {}", e))?;
        Ok(session.data.to_summary())
    }

    pub fn add_account(
        &self,
        service_id: &str,
        label: &str,
        username: &str,
        password: &str,
        email: &str,
        recovery_key: &str,
        notes: &str,
        url: &str,
    ) -> Result<VaultSummary, String> {
        self.update_activity();
        let path = self.get_vault_path()?;
        let storage = VaultStorage::new(&path);
        let mut guard = self.session.lock().map_err(|_| "Mutex poisoned".to_string())?;
        let session = guard.as_mut().ok_or_else(|| "Vault is locked".to_string())?;

        let svc = session
            .data
            .services
            .iter_mut()
            .find(|s| s.id == service_id)
            .ok_or_else(|| "Service not found".to_string())?;

        let packed = pack_account_notes(email, recovery_key, notes);
        svc.accounts.push(Account {
            id: uuid::Uuid::new_v4().to_string(),
            label: if label.is_empty() { "حساب".to_string() } else { label.to_string() },
            username: username.to_string(),
            password: SecretString::new(password.to_string()),
            notes: packed,
            url: if url.is_empty() { None } else { Some(url.to_string()) },
        });

        storage.save(session).map_err(|e| format!("Failed to save vault: {}", e))?;
        Ok(session.data.to_summary())
    }

    pub fn delete_account(&self, service_id: &str, account_id: &str) -> Result<VaultSummary, String> {
        self.update_activity();
        let path = self.get_vault_path()?;
        let storage = VaultStorage::new(&path);
        let mut guard = self.session.lock().map_err(|_| "Mutex poisoned".to_string())?;
        let session = guard.as_mut().ok_or_else(|| "Vault is locked".to_string())?;

        let svc = session
            .data
            .services
            .iter_mut()
            .find(|s| s.id == service_id)
            .ok_or_else(|| "Service not found".to_string())?;

        svc.accounts.retain(|a| a.id != account_id);

        storage.save(session).map_err(|e| format!("Failed to save vault: {}", e))?;
        Ok(session.data.to_summary())
    }

    pub fn get_account_password(&self, service_id: &str, account_id: &str) -> Result<String, String> {
        self.update_activity();
        let guard = self.session.lock().map_err(|_| "Mutex poisoned".to_string())?;
        let session = guard.as_ref().ok_or_else(|| "Vault is locked".to_string())?;

        session
            .data
            .get_account_password(service_id, account_id)
            .map(|p| p.to_string())
            .ok_or_else(|| "Account or password not found".to_string())
    }

    pub fn get_service_accounts(
        &self,
        service_id: &str,
    ) -> Result<Vec<(String, String, String, String, String, String, String, bool)>, String> {
        self.update_activity();
        let guard = self.session.lock().map_err(|_| "Mutex poisoned".to_string())?;
        let session = guard.as_ref().ok_or_else(|| "Vault is locked".to_string())?;

        let svc = session
            .data
            .services
            .iter()
            .find(|s| s.id == service_id)
            .ok_or_else(|| "Service not found".to_string())?;

        let res = svc
            .accounts
            .iter()
            .map(|a| {
                let (email, rec, notes) = unpack_account_notes(a.notes.as_deref());
                (
                    a.id.clone(),
                    a.label.clone(),
                    a.username.clone(),
                    email,
                    rec,
                    notes,
                    a.url.clone().unwrap_or_default(),
                    !a.password.as_str().is_empty(),
                )
            })
            .collect();

        Ok(res)
    }

    pub fn change_master_password(&self, old_pass: &str, new_pass: &str) -> Result<(), String> {
        self.update_activity();
        let path = self.get_vault_path()?;
        let storage = VaultStorage::new(&path);
        let mut guard = self.session.lock().map_err(|_| "Mutex poisoned".to_string())?;
        let session = guard.as_mut().ok_or_else(|| "Vault is locked".to_string())?;

        let old_key = DerivedKey::new(*session.key.as_bytes());
        let backup_path = path.with_extension("dat.bak");

        if path.exists() {
            let existing_bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            if existing_bytes.len() < MIN_VAULT_FILE_SIZE {
                return Err("Existing vault file is corrupt".to_string());
            }

            decrypt_payload(&existing_bytes, &old_key)
                .map_err(|_| "Failed to authenticate with current password".to_string())?;

            std::fs::copy(&path, &backup_path).map_err(|e| e.to_string())?;
            let _ = std::fs::remove_file(&path);
        }

        let new_header = VaultHeader::new_with_fresh_salts(session.header.argon2_params)
            .map_err(|e| {
                if backup_path.exists() && !path.exists() {
                    let _ = std::fs::copy(&backup_path, &path);
                }
                e.to_string()
            })?;

        let new_key = derive_key(new_pass, &new_header.salt, &new_header.argon2_params)
            .map_err(|e| {
                if backup_path.exists() && !path.exists() {
                    let _ = std::fs::copy(&backup_path, &path);
                }
                e.to_string()
            })?;

        session.header = new_header;
        session.key = new_key;
        session.data.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if let Err(e) = storage.save(session) {
            if backup_path.exists() && !path.exists() {
                let _ = std::fs::copy(&backup_path, &path);
            }
            session.key = old_key;
            return Err(e.to_string());
        }

        // If Windows Credential Manager has auto-unlock enabled, update the key
        let saved_key = unsafe { crate::win32::read_vault_key_wincred() };
        if let Ok(k) = saved_key {
            if k == old_pass {
                let _ = unsafe { crate::win32::store_vault_key_wincred(new_pass) };
            }
        }

        if let Ok(mut pass_guard) = self.current_master_password.lock() {
            *pass_guard = Some(new_pass.to_string());
        }

        Ok(())
    }

    pub fn export_vault_backup(&self, target_path: &Path) -> Result<(), String> {
        self.update_activity();
        let vault_path = self.get_vault_path()?;
        if !vault_path.exists() {
            return Err("Vault file does not exist".to_string());
        }

        std::fs::copy(&vault_path, target_path)
            .map_err(|e| format!("Failed to copy backup file: {}", e))?;
        Ok(())
    }

    pub fn import_vault_backup(&self, source_path: &Path, password: &str) -> Result<VaultSummary, String> {
        self.update_activity();
        let source_storage = VaultStorage::new(source_path);
        let test_session = source_storage
            .unlock(password)
            .map_err(|e| format!("Failed to decrypt backup: {}", e))?;

        let destination = self.get_vault_path()?;
        std::fs::copy(source_path, &destination)
            .map_err(|e| format!("Failed to copy imported vault: {}", e))?;

        let summary = test_session.data.to_summary();
        if let Ok(mut guard) = self.session.lock() {
            *guard = Some(test_session);
        }
        if let Ok(mut pass_guard) = self.current_master_password.lock() {
            *pass_guard = Some(password.to_string());
        }

        Ok(summary)
    }

    pub fn get_or_generate_recovery_key(&self) -> Result<String, String> {
        self.update_activity();
        let recovery_path = self.get_recovery_file_path()?;

        if let Ok(code) = unsafe { crate::win32::read_recovery_code_wincred() } {
            if !code.is_empty() {
                return Ok(code);
            }
        }

        if recovery_path.exists() {
            if let Ok(code) = std::fs::read_to_string(&recovery_path) {
                let trimmed = code.trim().to_string();
                if !trimmed.is_empty() {
                    let _ = unsafe { crate::win32::store_recovery_code_wincred(&trimmed) };
                    return Ok(trimmed);
                }
            }
        }

        let new_code = format!(
            "KINANA-{}-{}-{}",
            &uuid::Uuid::new_v4().to_string()[..8].to_uppercase(),
            &uuid::Uuid::new_v4().to_string()[..8].to_uppercase(),
            &uuid::Uuid::new_v4().to_string()[..8].to_uppercase()
        );

        let _ = std::fs::write(&recovery_path, &new_code);
        let _ = unsafe { crate::win32::store_recovery_code_wincred(&new_code) };

        Ok(new_code)
    }

    pub fn recover_vault_with_key(&self, recovery_key: &str, new_password: &str) -> Result<VaultSummary, String> {
        let expected_key = self.get_or_generate_recovery_key()?;
        if recovery_key.trim() != expected_key.trim() {
            return Err("Invalid recovery key".to_string());
        }

        let saved_pass = unsafe { crate::win32::read_vault_key_wincred() };
        if let Ok(pass) = saved_pass {
            self.unlock_vault(&pass)?;
            self.change_master_password(&pass, new_password)?;
            return self.get_vault_summary();
        }

        Err("Vault recovery requires auto-unlock credential cache on this machine".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_unpack_account_notes() {
        let packed = pack_account_notes("user@test.com", "REC-1234", "My secret note");
        assert!(packed.is_some());

        let (email, rec, notes) = unpack_account_notes(packed.as_deref());
        assert_eq!(email, "user@test.com");
        assert_eq!(rec, "REC-1234");
        assert_eq!(notes, "My secret note");
    }

    #[test]
    fn test_pack_empty_notes_returns_none() {
        let packed = pack_account_notes("", "", "");
        assert!(packed.is_none());
    }

    #[test]
    fn test_unpack_none_returns_empty() {
        let (email, rec, notes) = unpack_account_notes(None);
        assert_eq!(email, "");
        assert_eq!(rec, "");
        assert_eq!(notes, "");
    }
}
