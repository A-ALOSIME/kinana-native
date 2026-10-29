// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use std::time::Duration;

use kinana_native::app_state::AppState;
use kinana_native::password_gen;
use kinana_native::win32;

slint::include_modules!();

fn get_badge_color(name: &str) -> slint::Color {
    let colors = [
        slint::Color::from_argb_u8(255, 16, 185, 129), // emerald
        slint::Color::from_argb_u8(255, 14, 165, 233), // sky
        slint::Color::from_argb_u8(255, 139, 92, 246), // violet
        slint::Color::from_argb_u8(255, 245, 158, 11), // amber
        slint::Color::from_argb_u8(255, 244, 63, 94),  // rose
        slint::Color::from_argb_u8(255, 99, 102, 241), // indigo
        slint::Color::from_argb_u8(255, 20, 184, 166), // teal
        slint::Color::from_argb_u8(255, 249, 115, 22), // orange
    ];
    let hash: usize = name.bytes().fold(0usize, |acc, b| acc.wrapping_add(b as usize));
    colors[hash % colors.len()]
}

fn reload_services_into_ui(
    state: &AppState,
    window: &MainWindow,
    search_query: &str,
    category_filter: &str,
) {
    if let Ok(summary) = state.get_vault_summary() {
        let mut filtered_services = Vec::new();
        let q = search_query.trim().to_lowercase();

        for svc in &summary.services {
            // Category check from icon_data (e.g. "cat:Social")
            let category = if let Some(ref icon) = svc.icon_data {
                if icon.starts_with("cat:") {
                    icon[4..].to_string()
                } else {
                    "عام".to_string()
                }
            } else {
                "عام".to_string()
            };

            if category_filter != "الكل" && category != category_filter {
                continue;
            }

            if !q.is_empty() {
                let matches_name = svc.name.to_lowercase().contains(&q);
                let matches_acc = svc.accounts.iter().any(|a| a.username.to_lowercase().contains(&q));
                if !matches_name && !matches_acc {
                    continue;
                }
            }

            let initial = svc.name.chars().next().unwrap_or(' ').to_string();
            filtered_services.push(ServiceItem {
                id: svc.id.clone().into(),
                name: svc.name.clone().into(),
                initial: initial.into(),
                category: category.into(),
                accounts_count: svc.accounts.len() as i32,
                badge_color: get_badge_color(&svc.name),
            });
        }

        window.set_services(slint::ModelRc::new(slint::VecModel::from(filtered_services)));
        window.set_total_accounts_count(summary.total_accounts as i32);
    }
}

fn show_toast_message(window: &MainWindow, msg: &str) {
    window.set_toast_text(msg.into());
    window.set_show_toast(true);
}

