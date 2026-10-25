mod ai;
mod history;
mod metadata;
mod project;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(project::ProjectSession::default())
        .setup(|app| {
            ai::start(app.handle().clone()).map_err(std::io::Error::other)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            project::open_project,
            project::create_project,
            project::list_project_tree,
            project::read_document,
            project::save_document,
            project::create_document,
            project::create_directory,
            project::move_entry,
            project::delete_entry,
            project::set_pinned,
            project::set_locked,
            project::get_project_data,
            project::upsert_term,
            project::delete_term,
            project::set_term_exception,
            project::upsert_annotation,
            project::delete_annotation,
            project::search_documents,
            project::list_resources,
            project::import_resource,
            project::import_image,
            project::read_project_image,
            project::read_project_rules,
            project::save_project_rules,
            project::set_dirty_draft,
            project::publish_draft,
            ai::bridge_info,
            ai::list_history,
            ai::undo_change,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
