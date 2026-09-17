use std::path::{Path, PathBuf};

use backend::api::{self, LocateParentSessionRequestDto, ParseBoundaryRequest};
use backend::export::{self, ExportWorklogRequest, ExportWorklogResult};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Clone, Debug, Deserialize)]
struct TauriFilterDto {
    show_you: bool,
    show_codex: bool,
    show_tool_call: bool,
    show_tool_result: bool,
    show_meta: bool,
}

impl From<TauriFilterDto> for api::FilterDto {
    fn from(filter: TauriFilterDto) -> Self {
        Self {
            show_you: filter.show_you,
            show_codex: filter.show_codex,
            show_tool_call: filter.show_tool_call,
            show_tool_result: filter.show_tool_result,
            show_meta: filter.show_meta,
        }
    }
}

#[tauri::command]
fn parse_selected_jsonl(path: String, filter: TauriFilterDto) -> Result<Value, Value> {
    api::parse_for_transport(ParseBoundaryRequest {
        path,
        filter: Some(filter.into()),
    })
    .map(|response| response.to_json())
    .map_err(|error| error.to_json())
}

#[tauri::command]
fn locate_parent_session(parent_thread_id: String, current_path: String) -> Result<Value, Value> {
    api::locate_parent_session_for_transport(LocateParentSessionRequestDto {
        current_path,
        parent_thread_id,
    })
    .map(|response| response.to_json())
    .map_err(|error| error.to_json())
}

#[tauri::command]
fn export_worklog(source_path: String, parent_directory: String) -> Result<Value, Value> {
    let response = export::export_worklog(ExportWorklogRequest {
        source_path: PathBuf::from(source_path),
        parent_directory: PathBuf::from(parent_directory),
    })
    .map_err(|error| error.to_json())?;
    let folder_open_result = tauri_plugin_opener::open_path(&response.bundle_path, None::<&str>)
        .map_err(|error| error.to_string());

    Ok(export_worklog_response(response, folder_open_result))
}

fn export_worklog_response(
    response: ExportWorklogResult,
    folder_open_result: Result<(), String>,
) -> Value {
    let mut value = response.to_json();
    let (folder_opened, folder_open_error) = match folder_open_result {
        Ok(()) => (true, Value::Null),
        Err(error) => (false, json!(error)),
    };
    value["folder_opened"] = json!(folder_opened);
    value["folder_open_error"] = folder_open_error;
    value
}

#[tauri::command]
fn resolve_jsonl_initial_directory(remembered_directory: Option<String>) -> String {
    if let Some(remembered_directory) = remembered_directory
        .as_deref()
        .filter(|path| !path.trim().is_empty())
        .map(Path::new)
        .filter(|path| path.is_dir())
    {
        return remembered_directory.to_string_lossy().into_owned();
    }

    let codex_home = std::env::var_os("CODEX_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let user_home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);

    let candidates = [
        codex_home.as_ref().map(|path| path.join("sessions")),
        user_home
            .as_ref()
            .map(|path| path.join(".codex").join("sessions")),
        user_home,
    ];

    candidates
        .into_iter()
        .flatten()
        .find(|path| path.is_dir())
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .canonicalize()
                .unwrap_or_else(|_| PathBuf::from("."))
        })
        .to_string_lossy()
        .into_owned()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            export_worklog,
            locate_parent_session,
            parse_selected_jsonl,
            resolve_jsonl_initial_directory
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn export_result(refreshed: bool) -> ExportWorklogResult {
        ExportWorklogResult {
            bundle_path: "E:\\exports\\codex-worklog\\2026-06-19\\161859_session-019ee0ad"
                .to_owned(),
            generated_files: vec!["000_index.md".to_owned(), "manifest.json".to_owned()],
            refreshed,
        }
    }

    #[test]
    fn export_response_reports_folder_open_success() {
        let response = export_worklog_response(export_result(false), Ok(()));

        assert_eq!(response["status"], "exported");
        assert_eq!(response["folder_opened"], true);
        assert!(response["folder_open_error"].is_null());
    }

    #[test]
    fn export_response_preserves_success_when_folder_open_fails() {
        let response =
            export_worklog_response(export_result(false), Err("Explorer unavailable".to_owned()));

        assert_eq!(response["status"], "exported");
        assert_eq!(response["folder_opened"], false);
        assert_eq!(response["folder_open_error"], "Explorer unavailable");
    }

    #[test]
    fn refreshed_export_preserves_success_when_folder_open_fails() {
        let response =
            export_worklog_response(export_result(true), Err("Explorer unavailable".to_owned()));

        assert_eq!(response["status"], "exported");
        assert_eq!(response["refreshed"], true);
        assert_eq!(response["folder_opened"], false);
    }
}
