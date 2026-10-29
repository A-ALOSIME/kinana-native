use kinana_native::app_state::AppState;
use tempfile::tempdir;

#[test]
fn test_vault_create_lock_unlock_lifecycle() {
    let tmp = tempdir().unwrap();
    let state = AppState::with_custom_dir(tmp.path().to_path_buf());
    assert!(!state.vault_exists());

    // 1. Create vault
    let summary = state.create_vault("TestMasterPass123!").expect("Failed to create vault");
    assert_eq!(summary.total_services, 0);
    assert_eq!(summary.total_accounts, 0);
    assert!(state.vault_exists());
    assert!(state.is_unlocked());

    // 2. Add services
    let s1 = state.add_service(
        "Google",
        "تواصل",
        None,
        "ahmad@gmail.com",
        "SuperSecretPass1!",
        "ahmad@gmail.com",
        "REC-GOOGLE-123",
        "Personal email account",
        "https://google.com",
    ).expect("Failed to add Google service");

    assert_eq!(s1.total_services, 1);
    assert_eq!(s1.total_accounts, 1);

    let s2 = state.add_service(
        "Twitter",
        "تواصل",
        None,
        "ahmad_dev",
        "TwitterPassword2026$",
        "",
        "",
        "",
        "https://x.com",
    ).expect("Failed to add Twitter service");

    assert_eq!(s2.total_services, 2);
    assert_eq!(s2.total_accounts, 2);

    // 3. Verify password extraction
    let google_svc_id = &s2.services.iter().find(|s| s.name == "Google").unwrap().id;
    let accounts = state.get_service_accounts(google_svc_id).unwrap();
    assert_eq!(accounts.len(), 1);
    let google_acc_id = &accounts[0].0;
    let pwd = state.get_account_password(google_svc_id, google_acc_id).unwrap();
    assert_eq!(pwd, "SuperSecretPass1!");

    // 4. Duplicate service
    let s3 = state.duplicate_service(google_svc_id).expect("Failed to duplicate service");
    assert_eq!(s3.total_services, 3);
    assert!(s3.services.iter().any(|s| s.name == "Google (نسخة)"));

    // 5. Lock vault
    state.lock_vault();
    assert!(!state.is_unlocked());
    assert!(state.get_vault_summary().is_err());

    // 6. Unlock with wrong password fails
    assert!(state.unlock_vault("WrongPassword!").is_err());
    assert!(!state.is_unlocked());

    // 7. Unlock with correct password succeeds
    let unlocked = state.unlock_vault("TestMasterPass123!").expect("Failed to unlock");
    assert_eq!(unlocked.total_services, 3);
    assert!(state.is_unlocked());

    // 8. Change master password
    state.change_master_password("TestMasterPass123!", "NewSecretKey2026#").expect("Failed to change password");

    // 9. Lock and verify old password fails and new password works
    state.lock_vault();
    assert!(state.unlock_vault("TestMasterPass123!").is_err());
    let re_unlocked = state.unlock_vault("NewSecretKey2026#").expect("Failed to unlock with new password");
    assert_eq!(re_unlocked.total_services, 3);
}

#[test]
fn test_vault_export_and_import() {
    let tmp_src = tempdir().unwrap();
    let state = AppState::with_custom_dir(tmp_src.path().to_path_buf());
    state.create_vault("Pass123").unwrap();
    state.add_service("GitHub", "عمل", None, "octocat", "GitPass99!", "", "", "", "").unwrap();

    let export_file = tmp_src.path().join("backup.dat");
    state.export_vault_backup(&export_file).expect("Failed to export backup");
    assert!(export_file.exists());

    // Switch to a new empty directory
    let tmp_dst = tempdir().unwrap();
    let state_dst = AppState::with_custom_dir(tmp_dst.path().to_path_buf());
    assert!(!state_dst.vault_exists());

    // Import the backup
    let imported = state_dst.import_vault_backup(&export_file, "Pass123").expect("Failed to import");
    assert_eq!(imported.total_services, 1);
    assert_eq!(imported.services[0].name, "GitHub");
    assert!(state_dst.is_unlocked());
}
