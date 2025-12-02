// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cache_manager;
mod commands;
mod crypto;
mod database;
mod errors;
mod index_manager;
mod metrics;
mod metrics_storage;
mod models;
mod profiles;
mod s3_adapter;
mod validation;

use commands::*;

fn main() {
    let app_state = AppState::new();

    tauri::Builder::default()
        .manage(app_state)
        .setup(|_app| {
            // Auto-purge metrics data older than 30 days at startup
            std::thread::spawn(|| {
                metrics_storage::auto_purge_on_startup();
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_profiles,
            save_profile,
            delete_profile,
            test_connection,
            list_buckets,
            create_bucket,
            delete_bucket,
            can_delete_bucket,
            get_bucket_acl,
            get_bucket_configuration,
            calculate_bucket_stats,
            list_objects,
            get_object,
            put_object,
            delete_object,
            copy_object,
            change_content_type,
            create_folder,
            generate_presigned_url,
            calculate_folder_size,
            delete_folder,
            list_object_versions,
            get_file_size,
            upload_file,
            cancel_upload,
            get_object_tags,
            put_object_tags,
            delete_object_tags,
            get_object_metadata,
            update_object_metadata,
            // Index management commands
            start_initial_index,
            cancel_indexing,
            get_bucket_index_stats,
            get_prefix_index_stats,
            clear_bucket_index,
            is_bucket_indexed,
            is_bucket_index_complete,
            is_prefix_known,
            get_prefix_status,
            is_prefix_discovered_only,
            search_objects_in_index,
            get_all_bucket_indexes,
            get_index_file_size,
            // Download commands (streaming to disk)
            download_file,
            cancel_download,
            // Cache management commands
            get_cache_status,
            warmup_profile_cache,
            cleanup_profile_cache,
            clear_all_caches,
            // Metrics commands
            get_metrics_today,
            get_metrics_history,
            get_metrics_hourly,
            get_metrics_by_operation,
            get_metrics_errors,
            get_metrics_top_buckets,
            get_metrics_recent,
            get_metrics_failed,
            get_metrics_storage_info,
            purge_metrics,
            clear_metrics,
            record_cache_event,
            get_cache_summary,
            get_today_cache_stats,
            // Clipboard upload commands
            upload_from_bytes,
            read_clipboard_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
