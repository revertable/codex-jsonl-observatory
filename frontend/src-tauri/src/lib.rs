use std::path::{Path, PathBuf};

use backend::api::{self, LocateParentSessionRequestDto, ParseBoundaryRequest};
use backend::export::{self, ExportWorklogError, ExportWorklogRequest, ExportWorklogResult};
use serde::{Deserialize, Serialize};

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
fn parse_selected_jsonl(
    path: String,
    filter: TauriFilterDto,
) -> api::ApiResult<api::ParseResponseDto> {
    api::parse_for_transport(ParseBoundaryRequest {
        path,
        filter: Some(filter.into()),
    })
}

#[tauri::command]
fn locate_parent_session(
    parent_thread_id: String,
    current_path: String,
) -> api::ApiResult<api::LocateParentSessionResponseDto> {
    api::locate_parent_session_for_transport(LocateParentSessionRequestDto {
        current_path,
        parent_thread_id,
    })
}

#[derive(Debug, Serialize)]
struct TauriErrorResponse {
    error: TauriApiError,
}

#[derive(Debug, Serialize)]
struct TauriApiError {
    code: &'static str,
    message: String,
}

impl From<ExportWorklogError> for TauriErrorResponse {
    fn from(error: ExportWorklogError) -> Self {
        Self {
            error: TauriApiError {
                code: error.code,
                message: error.message,
            },
        }
    }
}

#[derive(Debug, Serialize)]
struct ExportWorklogResponse {
    status: &'static str,
    bundle_path: String,
    generated_files: Vec<String>,
    refreshed: bool,
    folder_opened: bool,
    folder_open_error: Option<String>,
}

#[tauri::command]
fn export_worklog(
    source_path: String,
    parent_directory: String,
) -> Result<ExportWorklogResponse, TauriErrorResponse> {
    let response = export::export_worklog(ExportWorklogRequest {
        source_path: PathBuf::from(source_path),
        parent_directory: PathBuf::from(parent_directory),
    })
    .map_err(TauriErrorResponse::from)?;
    let folder_open_result = tauri_plugin_opener::open_path(&response.bundle_path, None::<&str>)
        .map_err(|error| error.to_string());

    Ok(export_worklog_response(response, folder_open_result))
}

fn export_worklog_response(
    response: ExportWorklogResult,
    folder_open_result: Result<(), String>,
) -> ExportWorklogResponse {
    let (folder_opened, folder_open_error) = match folder_open_result {
        Ok(()) => (true, None),
        Err(error) => (false, Some(error)),
    };
    ExportWorklogResponse {
        status: "exported",
        bundle_path: response.bundle_path,
        generated_files: response.generated_files,
        refreshed: response.refreshed,
        folder_opened,
        folder_open_error,
    }
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

    fn response_json(response: ExportWorklogResponse) -> serde_json::Value {
        serde_json::to_value(response).expect("typed export response serializes")
    }

    #[test]
    fn export_response_reports_folder_open_success() {
        let response = response_json(export_worklog_response(export_result(false), Ok(())));

        assert_eq!(response["status"], "exported");
        assert_eq!(response["folder_opened"], true);
        assert!(response["folder_open_error"].is_null());
    }

    #[test]
    fn export_response_preserves_success_when_folder_open_fails() {
        let response = response_json(export_worklog_response(
            export_result(false),
            Err("Explorer unavailable".to_owned()),
        ));

        assert_eq!(response["status"], "exported");
        assert_eq!(response["folder_opened"], false);
        assert_eq!(response["folder_open_error"], "Explorer unavailable");
    }

    #[test]
    fn refreshed_export_preserves_success_when_folder_open_fails() {
        let response = response_json(export_worklog_response(
            export_result(true),
            Err("Explorer unavailable".to_owned()),
        ));

        assert_eq!(response["status"], "exported");
        assert_eq!(response["refreshed"], true);
        assert_eq!(response["folder_opened"], false);
    }
}
