pub mod fm;

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder},
    AppHandle, Emitter, Manager, Runtime, State,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

use fm::{
    analyze_identity_from_engine_with_progress, analyze_save, analyze_save_identity_with_progress,
    calibrating_panel, compare_saves, emit_done, export_diagnostic, parse_container,
    probe_known_value, run_fm_analysis, validate_fm_save_path, AnalyzeOptions, CalibratingMetadata,
    FmEngineResult, FmQuickValidation, FmSaveAnalysis, FmSaveAnalysisResult, FmSaveDiff,
    FmSaveInspection, FmSaveMetadata, KnownValueProbeResult,
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecentMenuItem {
    path: String,
    label: String,
}

struct RecentMenuState(Mutex<Vec<String>>);
struct AnalysisCancelState(Mutex<Option<Arc<AtomicBool>>>);

fn emit_menu_action(app: &AppHandle, action: &str) {
    let _ = app.emit("scoutiq://menu", action);
}

fn emit_open_recent(app: &AppHandle, path: &str) {
    let _ = app.emit("scoutiq://open-recent", path);
}

fn build_recent_submenu<R: Runtime>(
    app: &AppHandle<R>,
    recent: &[RecentMenuItem],
) -> tauri::Result<tauri::menu::Submenu<R>> {
    let mut recent_menu = SubmenuBuilder::new(app, "Zuletzt geöffnet");
    if recent.is_empty() {
        let empty = MenuItemBuilder::with_id("recent::empty", "Keine Einträge")
            .enabled(false)
            .build(app)?;
        recent_menu = recent_menu.item(&empty);
    } else {
        for (index, item) in recent.iter().enumerate() {
            let id = format!("recent::{index}");
            let entry = MenuItemBuilder::with_id(id, &item.label).build(app)?;
            recent_menu = recent_menu.item(&entry);
        }
    }
    recent_menu.build()
}

fn build_app_menu<R: Runtime>(
    app: &AppHandle<R>,
    recent: &[RecentMenuItem],
) -> tauri::Result<tauri::menu::Menu<R>> {
    let open_fm = MenuItemBuilder::with_id("open-fm-save", "FM-Spielstand öffnen…").build(app)?;
    let save_project = MenuItemBuilder::with_id("save-project", "Projekt speichern").build(app)?;
    let save_project_as =
        MenuItemBuilder::with_id("save-project-as", "Projekt speichern unter…").build(app)?;
    let reload = MenuItemBuilder::with_id("view-reload", "Neu laden").build(app)?;
    let about = MenuItemBuilder::with_id("help-about", "Über ScoutIQ").build(app)?;
    let recent_menu = build_recent_submenu(app, recent)?;

    let file_menu = SubmenuBuilder::new(app, "Datei")
        .item(&open_fm)
        .item(&recent_menu)
        .separator()
        .item(&save_project)
        .item(&save_project_as)
        .separator()
        .item(&PredefinedMenuItem::close_window(
            app,
            Some("Fenster schließen"),
        )?)
        .build()?;

    let view_menu = SubmenuBuilder::new(app, "Ansicht").item(&reload).build()?;
    let help_menu = SubmenuBuilder::new(app, "Hilfe").item(&about).build()?;

    MenuBuilder::new(app)
        .item(&file_menu)
        .item(&view_menu)
        .item(&help_menu)
        .build()
}

#[tauri::command]
fn update_recent_menu(
    app: AppHandle,
    state: State<'_, RecentMenuState>,
    items: Vec<RecentMenuItem>,
) -> Result<(), String> {
    {
        let mut paths = state.0.lock().map_err(|e| e.to_string())?;
        *paths = items.iter().map(|item| item.path.clone()).collect();
    }
    let menu = build_app_menu(&app, &items).map_err(|e| e.to_string())?;
    app.set_menu(menu).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn validate_fm_save(path: String) -> Result<FmQuickValidation, String> {
    if path.trim().is_empty() {
        return Err("FM-Spielstand konnte nicht geöffnet werden.".into());
    }
    tauri::async_runtime::spawn_blocking(move || validate_fm_save_path(&path))
        .await
        .map_err(|_| "Unbekannter Parserfehler.".to_string())?
}

/// Generic FM26 engine parse (streaming, cached metadata, no invented fields).
#[tauri::command]
async fn parse_fm_container(path: String) -> Result<FmEngineResult, String> {
    if path.trim().is_empty() {
        return Err("FM-Spielstand konnte nicht geöffnet werden.".into());
    }
    tauri::async_runtime::spawn_blocking(move || parse_container(&path))
        .await
        .map_err(|_| "Unbekannter Parserfehler.".to_string())?
}

/// Cancellable engine parse with progress events (`scoutiq://fm-progress`).
#[tauri::command]
fn start_fm_engine_analysis(
    app: AppHandle,
    cancel_state: State<'_, AnalysisCancelState>,
    path: String,
) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("FM-Spielstand konnte nicht geöffnet werden.".into());
    }
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut slot = cancel_state.0.lock().map_err(|e| e.to_string())?;
        if let Some(prev) = slot.as_ref() {
            prev.store(true, Ordering::SeqCst);
        }
        *slot = Some(Arc::clone(&cancel));
    }
    let app_job = app.clone();
    std::thread::Builder::new()
        .name("fm-engine".into())
        .spawn(move || {
            let result = analyze_save(
                &path,
                AnalyzeOptions {
                    use_cache: true,
                    emit_progress: true,
                    cancel: Some(&cancel),
                    app: Some(&app_job),
                },
            );
            match result {
                Ok(engine) => {
                    let identity =
                        analyze_identity_from_engine_with_progress(&engine, Some(&app_job));
                    let _ = app_job.emit(
                        "scoutiq://fm-engine-done",
                        serde_json::json!({
                            "ok": true,
                            "result": engine,
                            "identity": identity,
                        }),
                    );
                }
                Err(error) => {
                    let _ = app_job.emit(
                        "scoutiq://fm-engine-done",
                        serde_json::json!({ "ok": false, "error": error }),
                    );
                }
            }
        })
        .map_err(|_| "Unbekannter Parserfehler.".to_string())?;
    Ok(())
}

