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
mod report_pdf;
mod aleapp;
mod settings;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_dir = app.path().app_data_dir()
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
            extractors::mobile::pull_media,
            extractors::mobile::list_usb_devices,
            extractors::mobile::get_device_info,
            // === MOBILE AVANCÉ ===
            extractors::mobile::advanced::classify_device,
            extractors::mobile::advanced::diagnose_device,
            extractors::mobile::advanced::get_broken_screen_options,
            extractors::mobile::advanced::attempt_adb_unlock_navigation,
            extractors::mobile::advanced::start_scrcpy_session,
            extractors::mobile::advanced::get_extraction_matrix,
            extractors::mobile::advanced::extract_physical_mtk,
            extractors::mobile::advanced::extract_physical_edl,
            // === PROFILS CHIPSETS ===
            extractors::mobile::chipsets::detect_chipset,
            extractors::mobile::chipsets::list_recent_chipsets,
            // === PROFILS IOS ===
            extractors::mobile::ios_profiles::detect_ios_profile,
            extractors::mobile::ios_profiles::list_checkm8_devices,
            extractors::mobile::ios_profiles::list_ios_profiles,
            // === ACQUISITION APPAREILS VERROUILLÉS / NON DÉBOGABLES ===
            extractors::mobile::locked_extraction::locked_acquisition_plan,
            extractors::mobile::locked_extraction::acquire_sim_image,
            extractors::mobile::locked_extraction::acquire_removable_image,
            extractors::mobile::ios::analyze_ios_backup,
            extractors::mobile::ios::list_ios_devices,
            extractors::mobile::ios::create_ios_backup,
            // === SYMBIAN ===
            extractors::mobile::symbian::analyze_nbu_backup,
            extractors::mobile::symbian::list_nokia_devices,
            extractors::mobile::symbian::extract_nokia_sim,
            // === CARTE SIM ===
            extractors::mobile::sim::extract_sim,
            extractors::mobile::sim::clone_sim,
            extractors::mobile::sim::detect_sim_readers,
            // === HISTORIQUE D'APPELS ===
            extractors::mobile::call_history::extract_call_history,
            extractors::mobile::call_history::get_call_statistics,
            extractors::mobile::call_history::detect_patterns,
            extractors::mobile::call_history::resolve_contacts,
            extractors::mobile::call_history::detect_call_chains,
            extractors::mobile::call_history::get_call_timeline,
            extractors::mobile::call_history::export_calls_csv,
            // === REPERTOIRE TELEPHONIQUE ===
            extractors::mobile::contacts::extract_contacts,
            extractors::mobile::contacts::search_contacts,
            // === DETECTION IMAGES ===
            analyzer::image_detector::detect_images,
            analyzer::image_detector::detect_crypto_messages,
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
            decoders::whatsapp::decrypt_whatsapp_backup,
            decoders::whatsapp::parse_wa_contacts,
            decoders::whatsapp::scan_whatsapp_folder,
            decoders::whatsapp::analyze_whatsapp_folder,
            decoders::orange_money::parse_mobile_money,
            decoders::orange_money::scan_money_from_messages,
            decoders::file_carver::carve_files,
            decoders::sqlite_recover::recover_deleted_records,
            decoders::android_backup::parse_android_backup,
            decoders::android_backup::decode_database,
            decoders::passwords::extract_passwords,
            decoders::passwords::extract_ios_keychain,
            decoders::email::extract_emails,
            decoders::email::extract_ios_emails,
            decoders::browser_history::extract_browser_history,
            decoders::browser_history::extract_safari_history,
            decoders::browser_history::extract_search_queries,
            decoders::browser_history::extract_downloads,
            decoders::browser_history::build_interest_profile,
            decoders::browser_history::export_history_csv,
            aleapp::parse_with_aleapp,
            // === ANALYSEURS ===
            analyzer::timeline::build_timeline,
            analyzer::timeline::get_timeline_events,
            analyzer::timeline::build_comprehensive_timeline,
            analyzer::timeline::correlate_events,
            analyzer::timeline::detect_gaps,
            analyzer::timeline::filter_timeline,
            analyzer::timeline::get_timeline_stats,
            analyzer::timeline::export_timeline_csv,
            analyzer::money_flow::analyze_money_flow,
            analyzer::link_graph::build_link_graph,
            analyzer::link_graph::build_comprehensive_graph,
            analyzer::link_graph::analyze_graph,
            analyzer::link_graph::find_connections,
            analyzer::link_graph::get_node_details,
            analyzer::link_graph::export_graph,
            analyzer::data_analysis::get_data_analysis,
            analyzer::geolocation::extract_geolocation,
            analyzer::geolocation::add_geo_analysis_point,
            analyzer::geolocation::analyze_geo_trajectories,
            analyzer::geolocation::analyze_trajectories,
            analyzer::geolocation::infer_home_work,
            analyzer::geolocation::detect_impossible_travel,
            analyzer::geolocation::calculate_dwell_time,
            analyzer::geolocation::export_geojson,
            // === CSM ===
            csm::create_custody,
            csm::add_custody_step,
            csm::get_custody_chains,
            csm::get_custody,
            csm::transfer_custody,
            // === CARTOGRAPHIE ===
            cartography::create_map,
            cartography::add_geo_point,
            cartography::add_geo_points_batch,
            cartography::generate_heatmap,
            cartography::build_connections,
            cartography::reconstruct_trajectories,
            cartography::add_geofence,
            cartography::detect_geofence_breaches,
            cartography::get_map_data,
            cartography::export_map_geojson,
            cartography::get_point_details,
            cartography::delete_map,
            cartography::list_maps,
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
            filters::clear_demo_data,
            filters::save_extracted_to_file,
            // === RAPPORTS ===
            reports::generate_report,
            reports::generate_general_report,
            reports::generate_specific_user_report,
            reports::list_reports,
            reports::get_report,
            reports::sign_report,
            reports::get_available_templates,
            report_pdf::generate_pdf_report,
            // === AUDIT ===
            audit::get_audit_log,
            audit::log_action,
            // === CASES ===
            case_manager::create_case,
            case_manager::list_cases,
            case_manager::get_case,
            case_manager::export_chain_of_custody,
            case_manager::save_extracted_data,
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
