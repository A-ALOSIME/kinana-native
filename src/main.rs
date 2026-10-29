// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use slint::Model;

use kinana_native::app_state::AppState;
use kinana_native::password_gen;
use kinana_native::win32;

slint::include_modules!();

fn get_brand_color(name: &str) -> slint::Color {
    let lower = name.trim().to_lowercase();
    if lower.contains("absher") || lower.contains("أبشر") {
        slint::Color::from_argb_u8(255, 0, 104, 55) // #006837
    } else if lower.contains("nafath") || lower.contains("نفاذ") {
        slint::Color::from_argb_u8(255, 0, 122, 94) // #007a5e
    } else if lower.contains("tawakkalna") || lower.contains("توكلنا") {
        slint::Color::from_argb_u8(255, 16, 124, 65) // #107c41
    } else if lower.contains("rajhi") || lower.contains("الراجحي") {
        slint::Color::from_argb_u8(255, 26, 59, 139)
    } else if lower.contains("alinma") || lower.contains("الانماء") || lower.contains("الإنماء") {
        slint::Color::from_argb_u8(255, 150, 24, 40)
    } else if lower.contains("stc") {
        slint::Color::from_argb_u8(255, 79, 19, 137)
    } else if lower.contains("noon") || lower.contains("نون") {
        slint::Color::from_argb_u8(255, 251, 238, 19)
    } else if lower.contains("amazon") || lower.contains("امازون") || lower.contains("أمازون") {
        slint::Color::from_argb_u8(255, 255, 153, 0)
    } else {
        let colors = [
            slint::Color::from_argb_u8(255, 16, 185, 129),
            slint::Color::from_argb_u8(255, 14, 165, 233),
            slint::Color::from_argb_u8(255, 139, 92, 246),
            slint::Color::from_argb_u8(255, 245, 158, 11),
            slint::Color::from_argb_u8(255, 244, 63, 94),
            slint::Color::from_argb_u8(255, 99, 102, 241),
            slint::Color::from_argb_u8(255, 20, 184, 166),
            slint::Color::from_argb_u8(255, 249, 115, 22),
        ];
        let hash: usize = name.bytes().fold(0usize, |acc, b| acc.wrapping_add(b as usize));
        colors[hash % colors.len()]
    }
}

fn reload_services_into_ui(state: &AppState, window: &MainWindow) {
    if let Ok(summary) = state.get_vault_summary() {
        let mut service_items = Vec::new();

        for svc in &summary.services {
            let initial = svc.name.chars().next().unwrap_or(' ').to_string();
            let (has_cover, cover_image) = get_service_cover(&svc.name, svc.icon_data.as_deref());
            let badge_color = get_brand_color(&svc.name);

            service_items.push(ServiceItem {
                id: svc.id.clone().into(),
                name: svc.name.clone().into(),
                initial: initial.into(),
                category: "عام".into(),
                accounts_count: svc.accounts.len() as i32,
                badge_color,
                has_cover,
                cover_image,
            });
        }

        window.set_services(slint::ModelRc::new(slint::VecModel::from(service_items)));
        window.set_total_accounts_count(summary.total_accounts as i32);
    }
}

fn show_toast_message(window: &MainWindow, msg: &str) {
    window.set_toast_text(msg.into());
    window.set_show_toast(true);
}

fn get_preset_svg_bytes(name: &str) -> Option<&'static [u8]> {
    let lower = name.trim().to_lowercase();
    if lower.contains("absher") || lower.contains("أبشر") {
        Some(include_bytes!("../ui/assets/presets/absher.svg"))
    } else if lower.contains("nafath") || lower.contains("نفاذ") {
        Some(include_bytes!("../ui/assets/presets/nafath.svg"))
    } else if lower.contains("tawakkalna") || lower.contains("توكلنا") {
        Some(include_bytes!("../ui/assets/presets/tawakkalna.svg"))
    } else if lower.contains("rajhi") || lower.contains("الراجحي") {
        Some(include_bytes!("../ui/assets/presets/alrajhibank.svg"))
    } else if lower.contains("alinma") || lower.contains("الانماء") || lower.contains("الإنماء") {
        Some(include_bytes!("../ui/assets/presets/alinmabank.svg"))
    } else if lower.contains("stc pay") || lower.contains("stcpay") {
        Some(include_bytes!("../ui/assets/presets/stcpay.svg"))
    } else if lower.contains("stc") {
        Some(include_bytes!("../ui/assets/presets/stc.svg"))
    } else if lower.contains("noon") || lower.contains("نون") {
        Some(include_bytes!("../ui/assets/presets/noon.svg"))
    } else if lower.contains("jarir") || lower.contains("جرير") {
        Some(include_bytes!("../ui/assets/presets/jarir.svg"))
    } else if lower.contains("amazon") || lower.contains("أمازون") || lower.contains("امازون") {
        Some(include_bytes!("../ui/assets/presets/amazon.svg"))
    } else if lower.contains("sehhaty") || lower.contains("صحتي") {
        Some(include_bytes!("../ui/assets/presets/sehhaty.svg"))
    } else if lower.contains("qiwa") || lower.contains("قوى") {
        Some(include_bytes!("../ui/assets/presets/qiwa.svg"))
    } else if lower.contains("citizen") || lower.contains("المواطن") {
        Some(include_bytes!("../ui/assets/presets/citizenaccount.svg"))
    } else if lower.contains("openai") || lower.contains("chatgpt") {
        Some(include_bytes!("../ui/assets/presets/openai.svg"))
    } else if lower.contains("claude") || lower.contains("كلود") {
        Some(include_bytes!("../ui/assets/presets/claude.svg"))
    } else if lower.contains("steam") || lower.contains("ستيم") {
        Some(include_bytes!("../ui/assets/presets/steam.svg"))
    } else if lower.contains("playstation") || lower.contains("بلايستيشن") {
        Some(include_bytes!("../ui/assets/presets/playstation.svg"))
    } else if lower.contains("mada") || lower.contains("مدى") {
        Some(include_bytes!("../ui/assets/presets/mada.svg"))
    } else {
        None
    }
}