#[tauri::command]
fn get_calibrating_metadata(save_name: Option<String>) -> CalibratingMetadata {
    calibrating_panel(save_name).fields
}

/// Resolve managed club / competition identity from indexed engine data (no invented names).
#[tauri::command]
async fn analyze_fm_identity(app: AppHandle, path: String) -> Result<FmSaveAnalysisResult, String> {
    if path.trim().is_empty() {
        return Err("FM-Spielstand konnte nicht geöffnet werden.".into());
    }
    tauri::async_runtime::spawn_blocking(move || analyze_save_identity_with_progress(&path, &app))
        .await
        .map_err(|_| "Unbekannter Parserfehler.".to_string())?
}

/// Known-value probe (developer RE only) — arbitrary text, never hardcoded.
#[tauri::command]
async fn probe_fm_container(
    path: String,
    query: String,
    query_type: String,
) -> Result<KnownValueProbeResult, String> {
    if path.trim().is_empty() {
        return Err("FM-Spielstand konnte nicht geöffnet werden.".into());
    }
    tauri::async_runtime::spawn_blocking(move || probe_known_value(&path, &query, &query_type))
        .await
        .map_err(|_| "Unbekannter Parserfehler.".to_string())?
}

#[tauri::command]
async fn compare_fm_saves(path_a: String, path_b: String) -> Result<FmSaveDiff, String> {
    if path_a.trim().is_empty() || path_b.trim().is_empty() {
        return Err("Beide Spielstände müssen gewählt werden.".into());
    }
    tauri::async_runtime::spawn_blocking(move || compare_saves(&path_a, &path_b))
        .await
        .map_err(|_| "Unbekannter Parserfehler.".to_string())?
}

#[tauri::command]
async fn export_fm_diagnostic(
    path: String,
    query: Option<String>,
    query_type: Option<String>,
) -> Result<serde_json::Value, String> {
    if path.trim().is_empty() {
        return Err("FM-Spielstand konnte nicht geöffnet werden.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let container = parse_container(&path)?;
        let probe = match (query, query_type) {
            (Some(q), Some(t)) if !q.trim().is_empty() => Some(probe_known_value(&path, &q, &t)?),
            (Some(q), None) if !q.trim().is_empty() => Some(probe_known_value(&path, &q, "free")?),
            _ => None,
        };
        Ok(export_diagnostic(&container, probe.as_ref()))
    })
    .await
    .map_err(|_| "Unbekannter Parserfehler.".to_string())?
}

#[tauri::command]
fn start_fm_analysis(
    app: AppHandle,
    cancel_state: State<'_, AnalysisCancelState>,
    path: String,
) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("FM-Spielstand konnte nicht geöffnet werden.".into());
    }
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut slot = cancel_state.0.lock().map_err(|e| e.to_string())?;
        if let Some(prev) = slot.as_ref() {
            prev.store(true, Ordering::SeqCst);
        }
        *slot = Some(Arc::clone(&cancel));
    }
    let app_for_job = app.clone();
    std::thread::Builder::new()
        .name("fm-analysis".into())
        .spawn(move || {
            let result = run_fm_analysis(&app_for_job, &path, &cancel);
            emit_done(&app_for_job, result);
        })
        .map_err(|_| "Unbekannter Parserfehler.".to_string())?;
    Ok(())
}

