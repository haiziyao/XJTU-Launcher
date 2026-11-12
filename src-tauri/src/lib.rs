// Domain folders group related modules; the re-exports keep the long-standing
// `crate::<module>` paths used by command modules and Tauri registration.
mod agent;
mod campus;
mod commands;
mod core;
mod features;
mod identity;
mod plugins;
mod school;
mod state;
mod workbench;

pub(crate) use agent::{expose, mcp, providers};
pub(crate) use campus::{browser as campus_browser, timetable_custom, tool_names};
pub(crate) use core::{cache, config, network, storage, vault};
pub(crate) use features::{
    community, fonts, icons, personal_image, pet_desktop, resources, updates,
};
pub(crate) use identity::{accounts, auth};
pub(crate) use plugins::{capture as capture_store, settings as plugin_settings};

use tauri::Manager;

pub(crate) use commands::expose::remember_expose_port;
pub(crate) use commands::mcp::mcp_path;
pub(crate) use commands::workbench::{WorkbenchEdit, WorkbenchRead};
pub(crate) use commands::{
    apply_timetable_edit, download_lms_file, merge_cached_timetable, prepare_evaluations,
    providers_path, run_lms_homework, run_school_query, run_timetable, run_workbench_edit,
    run_workbench_read, submit_evaluations, timetable_customizations,
};
pub(crate) use state::{accounts_path, config_path, device_visitor, main_only, AppState};

pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .manage(pet_desktop::State::default())
        .manage(updates::State::default())
        .manage(expose::State::default())
        .setup(|app| {
            vault::initialize(storage::home(app.handle())?)?;
            // Moves the agent data and MCP config older builds wrote to
            // ~/.xjtu-launcher into the single data folder; the credentials
            // file next to them stays where the convention says it lives.
            let _ = storage::adopt_legacy(app.handle());
            // The plugin resource folder is part of the visible file area, so
            // it exists even before a plugin asks for a file.
            if let Ok(dir) = storage::file_path(app.handle(), "plugin") {
                let _ = std::fs::create_dir_all(dir);
            }
            campus_browser::install_menu(app.handle())?;
            pet_desktop::initialize(app.handle())?;
            // Resume the endpoint only if the user left it on; a bind failure is
            // kept in the shared state so the personal page can show the reason.
            if let Ok(config) = config_path(app.handle()).and_then(|path| config::load(&path)) {
                if config.mcp_server_enabled {
                    let state = app.state::<expose::State>();
                    if let Ok(port) =
                        expose::start(app.handle(), state.inner(), config.mcp_server_port)
                    {
                        remember_expose_port(app.handle(), port);
                    }
                }
            }
            Ok(())
        })
        .on_menu_event(|app, event| campus_browser::menu_event(app, event.id().as_ref()))
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                let _ = campus_browser::close_all(window.app_handle());
                // The hidden pet window would otherwise keep the process alive
                // after the main window is closed.
                if let Some(pet) = window.app_handle().get_webview_window(pet_desktop::LABEL) {
                    let _ = pet.destroy();
                }
            }
            if window.label() == pet_desktop::LABEL {
                if let tauri::WindowEvent::Moved(position) = event {
                    pet_desktop::remember_moved(window.app_handle(), *position);
                }
            }
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::config::plugin_settings,
            commands::config::save_plugin_settings,
            fonts::fonts_state,
            fonts::font_download,
            fonts::font_download_cancel,
            fonts::font_add_local,
            fonts::font_remove,
            fonts::font_data,
            updates::update_status,
            updates::check_app_update,
            updates::install_app_update,
            updates::restart_after_update,
            commands::school::school_catalog,
            commands::system::entry_icons,
            commands::school::school_query,
            commands::school::calendar_image,
            commands::network::campus_notices,
            cache::cache_read,
            pet_desktop::pet_view,
            pet_desktop::pet_restore,
            pet_desktop::pet_open_settings,
            pet_desktop::pet_set_bar_hidden,
            pet_desktop::pet_push_notification,
            pet_desktop::pet_preview,
            commands::lms::lms_open_file,
            commands::lms::lms_save_file,
            commands::lms::lms_replay_videos,
            commands::system::save_export,
            capture_store::capture_session_load,
            capture_store::capture_session_save,
            capture_store::capture_page_add,
            capture_store::capture_pages_read,
            capture_store::capture_pages_delete,
            capture_store::capture_pages_order,
            capture_store::capture_session_clear,
            commands::workbench::workbench_overview,
            commands::workbench::workbench_folder,
            commands::workbench::workbench_create_folder,
            commands::workbench::workbench_import,
            commands::workbench::workbench_update,
            commands::workbench::workbench_delete,
            commands::workbench::workbench_open,
            commands::workbench::workbench_reveal,
            commands::workbench::workbench_sync,
            commands::workbench::workbench_submit_preview,
            commands::workbench::workbench_submit,
            commands::school::school_prepare_evaluations,
            commands::school::school_submit_evaluations,
            commands::school::school_cancel_evaluations,
            commands::providers::list_model_providers,
            commands::providers::list_provider_models,
            commands::providers::save_model_provider,
            commands::providers::remove_model_provider,
            commands::providers::select_model_provider,
            commands::providers::save_provider_models,
            commands::providers::set_provider_model,
            commands::providers::provider_balance,
            commands::providers::test_provider,
            commands::providers::reveal_provider_key,
            commands::mcp::list_mcp_servers,
            commands::mcp::save_mcp_server,
            commands::mcp::remove_mcp_server,
            commands::mcp::set_mcp_server_enabled,
            commands::mcp::probe_mcp_server,
            commands::agent::agent_list,
            commands::agent::agent_workspaces,
            commands::agent::agent_create_workspace,
            commands::agent::agent_set_workspace,
            commands::agent::agent_get,
            commands::agent::agent_new,
            commands::agent::agent_last_opened,
            commands::agent::agent_set_last_opened,
            commands::agent::agent_delete,
            commands::agent::agent_rename,
            commands::agent::agent_send,
            commands::agent::agent_cancel,
            commands::agent::agent_tools,
            community::agent_search_status,
            community::community_agent_search,
            resources::resource_sites,
            commands::expose::expose_status,
            commands::expose::expose_tools,
            commands::expose::expose_set_enabled,
            commands::expose::expose_set_port,
            commands::expose::expose_probe,
            commands::agent::agent_presets,
            commands::agent::agent_save_preset,
            commands::agent::agent_remove_preset,
            commands::agent::agent_memories,
            commands::agent::agent_save_memory,
            commands::agent::agent_remove_memory,
            commands::agent::agent_set_role,
            commands::agent::agent_pin,
            commands::personal::personal_image_source,
            commands::personal::read_personal_image,
            commands::config::load_config,
            commands::config::save_config,
            commands::config::list_plugins,
            commands::accounts::account_status,
            commands::accounts::account_login_state,
            commands::accounts::forget_account_password,
            commands::accounts::start_account_login,
            commands::accounts::advance_account_login,
            commands::accounts::account_login_captcha,
            commands::accounts::account_login_qr,
            commands::accounts::cancel_account_login,
            commands::accounts::list_accounts,
            commands::accounts::switch_account,
            commands::accounts::remove_account,
            commands::accounts::set_account_avatar,
            commands::accounts::logout,
            commands::accounts::save_api_key,
            commands::timetable::query_timetable,
            commands::timetable::edit_timetable,
            commands::session::refresh_direct_session,
            commands::session::refresh_fitness_session,
            commands::session::refresh_profile_session,
            commands::session::refresh_lms_session,
            commands::session::refresh_library_session,
            commands::session::refresh_venue_session,
            commands::network::check_network,
            commands::network::open_external,
            commands::storage::storage_status,
            commands::storage::storage_preview,
            commands::storage::storage_migrate,
            commands::storage::storage_reveal,
            commands::storage::storage_display_path,
            commands::storage::storage_download_path
        ])
        .run(tauri::generate_context!())
        .expect("无法启动 XJTU Launcher");
}