fn get_service_cover(name: &str, icon_data: Option<&str>) -> (bool, slint::Image) {
    if let Some(path_str) = icon_data {
        let p = Path::new(path_str);
        if p.exists() {
            if let Ok(img) = slint::Image::load_from_path(p) {
                return (true, img);
            }
        }
    }

    if let Some(svg_bytes) = get_preset_svg_bytes(name) {
        if let Ok(img) = slint::Image::load_from_svg_data(svg_bytes) {
            return (true, img);
        }
    }

    (false, slint::Image::default())
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
            reload_services_into_ui(&state, &main_window);
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
                    let _ = unsafe { win32::store_vault_key_wincred(pass) };
                    window.set_has_saved_credentials(true);
                    window.set_current_view("dashboard".into());
                    reload_services_into_ui(&state, &window);
                    if let Ok(key) = state.get_or_generate_recovery_key() {
                        window.set_emergency_key_display(key.into());
                    }
                    show_toast_message(&window, "تم إنشاء الخزنة بنجاح وحفظ الرمز في نظام ويندوز");
                }
                Err(e) => {
                    window.set_status_message(format!("فشل إنشاء الخزنة: {}", e).into());
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
                window.set_status_message("يرجى إدخال كلمة المرور".into());
                window.set_is_error_status(true);
                return;
            }

            match state.unlock_vault(pass) {
                Ok(_) => {
                    let _ = unsafe { win32::store_vault_key_wincred(pass) };
                    window.set_has_saved_credentials(true);
                    window.set_status_message("".into());
                    window.set_is_error_status(false);
                    window.set_current_view("dashboard".into());
                    window.set_unlock_pass("".into());
                    reload_services_into_ui(&state, &window);
                    if let Ok(key) = state.get_or_generate_recovery_key() {
                        window.set_emergency_key_display(key.into());
                    }
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
    // CALLBACK: TRY QUICK UNLOCK (WINCRED)
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
                    window.set_unlock_pass("".into());
                    reload_services_into_ui(&state, &window);
                    if let Ok(key) = state.get_or_generate_recovery_key() {
                        window.set_emergency_key_display(key.into());
                    }
                    show_toast_message(&window, "تم الفتح التلقائي بنجاح");
                }
                _ => {
                    window.set_status_message("لم يتم العثور على رمز محفوظ صالح في ويندوز".into());
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
    // CALLBACK: COPY TO CLIPBOARD
    // =========================================================================
    {
        let window_weak = main_window.as_weak();
        main_window.on_copy_to_clipboard(move |text| {
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
    // CALLBACK: COPY SERVICE PASSWORD
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
            window.set_active_service_name(name.clone());
            window.set_active_service_category(category);

            let (has_c, cover_img) = get_service_cover(&name, None);
            window.set_active_service_has_cover(has_c);
            window.set_active_service_cover(cover_img);

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
                        is_revealed: false,
                        actual_password: "".into(),
                    })
                    .collect();

                window.set_current_accounts(slint::ModelRc::new(slint::VecModel::from(items)));
                window.set_show_details_modal(true);
            }
        });
    }

    // =========================================================================
    // CALLBACK: TOGGLE REVEAL PASSWORD
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_toggle_reveal_password(move |service_id, account_id| {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            if let Ok(pwd) = state.get_account_password(service_id.as_str(), account_id.as_str()) {
                let accounts = window.get_current_accounts();
                let mut updated = Vec::new();
                for i in 0..accounts.row_count() {
                    if let Some(mut acc) = accounts.row_data(i) {
                        if acc.id == account_id {
                            acc.is_revealed = !acc.is_revealed;
                            if acc.is_revealed {
                                acc.actual_password = pwd.clone().into();
                            } else {
                                acc.actual_password = "".into();
                            }
                        }
                        updated.push(acc);
                    }
                }
                window.set_current_accounts(slint::ModelRc::new(slint::VecModel::from(updated)));
            }
        });
    }

    // =========================================================================
    // CALLBACK: SET AUTO LOCK TIMEOUT
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_set_auto_lock_timeout(move |mins| {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };
            state.set_auto_lock_timeout_mins(mins as u64);
            window.set_auto_lock_timeout_mins(mins);
            show_toast_message(&window, &format!("تم ضبط مهلة القفل على {} دقيقة", mins));
        });
    }

    // =========================================================================
    // CALLBACK: TOGGLE AUTO UNLOCK
    // =========================================================================
    {
        let window_weak = main_window.as_weak();
        main_window.on_toggle_auto_unlock(move |enable| {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };
            if !enable {
                let _ = unsafe { win32::delete_vault_key_wincred() };
                window.set_has_saved_credentials(false);
                window.set_auto_unlock_enabled(false);
                show_toast_message(&window, "تم تعطيل الفتح التلقائي وحذف الرمز من ويندوز");
            }
        });
    }

    // =========================================================================
    // CALLBACK: COVER IMAGE PICKER & REMOVAL
    // =========================================================================
    {
        let window_weak = main_window.as_weak();
        main_window.on_pick_service_cover(move || {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Images", &["png", "jpg", "jpeg", "webp", "svg", "bmp"])
                .set_title("اختر غلاف أو صورة الخدمة")
                .pick_file()
            {
                let path_str = path.to_string_lossy().to_string();
                if let Ok(img) = slint::Image::load_from_path(&path) {
                    window.set_new_service_icon_path(path_str.into());
                    window.set_new_service_has_custom_icon(true);
                    window.set_new_service_preview_icon(img);
                }
            }
        });
    }

    {
        let window_weak = main_window.as_weak();
        main_window.on_remove_service_cover(move || {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };
            window.set_new_service_icon_path("".into());
            window.set_new_service_has_custom_icon(false);
            window.set_new_service_preview_icon(slint::Image::default());
        });
    }

    // =========================================================================
    // CALLBACK: ADD SERVICE
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_add_service(
            move |name, category, username, password, email, recovery, notes, url, icon_path| {
                let window = match window_weak.upgrade() {
                    Some(w) => w,
                    None => return,
                };

                let name = name.trim();
                if name.is_empty() {
                    show_toast_message(&window, "يرجى كتابة اسم الخدمة");
                    return;
                }

                let icon_data = if icon_path.trim().is_empty() {
                    None
                } else {
                    Some(icon_path.trim().to_string())
                };

                match state.add_service(
                    name,
                    category.trim(),
                    icon_data,
                    username.trim(),
                    password.trim(),
                    email.trim(),
                    recovery.trim(),
                    notes.trim(),
                    url.trim(),
                ) {
                    Ok(_) => {
                        reload_services_into_ui(&state, &window);
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
                reload_services_into_ui(&state, &window);
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
                reload_services_into_ui(&state, &window);
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
                                is_revealed: false,
                                actual_password: "".into(),
                            })
                            .collect();

                        window.set_current_accounts(slint::ModelRc::new(slint::VecModel::from(items)));
                    }

                    reload_services_into_ui(&state, &window);
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
                            is_revealed: false,
                            actual_password: "".into(),
                        })
                        .collect();

                    window.set_current_accounts(slint::ModelRc::new(slint::VecModel::from(items)));
                }

                reload_services_into_ui(&state, &window);
                show_toast_message(&window, "تم حذف الحساب");
            }
        });
    }

    // =========================================================================
    // CALLBACK: REQUEST GENERATE PASSWORD
    // =========================================================================
    {
        let window_weak = main_window.as_weak();
        main_window.on_request_generate_password(move || {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return,
            };
            let pwd = password_gen::generate_password(16, true, true, true);
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
                        reload_services_into_ui(&state, &window);
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
    // CALLBACK: RECOVER VAULT
    // =========================================================================
    {
        let state = Arc::clone(&state);
        let window_weak = main_window.as_weak();
        main_window.on_recover_vault(move |recovery_key, new_pass, confirm_pass| {
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

            if new_p != confirm_pass.trim() {
                show_toast_message(&window, "كلمة المرور غير متطابقة مع التأكيد");
                return;
            }

            match state.recover_vault_with_key(key, new_p) {
                Ok(_) => {
                    window.set_show_recovery_modal(false);
                    window.set_current_view("dashboard".into());
                    reload_services_into_ui(&state, &window);
                    show_toast_message(&window, "تم استعادة الخزنة وتعيين كلمة المرور بنجاح");
                }
                Err(e) => show_toast_message(&window, &format!("فشل الاسترداد: {}", e)),
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
