#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use assistant_core::downloads::{DownloadState, Downloads, Task};
use assistant_core::installation::{InstallState, Installer};
use assistant_core::localization::RecoveryState;
use assistant_core::localization::{LocalState, Localizer};
use assistant_core::{Settings, Snapshot};
use std::sync::{Arc, Mutex};
use tauri::Emitter;
#[tauri::command]
fn configure_download_channel(
    accelerated: bool,
    downloads: tauri::State<'_, Arc<Downloads>>,
) -> Result<DownloadState, String> {
    downloads
        .configure_channel(assistant_core::downloads::ChannelConfig {
            accelerated,
            repository: "https://github.com/shibaweidu/claude-cn-assistant".into(),
        })
        .map_err(err)
}
static SETTINGS_LOCK: Mutex<()> = Mutex::new(());
static INSTALL_GATE: Mutex<()> = Mutex::new(());
#[tauri::command]
async fn get_recovery_state(
    localizer: tauri::State<'_, Arc<Localizer>>,
) -> Result<RecoveryState, String> {
    let service = Arc::clone(localizer.inner());
    tauri::async_runtime::spawn_blocking(move || service.recovery_state().map_err(err))
        .await
        .map_err(err)?
}
#[tauri::command]
async fn restore_localized_backup(
    entry_id: String,
    localizer: tauri::State<'_, Arc<Localizer>>,
    installer: tauri::State<'_, Arc<Installer>>,
) -> Result<LocalState, String> {
    let service = Arc::clone(localizer.inner());
    let installer = Arc::clone(installer.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _gate = INSTALL_GATE
            .try_lock()
            .map_err(|_| "安装或副本任务正在进行".to_string())?;
        if installer.state().active() {
            return Err("请先结束安装等待".into());
        }
        service.restore(&entry_id).map_err(err)
    })
    .await
    .map_err(err)?
}
#[tauri::command]
async fn reconcile_localization(
    localizer: tauri::State<'_, Arc<Localizer>>,
    installer: tauri::State<'_, Arc<Installer>>,
) -> Result<LocalState, String> {
    let service = Arc::clone(localizer.inner());
    let installer = Arc::clone(installer.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _gate = INSTALL_GATE
            .try_lock()
            .map_err(|_| "安装或副本任务正在进行".to_string())?;
        if installer.state().active() {
            return Err("请先结束安装等待".into());
        }
        service.reconcile().map_err(err)
    })
    .await
    .map_err(err)?
}
#[tauri::command]
async fn use_current_localization(
    localizer: tauri::State<'_, Arc<Localizer>>,
) -> Result<LocalState, String> {
    let service = Arc::clone(localizer.inner());
    tauri::async_runtime::spawn_blocking(move || service.use_current().map_err(err))
        .await
        .map_err(err)?
}
#[tauri::command]
async fn archive_localization_entry(
    entry_id: String,
    localizer: tauri::State<'_, Arc<Localizer>>,
    installer: tauri::State<'_, Arc<Installer>>,
) -> Result<(), String> {
    let service = Arc::clone(localizer.inner());
    let installer = Arc::clone(installer.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _gate = INSTALL_GATE
            .try_lock()
            .map_err(|_| "安装或副本任务正在进行".to_string())?;
        if installer.state().active() {
            return Err("请先结束安装等待".into());
        }
        service.archive_entry(&entry_id).map_err(err)
    })
    .await
    .map_err(err)?
}
#[tauri::command]
async fn get_localization_runtime(
    localizer: tauri::State<'_, Arc<Localizer>>,
) -> Result<assistant_core::localization::RuntimeState, String> {
    let service = Arc::clone(localizer.inner());
    tauri::async_runtime::spawn_blocking(move || service.runtime_state().map_err(err))
        .await
        .map_err(err)?
}
#[tauri::command]
fn get_localization_state(localizer: tauri::State<'_, Arc<Localizer>>) -> LocalState {
    localizer.state()
}
#[tauri::command]
async fn prepare_localized_copy(
    force_exit: bool,
    app: tauri::AppHandle,
    localizer: tauri::State<'_, Arc<Localizer>>,
    installer: tauri::State<'_, Arc<Installer>>,
) -> Result<LocalState, String> {
    let localizer = Arc::clone(localizer.inner());
    let installer = Arc::clone(installer.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _gate = INSTALL_GATE
            .try_lock()
            .map_err(|_| "安装或汉化任务正在进行，请稍候".to_string())?;
        if installer.state().active() {
            return Err("请先完成安装或结束等待".into());
        }
        localizer
            .prepare(force_exit, |state| {
                let _ = app.emit("localization-task", state);
            })
            .map_err(err)
    })
    .await
    .map_err(err)?
}
#[tauri::command]
fn cancel_localization(localizer: tauri::State<'_, Arc<Localizer>>) -> Result<(), String> {
    localizer.cancel().map_err(err)
}
#[tauri::command]
async fn launch_localized_copy(
    localizer: tauri::State<'_, Arc<Localizer>>,
    installer: tauri::State<'_, Arc<Installer>>,
) -> Result<LocalState, String> {
    let localizer = Arc::clone(localizer.inner());
    let installer = Arc::clone(installer.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _gate = INSTALL_GATE
            .try_lock()
            .map_err(|_| "安装或汉化任务正在进行，请稍候".to_string())?;
        if installer.state().active() {
            return Err("请先完成安装或结束等待".into());
        }
        localizer.launch().map_err(err)
    })
    .await
    .map_err(err)?
}
#[tauri::command]
fn get_install_state(installer: tauri::State<'_, Arc<Installer>>) -> InstallState {
    installer.state()
}
#[tauri::command]
async fn prepare_install(
    installer: tauri::State<'_, Arc<Installer>>,
    downloads: tauri::State<'_, Arc<Downloads>>,
) -> Result<InstallState, String> {
    let installer = Arc::clone(installer.inner());
    let downloads = Arc::clone(downloads.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _gate = INSTALL_GATE
            .try_lock()
            .map_err(|_| "安装或汉化任务正在进行，请稍候".to_string())?;
        let id = installer.reserve().map_err(err)?;
        installer.prepare(&downloads, &id).map_err(err)
    })
    .await
    .map_err(err)?
}
#[tauri::command]
async fn launch_install(
    install_id: String,
    force_exit: bool,
    installer: tauri::State<'_, Arc<Installer>>,
    downloads: tauri::State<'_, Arc<Downloads>>,
) -> Result<InstallState, String> {
    let installer = Arc::clone(installer.inner());
    let downloads = Arc::clone(downloads.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _gate = INSTALL_GATE
            .try_lock()
            .map_err(|_| "安装或汉化任务正在进行，请稍候".to_string())?;
        installer
            .launch(&downloads, &install_id, force_exit)
            .map_err(err)
    })
    .await
    .map_err(err)?
}
#[tauri::command]
async fn recheck_install(
    installer: tauri::State<'_, Arc<Installer>>,
) -> Result<InstallState, String> {
    let installer = Arc::clone(installer.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _gate = INSTALL_GATE
            .try_lock()
            .map_err(|_| "安装或汉化任务正在进行，请稍候".to_string())?;
        installer.recheck().map_err(err)
    })
    .await
    .map_err(err)?
}
#[tauri::command]
async fn stop_install_wait(
    installer: tauri::State<'_, Arc<Installer>>,
) -> Result<InstallState, String> {
    let installer = Arc::clone(installer.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _gate = INSTALL_GATE
            .try_lock()
            .map_err(|_| "安装或汉化任务正在进行，请稍候".to_string())?;
        installer.stop().map_err(err)
    })
    .await
    .map_err(err)?
}
fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}
#[tauri::command]
fn get_download_state(downloads: tauri::State<'_, Arc<Downloads>>) -> DownloadState {
    downloads.state()
}
#[tauri::command]
async fn check_release(
    manual: bool,
    downloads: tauri::State<'_, Arc<Downloads>>,
) -> Result<DownloadState, String> {
    downloads.check(manual).await.map_err(err)
}
#[tauri::command]
fn start_download(
    platform: String,
    architecture: String,
    app: tauri::AppHandle,
    downloads: tauri::State<'_, Arc<Downloads>>,
    installer: tauri::State<'_, Arc<Installer>>,
) -> Result<Task, String> {
    let _gate = INSTALL_GATE
        .try_lock()
        .map_err(|_| "安装或汉化任务正在进行，请稍候".to_string())?;
    if installer.state().active() {
        return Err("请先完成安装流程或结束等待，再开始新的下载".into());
    }
    let (task, release, asset) = downloads.reserve(&platform, &architecture).map_err(err)?;
    let service = Arc::clone(downloads.inner());
    let result = task.clone();
    tauri::async_runtime::spawn(async move {
        service
            .run(task, release, asset, |task| {
                let _ = app.emit("download-task", task);
            })
            .await;
    });
    Ok(result)
}
#[tauri::command]
fn cancel_download(
    task_id: String,
    downloads: tauri::State<'_, Arc<Downloads>>,
) -> Result<(), String> {
    downloads.cancel(&task_id).map_err(err)
}
#[tauri::command]
async fn verify_download(downloads: tauri::State<'_, Arc<Downloads>>) -> Result<Task, String> {
    let service = Arc::clone(downloads.inner());
    tauri::async_runtime::spawn_blocking(move || service.verify_saved())
        .await
        .map_err(err)?
        .map_err(err)
}
#[tauri::command]
async fn reveal_download(downloads: tauri::State<'_, Arc<Downloads>>) -> Result<(), String> {
    let service = Arc::clone(downloads.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let path = service.ready_path().map_err(err)?;
        #[cfg(target_os = "macos")]
        assistant_core::macos::run("/usr/bin/open", &["-R".as_ref(), path.as_os_str()])
            .map_err(err)?;
        #[cfg(not(target_os = "macos"))]
        std::process::Command::new("explorer.exe")
            .arg("/select,")
            .arg(path)
            .spawn()
            .map_err(err)?;
        Ok(())
    })
    .await
    .map_err(err)?
}
#[tauri::command]
async fn get_app_snapshot() -> Result<Snapshot, String> {
    tauri::async_runtime::spawn_blocking(|| assistant_core::snapshot(&assistant_core::data_dir()?))
        .await
        .map_err(err)?
        .map_err(err)
}
#[tauri::command]
async fn update_settings(settings: Settings) -> Result<Settings, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _lock = SETTINGS_LOCK.lock().map_err(err)?;
        assistant_core::save_settings(&assistant_core::data_dir().map_err(err)?, &settings)
            .map_err(err)?;
        Ok(settings)
    })
    .await
    .map_err(err)?
}
#[tauri::command]
async fn select_source_installation() -> Result<Option<Settings>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        #[cfg(target_os = "macos")]
        let path = rfd::FileDialog::new()
            .set_title("选择 Claude.app")
            .add_filter("Claude 应用", &["app"])
            .pick_file();
        #[cfg(not(target_os = "macos"))]
        let path = rfd::FileDialog::new()
            .set_title("选择 Claude Desktop 安装目录内的 Claude.exe")
            .add_filter("Claude", &["exe"])
            .pick_file();
        let Some(path) = path else { return Ok(None) };
        assistant_core::validate_source(&path).map_err(err)?;
        let _lock = SETTINGS_LOCK.lock().map_err(err)?;
        let root = assistant_core::data_dir().map_err(err)?;
        let mut settings = assistant_core::load_settings(&root).map_err(err)?;
        settings.source_path = Some(path);
        assistant_core::save_settings(&root, &settings).map_err(err)?;
        Ok(Some(settings))
    })
    .await
    .map_err(err)?
}
#[tauri::command]
async fn launch_official() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        assistant_core::launch_official(&assistant_core::data_dir()?)
    })
    .await
    .map_err(err)?
    .map_err(err)
}
#[tauri::command]
fn open_data_directory() -> Result<(), String> {
    let root = assistant_core::data_dir().map_err(err)?;
    std::fs::create_dir_all(&root).map_err(err)?;
    #[cfg(target_os = "macos")]
    assistant_core::macos::run("/usr/bin/open", &[root.as_os_str()]).map_err(err)?;
    #[cfg(not(target_os = "macos"))]
    std::process::Command::new("explorer.exe")
        .arg(root)
        .spawn()
        .map_err(err)?;
    Ok(())
}
#[tauri::command]
fn open_project_link(kind: String) -> Result<(), String> {
    let url = match kind.as_str() {
        "release" => "https://github.com/Wangnov/claude-app-mirror/releases/latest",
        "source" => "https://github.com/Wangnov/claude-app-mirror",
        _ => return Err("链接不在允许列表中".into()),
    };
    #[cfg(target_os = "macos")]
    assistant_core::macos::run("/usr/bin/open", &[url.as_ref()]).map_err(err)?;
    #[cfg(not(target_os = "macos"))]
    std::process::Command::new("explorer.exe")
        .arg(url)
        .spawn()
        .map_err(err)?;
    Ok(())
}
fn main() {
    let downloads = Arc::new(
        Downloads::new(assistant_core::data_dir().expect("无法获取应用数据目录"))
            .expect("无法初始化下载服务"),
    );
    tauri::Builder::default()
        .manage(Arc::new(
            Installer::new(assistant_core::data_dir().expect("无法获取应用目录"))
                .expect("无法初始化安装协调服务"),
        ))
        .manage(downloads)
        .manage(Arc::new(
            Localizer::new(assistant_core::data_dir().expect("应用目录不可用"))
                .expect("无法初始化汉化服务"),
        ))
        .invoke_handler(tauri::generate_handler![
            get_app_snapshot,
            update_settings,
            select_source_installation,
            launch_official,
            open_data_directory,
            open_project_link,
            get_download_state,
            configure_download_channel,
            check_release,
            start_download,
            cancel_download,
            verify_download,
            reveal_download,
            get_install_state,
            prepare_install,
            launch_install,
            recheck_install,
            stop_install_wait,
            get_localization_state,
            get_localization_runtime,
            prepare_localized_copy,
            cancel_localization,
            launch_localized_copy,
            get_recovery_state,
            restore_localized_backup,
            reconcile_localization,
            use_current_localization,
            archive_localization_entry
        ])
        .run(tauri::generate_context!())
        .expect("无法启动桌面助手");
}
