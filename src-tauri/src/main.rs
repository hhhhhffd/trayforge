// No console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod hotkey;
mod picker;
mod settings;
mod taskbar;
mod totp_store;
mod window;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    RunEvent,
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::ShortcutState;

fn main() {
    let app = tauri::Builder::default()
        // Must be first: a second launch toggles the running instance's window.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window::toggle(app, None)
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        window::toggle(app, None);
                    }
                })
                .build(),
        )
        .manage(hotkey::CurrentHotkey::default())
        .manage(window::PickerActive::default())
        .manage(window::LastBlurClose::default())
        .manage(commands::Drafts::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_tools,
            commands::process_tool,
            commands::get_settings,
            commands::save_settings,
            commands::totp_generate,
            commands::totp_parse_uri,
            commands::totp_load_accounts,
            commands::totp_save_accounts,
            commands::get_drafts,
            commands::set_draft,
            commands::decode_qr_image,
            commands::http_request,
            commands::pick_screen_color,
            commands::read_text_file,
            commands::write_text_file,
            commands::hide_window,
            commands::window_ready,
            commands::set_launch_at_login,
            commands::reset_window_state,
        ])
        .setup(|app| {
            let handle = app.handle();
            let open = MenuItem::with_id(handle, "open", "Open Trayforge", true, None::<&str>)?;
            let settings_item =
                MenuItem::with_id(handle, "settings", "Settings", true, None::<&str>)?;
            let quit = MenuItem::with_id(handle, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(
                handle,
                &[
                    &open,
                    &settings_item,
                    &PredefinedMenuItem::separator(handle)?,
                    &quit,
                ],
            )?;

            let icon = app
                .default_window_icon()
                .cloned()
                .ok_or("missing default window icon")?;
            TrayIconBuilder::new()
                .icon(icon)
                .tooltip("Trayforge")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => window::show(app, None),
                    "settings" => window::show(app, Some("settings")),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        position,
                        ..
                    } = event
                    {
                        window::toggle_from_tray(tray.app_handle(), position);
                    }
                })
                .build(handle)?;

            let accelerator = settings::hotkey(&settings::load_lenient(handle));
            if let Err(e) = hotkey::apply(handle, &accelerator) {
                // The tray still works, so a taken hotkey must not stop the app.
                eprintln!("trayforge: {e}");
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Trayforge");

    app.run(|_app, event| {
        // The app lives in the tray: closing the last window must not end the process.
        if let RunEvent::ExitRequested {
            code: None, api, ..
        } = event
        {
            api.prevent_exit();
        }
    });
}