fn main() -> Result<(), slint::PlatformError> {
    let main_window = MainWindow::new()?;
    let state = Arc::new(AppState::default());

    // 1. Initial State Check
    if !state.vault_exists() {
        main_window.set_current_view("setup".into());
    } else {
        main_window.set_current_view("unlock".into());
    }

    // Check saved credentials for quick unlock
    let has_saved = unsafe { win32::read_vault_key_wincred().is_ok() };
    main_window.set_has_saved_credentials(has_saved);

    // Initial password generator value
    let initial_gen = password_gen::generate_password(16, true, true, true);
    main_window.set_gen_password(initial_gen.into());

    // Load emergency recovery key if available
    if let Ok(key) = state.get_or_generate_recovery_key() {
        main_window.set_emergency_key_display(key.into());
    }

    // Auto-unlock attempt if enabled
    if state.vault_exists() && has_saved {
        if let Ok(Some(_)) = state.auto_unlock_vault() {
            main_window.set_current_view("dashboard".into());
            reload_services_into_ui(&state, &main_window, "", "الكل");
            show_toast_message(&main_window, "تم فتح الخزنة تلقائياً بنجاح");
        }
    }

    // =========================================================================
    // CALLBACK: TRY CREATE VAULT
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_try_create_vault(move |pass, confirm| {
            let pass = pass.trim();
            let confirm = confirm.trim();
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            if pass.is_empty() {
                window.set_status_message("رمز المرور لا يمكن أن يكون فارغاً".into());
                window.set_is_error_status(true);
                return;
            }

            if pass != confirm {
                window.set_status_message("رمزي المرور غير متطابقين".into());
                window.set_is_error_status(true);
                return;
            }

            match state.create_vault(pass) {
                Ok(_) => {
                    window.set_status_message("".into());
                    window.set_is_error_status(false);
                    window.set_current_view("dashboard".into());
                    reload_services_into_ui(&state, &window, "", "الكل");
                    show_toast_message(&window, "تم إنشاء الخزنة بنجاح");
                }
                Err(err) => {
                    window.set_status_message(err.into());
                    window.set_is_error_status(true);
                }
            }
        });
    }

    // =========================================================================
    // CALLBACK: TRY UNLOCK
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_try_unlock(move |pass| {
            let pass = pass.trim();
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            if pass.is_empty() {
                window.set_status_message("أدخل رمز المرور أولاً".into());
                window.set_is_error_status(true);
                return;
            }

            match state.unlock_vault(pass) {
                Ok(_) => {
                    window.set_status_message("".into());
                    window.set_is_error_status(false);
                    window.set_unlock_pass("".into());
                    window.set_current_view("dashboard".into());
                    reload_services_into_ui(&state, &window, "", "الكل");
                    show_toast_message(&window, "تم فتح الخزنة بنجاح");
                }
                Err(_) => {
                    window.set_status_message("رمز المرور غير صحيح".into());
                    window.set_is_error_status(true);
                }
            }
        });
    }

    // =========================================================================
    // CALLBACK: TRY QUICK UNLOCK
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_try_quick_unlock(move || {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            match state.auto_unlock_vault() {
                Ok(Some(_)) => {
                    window.set_status_message("".into());
                    window.set_is_error_status(false);
                    window.set_current_view("dashboard".into());
                    reload_services_into_ui(&state, &window, "", "الكل");
                    show_toast_message(&window, "تم الفتح السريع بنجاح");
                }
                _ => {
                    window.set_status_message("تعذر فتح الخزنة تلقائياً. اكتب الرمز يدوياً.".into());
                    window.set_is_error_status(true);
                }
            }
        });
    }

    // =========================================================================
    // CALLBACK: LOCK VAULT
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_lock_vault(move || {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            state.lock_vault();
            window.set_services(slint::ModelRc::new(slint::VecModel::default()));
            window.set_current_accounts(slint::ModelRc::new(slint::VecModel::default()));
            window.set_current_view("unlock".into());
            window.set_unlock_pass("".into());
            show_toast_message(&window, "تم قفل الخزنة وتفريغ الذاكرة");
        });
    }

    // =========================================================================
    // CALLBACK: COPY SECURE TEXT
    // =========================================================================
    {
        let window_weak = main_window.as_weak();
        main_window.on_copy_secure_text(move |text| {
            if text.is_empty() {
                return;
            }
            if let Ok(()) = unsafe { win32::copy_to_windows_clipboard_secure(text.as_str()) } {
                if let Some(w) = window_weak.upgrade() {
                    show_toast_message(&w, "تم النسخ بأمان إلى الحافظة");
                }
            }
        });
    }

    // =========================================================================
    // CALLBACK: COPY SERVICE PASSWORD (QUICK ACTION)
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_copy_service_password(move |service_id| {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            if let Ok(accounts) = state.get_service_accounts(service_id.as_str()) {
                if let Some(first) = accounts.first() {
                    if let Ok(pwd) = state.get_account_password(service_id.as_str(), &first.0) {
                        let _ = unsafe { win32::copy_to_windows_clipboard_secure(&pwd) };
                        show_toast_message(&window, "تم نسخ كلمة المرور بأمان");
                        return;
                    }
                }
            }
            show_toast_message(&window, "لا توجد كلمة مرور مسجلة لهذه الخدمة");
        });
    }

    // =========================================================================
    // CALLBACK: COPY ACCOUNT PASSWORD
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_copy_account_password(move |service_id, account_id| {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            if let Ok(pwd) = state.get_account_password(service_id.as_str(), account_id.as_str()) {
                let _ = unsafe { win32::copy_to_windows_clipboard_secure(&pwd) };
                show_toast_message(&window, "تم نسخ كلمة المرور بأمان");
            }
        });
    }

    // =========================================================================
    // CALLBACK: OPEN SERVICE DETAILS
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_open_service_details(move |service_id, name, category| {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            window.set_active_service_id(service_id.clone());
            window.set_active_service_name(name);
            window.set_active_service_category(category);

            if let Ok(accounts) = state.get_service_accounts(service_id.as_str()) {
                let items: Vec<AccountItem> = accounts
                    .into_iter()
                    .map(|(id, label, user, email, rec, notes, url, has_p)| AccountItem {
                        id: id.into(),
                        label: label.into(),
                        username: user.into(),
                        email: email.into(),
                        recovery_key: rec.into(),
                        notes: notes.into(),
                        url: url.into(),
                        has_password: has_p,
                    })
                    .collect();

                window.set_current_accounts(slint::ModelRc::new(slint::VecModel::from(items)));
                window.set_show_details_modal(true);
            }
        });
    }

    // =========================================================================
    // CALLBACK: ADD SERVICE
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_add_service(
            move |name, category, username, password, email, recovery, notes, url| {
                let window = match window_weak.upgrade() {
                    Some(w) => w,
                    None => return,
                };

                let name = name.trim();
                if name.is_empty() {
                    show_toast_message(&window, "يرجى كتابة اسم الخدمة");
                    return;
                }

                match state.add_service(
                    name,
                    category.trim(),
                    None,
                    username.trim(),
                    password.trim(),
                    email.trim(),
                    recovery.trim(),
                    notes.trim(),
                    url.trim(),
                ) {
                    Ok(_) => {
                        let search = window.get_search_text();
                        let cat = window.get_selected_category();
                        reload_services_into_ui(&state, &window, search.as_str(), cat.as_str());
                        show_toast_message(&window, "تمت إضافة الخدمة بنجاح");
                    }
                    Err(err) => {
                        show_toast_message(&window, &format!("خطأ: {}", err));
                    }
                }
            },
        );
    }

    // =========================================================================
    // CALLBACK: DELETE SERVICE
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_delete_service(move |service_id| {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            if let Ok(_) = state.delete_service(service_id.as_str()) {
                let search = window.get_search_text();
                let cat = window.get_selected_category();
                reload_services_into_ui(&state, &window, search.as_str(), cat.as_str());
                show_toast_message(&window, "تم حذف الخدمة");
            }
        });
    }

    // =========================================================================
    // CALLBACK: DUPLICATE SERVICE
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_duplicate_service(move |service_id| {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            if let Ok(_) = state.duplicate_service(service_id.as_str()) {
                let search = window.get_search_text();
                let cat = window.get_selected_category();
                reload_services_into_ui(&state, &window, search.as_str(), cat.as_str());
                show_toast_message(&window, "تم تكرار الخدمة بنجاح");
            }
        });
    }

    // =========================================================================
    // CALLBACK: ADD ACCOUNT
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_add_account(
            move |service_id, label, username, password, email, recovery, notes, url| {
                let window = match window_weak.upgrade() {
                    Some(w) => w,
                    None => return,
                };

                let res = state.add_account(
                    service_id.as_str(),
                    label.trim(),
                    username.trim(),
                    password.trim(),
                    email.trim(),
                    recovery.trim(),
                    notes.trim(),
                    url.trim(),
                );

                if let Ok(_) = res {
                    // Refresh accounts in modal
                    if let Ok(accounts) = state.get_service_accounts(service_id.as_str()) {
                        let items: Vec<AccountItem> = accounts
                            .into_iter()
                            .map(|(id, l, u, e, r, n, ur, has_p)| AccountItem {
                                id: id.into(),
                                label: l.into(),
                                username: u.into(),
                                email: e.into(),
                                recovery_key: r.into(),
                                notes: n.into(),
                                url: ur.into(),
                                has_password: has_p,
                            })
                            .collect();

                        window.set_current_accounts(slint::ModelRc::new(slint::VecModel::from(items)));
                    }

                    let search = window.get_search_text();
                    let cat = window.get_selected_category();
                    reload_services_into_ui(&state, &window, search.as_str(), cat.as_str());
                    show_toast_message(&window, "تمت إضافة الحساب بنجاح");
                }
            },
        );
    }

    // =========================================================================
    // CALLBACK: DELETE ACCOUNT
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_delete_account(move |service_id, account_id| {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            if let Ok(_) = state.delete_account(service_id.as_str(), account_id.as_str()) {
                if let Ok(accounts) = state.get_service_accounts(service_id.as_str()) {
                    let items: Vec<AccountItem> = accounts
                        .into_iter()
                        .map(|(id, l, u, e, r, n, ur, has_p)| AccountItem {
                            id: id.into(),
                            label: l.into(),
                            username: u.into(),
                            email: e.into(),
                            recovery_key: r.into(),
                            notes: n.into(),
                            url: ur.into(),
                            has_password: has_p,
                        })
                        .collect();

                    window.set_current_accounts(slint::ModelRc::new(slint::VecModel::from(items)));
                }

                let search = window.get_search_text();
                let cat = window.get_selected_category();
                reload_services_into_ui(&state, &window, search.as_str(), cat.as_str());
                show_toast_message(&window, "تم حذف الحساب");
            }
        });
    }

    // =========================================================================
    // CALLBACK: REQUEST GENERATE PASSWORD
    // =========================================================================
    {
        let window_weak = main_window.as_weak();
        main_window.on_request_generate_password(move |len, symbols, numbers, uppercase| {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };
            let pwd = password_gen::generate_password(len as usize, symbols, numbers, uppercase);
            window.set_gen_password(pwd.into());
        });
    }

    // =========================================================================
    // CALLBACK: EXPORT BACKUP
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_export_backup(move || {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            let dialog = rfd::FileDialog::new()
                .add_filter("Kinana Vault", &["dat"])
                .set_file_name("kinana_backup.dat");

            if let Some(dest) = dialog.save_file() {
                match state.export_vault_backup(&dest) {
                    Ok(_) => show_toast_message(&window, "تم تصدير النسخة الاحتياطية بنجاح"),
                    Err(e) => show_toast_message(&window, &format!("فشل التصدير: {}", e)),
                }
            }
        });
    }

    // =========================================================================
    // CALLBACK: IMPORT BACKUP
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_import_backup(move || {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            let dialog = rfd::FileDialog::new().add_filter("Kinana Vault", &["dat"]);

            if let Some(src) = dialog.pick_file() {
                // If the user already unlocked or if in setup, prompt or try auto unlock
                let mut password = window.get_unlock_pass().to_string();
                if password.is_empty() {
                    password = window.get_setup_pass().to_string();
                }

                if password.is_empty() {
                    show_toast_message(&window, "اكتب كلمة المرور أولاً في الحقل لاستيراد النسخة");
                    return;
                }

                match state.import_vault_backup(&src, &password) {
                    Ok(_) => {
                        window.set_current_view("dashboard".into());
                        reload_services_into_ui(&state, &window, "", "الكل");
                        show_toast_message(&window, "تم استيراد الخزنة بنجاح");
                    }
                    Err(e) => show_toast_message(&window, &format!("فشل الاستيراد: {}", e)),
                }
            }
        });
    }

    // =========================================================================
    // CALLBACK: CHANGE MASTER PASSWORD
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_change_master_password(move |old_pass, new_pass, confirm_pass| {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            let new_p = new_pass.trim();
            if new_p.is_empty() {
                show_toast_message(&window, "رمز المرور الجديد لا يمكن أن يكون فارغاً");
                return;
            }

            if new_p != confirm_pass.trim() {
                show_toast_message(&window, "الرمز الجديد غير متطابق مع التأكيد");
                return;
            }

            match state.change_master_password(old_pass.trim(), new_p) {
                Ok(_) => {
                    window.set_change_old_pass("".into());
                    window.set_change_new_pass("".into());
                    window.set_change_confirm_pass("".into());
                    show_toast_message(&window, "تم تحديث رمز المرور بنجاح");
                }
                Err(e) => show_toast_message(&window, &format!("فشل التغيير: {}", e)),
            }
        });
    }

    // =========================================================================
    // CALLBACK: TRY RECOVER VAULT
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_try_recover_vault(move |recovery_key, new_pass| {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            let key = recovery_key.trim();
            let new_p = new_pass.trim();

            if key.is_empty() || new_p.is_empty() {
                show_toast_message(&window, "أدخل مفتاح الاسترداد والرمز الجديد");
                return;
            }

            match state.recover_vault_with_key(key, new_p) {
                Ok(_) => {
                    window.set_show_recovery_modal(false);
                    window.set_current_view("dashboard".into());
                    reload_services_into_ui(&state, &window, "", "الكل");
                    show_toast_message(&window, "تم استرداد الخزنة بنجاح");
                }
                Err(e) => show_toast_message(&window, &format!("فشل الاسترداد: {}", e)),
            }
        });
    }

    // =========================================================================
    // CALLBACK: SEARCH & FILTER
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_search_services(move |q| {
            if let Some(w) = window_weak.upgrade() {
                let cat = w.get_selected_category();
                reload_services_into_ui(&state, &w, q.as_str(), cat.as_str());
            }
        });
    }

    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_filter_by_category(move |cat| {
            if let Some(w) = window_weak.upgrade() {
                let q = w.get_search_text();
                reload_services_into_ui(&state, &w, q.as_str(), cat.as_str());
            }
        });
    }

    // =========================================================================
    // CALLBACK: OPEN EXTERNAL URL
    // =========================================================================
    main_window.on_open_external_url(move |url| {
        let u = url.trim();
        if !u.is_empty() {
            let _ = std::process::Command::new("cmd")
                .args(&["/C", "start", "", u])
                .spawn();
        }
    });

    // =========================================================================
    // TIMERS: AUTO-LOCK & TOAST DISMISSAL & MEMORY TRIMMING
    // =========================================================================
    let auto_lock_timer = slint::Timer::default();
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        auto_lock_timer.start(
            slint::TimerMode::Repeated,
            Duration::from_secs(1),
            move || {
                let window = match window_weak.upgrade() {
                    Some(w) => w,
                    None => return,
                };

                // Check auto-lock timeout
                if state.is_unlocked() && state.check_idle_timeout() {
                    state.lock_vault();
                    window.set_services(slint::ModelRc::new(slint::VecModel::default()));
                    window.set_current_accounts(slint::ModelRc::new(slint::VecModel::default()));
                    window.set_current_view("unlock".into());
                    window.set_unlock_pass("".into());
                    show_toast_message(&window, "تم قفل الخزنة تلقائياً لحماية بياناتك");
                }
            },
        );
    }

    let memory_trim_timer = slint::Timer::default();
    memory_trim_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_secs(5),
        move || {
            win32::trim_working_set_tree();
        },
    );

    let toast_timer = slint::Timer::default();
    {
        let window_weak = main_window.as_weak();
        toast_timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(3000),
            move || {
                if let Some(w) = window_weak.upgrade() {
                    if w.get_show_toast() {
                        w.set_show_toast(false);
                    }
                }
            },
        );
    }

    // Run the native application event loop
    main_window.run()
}
