// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod crypto;
mod errors;
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
