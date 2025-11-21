// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod errors;
mod models;
mod profiles;
mod s3_adapter;

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
            multipart_upload_start,
            multipart_upload_part,
            multipart_upload_part_from_file,
            multipart_upload_complete,
            multipart_upload_abort,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
