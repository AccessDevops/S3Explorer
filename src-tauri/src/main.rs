// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod crypto;
mod database;
mod errors;
mod index_manager;
mod metrics;
mod models;
mod profiles;
mod s3_adapter;
mod validation;

use commands::*;

fn main() {
    let app_state = AppState::new();

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            list_profiles,
            save_profile,
            delete_profile,
            test_connection,
            list_buckets,
            create_bucket,
            get_bucket_acl,
            calculate_bucket_stats,
            estimate_bucket_stats,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