#[tauri::command]
fn cancel_fm_analysis(cancel_state: State<'_, AnalysisCancelState>) -> Result<(), String> {
    let slot = cancel_state.0.lock().map_err(|e| e.to_string())?;
    if let Some(flag) = slot.as_ref() {
        flag.store(true, Ordering::SeqCst);
    }
    Ok(())
}

#[tauri::command]
async fn inspect_fm_save(path: String) -> Result<FmSaveInspection, String> {
    if path.trim().is_empty() {
        return Err("FM-Spielstand konnte nicht geöffnet werden.".into());
    }
    tauri::async_runtime::spawn_blocking(move || fm::inspect_fm_save_path(&path))
        .await
        .map_err(|_| "Unbekannter Parserfehler.".to_string())
}

#[tauri::command]
async fn extract_fm_metadata(path: String) -> Result<FmSaveMetadata, String> {
    if path.trim().is_empty() {
        return Err("FM-Spielstand konnte nicht geöffnet werden.".into());
    }
    tauri::async_runtime::spawn_blocking(move || fm::extract_fm_metadata_path(&path))
        .await
        .map_err(|_| "Unbekannter Parserfehler.".to_string())
}

#[tauri::command]
async fn analyze_fm_save(path: String) -> Result<FmSaveAnalysis, String> {
    if path.trim().is_empty() {
        return Err("FM-Spielstand konnte nicht geöffnet werden.".into());
    }
    tauri::async_runtime::spawn_blocking(move || fm::analyze_fm_save_path(&path))
        .await
        .map_err(|_| "Unbekannter Parserfehler.".to_string())?
}

/// Point the parser at the bundled competition-name catalogue.
///
/// The bundler puts resources somewhere only Tauri knows — `/usr/lib/<app>/…`
/// on Linux, `…app/Contents/Resources` on macOS — so the parser's own
/// exe-relative search would never find it. Resolving it here and exporting
/// `SCOUTIQ_COMPETITION_NAMES` keeps the resolution in one place and leaves the
/// parser free of Tauri types.
///
/// A pre-set variable wins, so a deployment can point at its own file. Failure
/// costs only competition *display names*; every uid still resolves.
fn locate_competition_catalogue(app: &tauri::App) {
    use crate::fm::entities::competition_name::{CATALOGUE_PATH_ENV, CATALOGUE_RELATIVE_PATH};
    use tauri::path::BaseDirectory;
    use tauri::Manager;

    if std::env::var_os(CATALOGUE_PATH_ENV).is_some() {
        return;
    }
    match app
        .path()
        .resolve(CATALOGUE_RELATIVE_PATH, BaseDirectory::Resource)
    {
        Ok(path) if path.is_file() => std::env::set_var(CATALOGUE_PATH_ENV, path),
        _ => {
            // Dev runs fall back to the crate-relative copy inside the parser.
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(RecentMenuState(Mutex::new(Vec::new())))
        .manage(AnalysisCancelState(Mutex::new(None)))
        .setup(|app| {
            locate_competition_catalogue(app);

            let handle = app.handle().clone();
            let menu = build_app_menu(&handle, &[])?;
            app.set_menu(menu)?;

            let handle_for_events = handle.clone();
            app.on_menu_event(move |app, event| {
                let id = event.id().as_ref().to_string();
                if let Some(index_str) = id.strip_prefix("recent::") {
                    if let Ok(index) = index_str.parse::<usize>() {
                        if let Some(state) = app.try_state::<RecentMenuState>() {
                            if let Ok(paths) = state.0.lock() {
                                if let Some(path) = paths.get(index) {
                                    emit_open_recent(&handle_for_events, path);
                                    return;
                                }
                            }
                        }
                    }
                }
                match id.as_str() {
                    "open-fm-save" | "save-project" | "save-project-as" | "view-reload" => {
                        emit_menu_action(&handle_for_events, &id);
                    }
                    "help-about" => {
                        handle_for_events
                            .dialog()
                            .message("ScoutIQ — Football Manager Assistant\nVersion 0.1.0")
                            .title("Über ScoutIQ")
                            .buttons(MessageDialogButtons::Ok)
                            .show(|_| {});
                    }
                    _ => {}
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            update_recent_menu,
            validate_fm_save,
            parse_fm_container,
            start_fm_engine_analysis,
            get_calibrating_metadata,
            analyze_fm_identity,
            probe_fm_container,
            compare_fm_saves,
            export_fm_diagnostic,
            start_fm_analysis,
            cancel_fm_analysis,
            inspect_fm_save,
            extract_fm_metadata,
            analyze_fm_save
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
