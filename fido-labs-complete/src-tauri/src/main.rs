// FIDO-LABS v3.0 — Forensic Integrated Data Operator
// Extraction + Analyse + CSM + Cartographie + Graphes + Calendrier + Filtres + Rapports
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod extractors;
mod decoders;
mod analyzer;
mod audit;
mod case_manager;
mod license;
mod csm;
mod cartography;
mod calendar;
mod filters;
mod reports;
mod settings;

use tauri::Manager;

fn main() {
    // Initialiser les données de démo pour les filtres
    filters::seed_filtered_items();

    tauri::Builder::default()
        .setup(|app| {
            let app_dir = app.path_resolver().app_data_dir()
                .expect("Impossible de trouver le répertoire app data");
            std::fs::create_dir_all(&app_dir).ok();
            audit::init_audit_db(&app_dir.to_string_lossy()).ok();
            case_manager::init_cases_db(&app_dir.to_string_lossy()).ok();
            log::info!("FIDO-LABS v3.0 démarré — app_dir: {}", app_dir.display());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // === EXTRACTEURS MOBILE ===
            extractors::mobile::detect_device,
            extractors::mobile::extract_logical,
            extractors::mobile::list_usb_devices,
            extractors::mobile::get_device_info,
            // === MOBILE AVANCÉ ===
            extractors::mobile::advanced::diagnose_device,
            extractors::mobile::advanced::get_broken_screen_options,
            extractors::mobile::advanced::attempt_adb_unlock_navigation,
            extractors::mobile::advanced::start_scrcpy_session,
            extractors::mobile::advanced::get_extraction_matrix,
            // === SD CARD ===
            extractors::sdcard::image_sd,
            extractors::sdcard::carve_sd_card,
            extractors::sdcard::list_partitions,
            // === DISQUE ===
            extractors::disk::create_image,
            extractors::disk::list_disks,
            extractors::disk::verify_image,
            // === DÉCODEURS ===
            decoders::whatsapp::parse_whatsapp_db,
            decoders::orange_money::parse_mobile_money,
            decoders::file_carver::carve_files,
            // === ANALYSEURS ===
            analyzer::timeline::build_timeline,
            analyzer::timeline::get_timeline_events,
            analyzer::money_flow::analyze_money_flow,
            analyzer::link_graph::build_link_graph,
            // === CSM ===
            csm::create_custody,
            csm::add_custody_step,
            csm::get_custody_chains,
            csm::get_custody,
            csm::transfer_custody,
            // === CARTOGRAPHIE ===
            cartography::create_map,
            cartography::add_geo_point,
            cartography::get_map_summary,
            // === CALENDRIER ===
            calendar::create_event,
            calendar::get_events,
            calendar::get_events_by_day,
            calendar::update_event_status,
            calendar::add_reminder,
            calendar::get_weekly_summary,
            // === FILTRES ===
            filters::create_filter,
            filters::list_filters,
            filters::apply_filter,
            filters::save_filter_as_template,
            // === RAPPORTS ===
            reports::generate_report,
            reports::generate_general_report,
            reports::generate_specific_user_report,
            reports::list_reports,
            reports::get_report,
            reports::sign_report,
            reports::get_available_templates,
            // === AUDIT ===
            audit::get_audit_log,
            audit::log_action,
            // === CASES ===
            case_manager::create_case,
            case_manager::list_cases,
            case_manager::get_case,
            case_manager::export_chain_of_custody,
            // === LICENCE ===
            license::activate_license,
            license::get_license_status,
            license::get_available_modules,
            license::check_module_access,
            // === SETTINGS ===
            settings::get_app_info,
            settings::get_keyboard_shortcuts,
            settings::get_export_templates,
        ])
        .run(tauri::generate_context!())
        .expect("Erreur lors du lancement de FIDO-LABS");
}
