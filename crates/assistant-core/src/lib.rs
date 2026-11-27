use anyhow::{bail, Context, Result};
mod app_layout;
pub mod downloads;
pub mod installation;
mod ion_asar;
pub mod localization;
pub mod macos;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub schema_version: u32,
    pub dark: bool,
    pub auto: bool,
    pub rebuild: bool,
    pub localized: bool,
    pub source_path: Option<PathBuf>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            dark: false,
            auto: true,
            rebuild: true,
            localized: false,
            source_path: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Installation {
    pub path: PathBuf,
    pub version: String,
    pub architecture: String,
    pub kind: String,
    pub layout: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub settings: Settings,
    pub installations: Vec<Installation>,
    pub selected: Option<Installation>,
    pub running: bool,
    pub warnings: Vec<String>,
    pub data_dir: PathBuf,
    pub platform: String,
}
pub fn data_dir() -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    return Ok(macos::home()?.join("Library/Application Support/ClaudeCnAssistant"));
    #[cfg(not(target_os = "macos"))]
    Ok(
        PathBuf::from(std::env::var_os("LOCALAPPDATA").context("无法获取本地应用数据目录")?)
            .join("ClaudeCnAssistant"),
    )
}
pub fn load_settings(root: &Path) -> Result<Settings> {
    let path = root.join("settings.json");
    if !path.exists() {
        return Ok(Settings::default());
    }
    let settings: Settings = serde_json::from_slice(&fs::read(&path)?)
        .context("设置文件损坏，原文件已保留；请打开数据目录检查 settings.json")?;
    if settings.schema_version != 1 {
        bail!("设置来自不支持的版本，原文件已保留");
    }
    Ok(settings)
}
pub fn save_settings(root: &Path, settings: &Settings) -> Result<()> {
    if settings.schema_version != 1 {
        bail!("设置版本不受支持");
    }
    if let Some(path) = &settings.source_path {
        validate_source(path)?;
    }
    fs::create_dir_all(root)?;
    let mut temp = tempfile::NamedTempFile::new_in(root)?;
    use std::io::Write;
    temp.write_all(&serde_json::to_vec_pretty(settings)?)?;
    temp.as_file().sync_all()?;
    let target = root.join("settings.json");
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let from: Vec<u16> = temp
            .path()
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let to: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
        let ok = unsafe {
            windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                from.as_ptr(),
                to.as_ptr(),
                windows_sys::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING
                    | windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    #[cfg(not(windows))]
    temp.persist(target).map_err(|e| e.error)?;
    Ok(())
}
pub fn validate_source(path: &Path) -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    return macos::validate_bundle(path);
    #[cfg(not(target_os = "macos"))]
    {
        if path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| !s.eq_ignore_ascii_case("Claude.exe"))
            .unwrap_or(true)
        {
            bail!("请选择安装目录内真实的 Claude.exe，不是命令行别名或快捷方式");
        }
        if path
            .to_string_lossy()
            .to_lowercase()
            .contains("\\microsoft\\windowsapps\\")
        {
            bail!("WindowsApps 执行别名不是实际安装文件");
        }
        let canonical = path
            .canonicalize()
            .context("无法访问 Claude.exe，请确认路径和读取权限")?;
        if !canonical.is_file() || !canonical.parent().unwrap().join("resources").is_dir() {
            bail!("所选目录缺少 resources，不是完整的 Claude Desktop 安装");
        }
        Ok(canonical)
    }
}
fn hidden(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command.stdin(Stdio::null()).stderr(Stdio::null());
}
pub(crate) fn powershell(script: &str, source: Option<&Path>) -> Result<String> {
    let mut output = tempfile::tempfile()?;
    let mut cmd = Command::new("powershell.exe");
    // 桌面应用可能由 PowerShell 7 启动，其 PSModulePath 会污染 Windows PowerShell 5。
    // 系统检测仅需要 Windows 自带模块。
    if let Some(windows) = std::env::var_os("SystemRoot") {
        cmd.env(
            "PSModulePath",
            PathBuf::from(windows).join("System32/WindowsPowerShell/v1.0/Modules"),
        );
    }
    hidden(&mut cmd);
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script])
        .stdout(output.try_clone()?);
    if let Some(path) = source {
        cmd.env("CLAUDE_CN_PROBE_PATH", path);
    }
    let mut child = cmd.spawn().context("无法启动系统检测程序")?;
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(status) = child.try_wait()? {
            if !status.success() {
                bail!("系统检测失败，可能没有读取安装信息的权限");
            }
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("系统检测超时，请重新检测");
        }
        std::thread::sleep(Duration::from_millis(40));
    }
    use std::io::{Read, Seek, SeekFrom};
    output.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    output.take(1024 * 1024).read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes)
        .trim_start_matches('\u{feff}')
        .trim()
        .to_string())
}
#[cfg(not(target_os = "macos"))]
const PROBE: &str = r#"
$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$paths=[Collections.Generic.List[string]]::new(); $warnings=[Collections.Generic.List[string]]::new()
if($env:CLAUDE_CN_PROBE_PATH){ $paths.Add($env:CLAUDE_CN_PROBE_PATH) }
foreach($root in @("$env:LOCALAPPDATA\AnthropicClaude","$env:LOCALAPPDATA\Programs\Claude","$env:LOCALAPPDATA\Claude","$env:ProgramFiles\Claude")) {
 if(Test-Path -LiteralPath $root){
  $paths.Add((Join-Path $root 'Claude.exe'))
  Get-ChildItem -LiteralPath $root -Directory -Filter 'app-*' -ErrorAction SilentlyContinue | ForEach-Object { $paths.Add((Join-Path $_.FullName 'Claude.exe')) }
 }
}
try { Get-AppxPackage -Name '*Claude*' -ErrorAction Stop | ForEach-Object { if($_.InstallLocation){ $paths.Add((Join-Path $_.InstallLocation 'app\Claude.exe')); $paths.Add((Join-Path $_.InstallLocation 'Claude.exe')) } } } catch { $warnings.Add('无法枚举商店安装，请手动选择 Claude.exe') }
$items=@(foreach($p in ($paths | Select-Object -Unique)) {
 if((Test-Path -LiteralPath $p -PathType Leaf) -and (Test-Path -LiteralPath (Join-Path (Split-Path $p) 'resources') -PathType Container)) {
  $f=Get-Item -LiteralPath $p
  if($f.Attributes -band [IO.FileAttributes]::ReparsePoint){ continue }
  $version=$f.VersionInfo.ProductVersion; if(-not $version){ $version=$f.VersionInfo.FileVersion }
  @{path=$p; version="$version"; kind=$(if($p -match '\\WindowsApps\\'){'msix'}else{'standard'})}
 }
})
$running=@(Get-Process -Name 'Claude' -ErrorAction SilentlyContinue).Count -gt 0
@{items=$items; running=$running; warnings=@($warnings)} | ConvertTo-Json -Depth 5 -Compress
"#;
#[cfg(not(target_os = "macos"))]
#[derive(Deserialize)]
struct Probe {
    items: Vec<ProbeItem>,
    running: bool,
    warnings: Vec<String>,
}
#[cfg(not(target_os = "macos"))]
#[derive(Deserialize)]
struct ProbeItem {
    path: PathBuf,
    version: String,
    kind: String,
}
#[cfg(any(not(target_os = "macos"), test))]
fn architecture(path: &Path) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let parse = || -> Option<String> {
        let mut f = fs::File::open(path).ok()?;
        let mut dos = [0u8; 64];
        f.read_exact(&mut dos).ok()?;
        if &dos[..2] != b"MZ" {
            return None;
        }
        let offset = u32::from_le_bytes(dos[60..64].try_into().ok()?);
        f.seek(SeekFrom::Start(offset as u64)).ok()?;
        let mut pe = [0u8; 6];
        f.read_exact(&mut pe).ok()?;
        if &pe[..4] != b"PE\0\0" {
            return None;
        }
        Some(
            match u16::from_le_bytes([pe[4], pe[5]]) {
                0x8664 => "x64",
                0xaa64 => "arm64",
                0x14c => "x86",
                _ => "unknown",
            }
            .into(),
        )
    };
    parse().unwrap_or_else(|| "unknown".into())
}
pub fn snapshot(root: &Path) -> Result<Snapshot> {
    let settings = load_settings(root)?;
    #[cfg(target_os = "macos")]
    return macos::snapshot(root, settings);
    #[cfg(not(target_os = "macos"))]
    {
        let probe: Probe =
            serde_json::from_str(&powershell(PROBE, settings.source_path.as_deref())?)
                .context("系统检测返回了无法识别的数据")?;
        let mut warnings = probe.warnings;
        let installations: Vec<Installation> = probe
            .items
            .into_iter()
            .filter_map(|i| {
                if validate_source(&i.path).is_err() {
                    return None;
                }
                let resources = i.path.parent()?.join("resources");
                let layout = if resources.join("ion-dist.asar").is_file() {
                    "asar"
                } else if resources.join("ion-dist").is_dir() {
                    "directory"
                } else {
                    "unknown"
                };
                Some(Installation {
                    architecture: architecture(&i.path),
                    path: i.path,
                    version: i.version,
                    kind: i.kind,
                    layout: layout.into(),
                })
            })
            .collect();
        let selected = if let Some(path) = &settings.source_path {
            let selected = installations
                .iter()
                .find(|i| {
                    i.path
                        .to_string_lossy()
                        .eq_ignore_ascii_case(&path.to_string_lossy())
                })
                .cloned();
            if selected.is_none() {
                warnings.push("已选择的安装不可用，请重新选择 Claude.exe".into());
            }
            selected
        } else if installations.len() == 1 {
            installations.first().cloned()
        } else {
            None
        };
        Ok(Snapshot {
            settings,
            installations,
            selected,
            running: probe.running,
            warnings,
            data_dir: root.to_path_buf(),
            platform: std::env::consts::OS.into(),
        })
    }
}
pub fn launch_official(root: &Path) -> Result<()> {
    let snapshot = snapshot(root)?;
    let installation = snapshot.selected.context("请先选择一个有效安装")?;
    let exe = validate_source(&installation.path)?;
    #[cfg(target_os = "macos")]
    {
        macos::run("/usr/bin/open", &["-n".as_ref(), exe.as_os_str()])?;
        return Ok(());
    }
    #[cfg(not(target_os = "macos"))]
    {
        let mut command = Command::new(&exe);
        hidden(&mut command);
        command
            .current_dir(exe.parent().context("安装目录无效")?)
            .spawn()
            .context("无法启动官方 Claude")?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn future_schema_is_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        fs::write(&file, br#"{"schemaVersion":99}"#).unwrap();
        assert!(load_settings(dir.path()).is_err());
        assert_eq!(fs::read(&file).unwrap(), br#"{"schemaVersion":99}"#);
    }
    #[test]
    fn pe_architecture_is_read_from_header() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("Claude.exe");
        let mut bytes = vec![0u8; 134];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60..64].copy_from_slice(&128u32.to_le_bytes());
        bytes[128..132].copy_from_slice(b"PE\0\0");
        bytes[132..134].copy_from_slice(&0xaa64u16.to_le_bytes());
        fs::write(&file, bytes).unwrap();
        assert_eq!(architecture(&file), "arm64");
        fs::write(&file, b"MZ").unwrap();
        assert_eq!(architecture(&file), "unknown");
    }
    #[test]
    fn settings_roundtrip_and_replacement() {
        let dir = tempfile::tempdir().unwrap();
        let mut settings = Settings::default();
        save_settings(dir.path(), &settings).unwrap();
        settings.dark = true;
        save_settings(dir.path(), &settings).unwrap();
        assert_eq!(load_settings(dir.path()).unwrap(), settings);
    }
    #[test]
    fn corrupt_settings_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("settings.json"), b"broken").unwrap();
        assert!(load_settings(dir.path()).is_err());
        assert_eq!(
            fs::read(dir.path().join("settings.json")).unwrap(),
            b"broken"
        );
    }
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn rejects_alias_and_incomplete_install() {
        assert!(validate_source(Path::new(
            "C:\\Users\\test\\AppData\\Local\\Microsoft\\WindowsApps\\Claude.exe"
        ))
        .is_err());
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("Claude.exe");
        fs::write(&exe, b"MZ").unwrap();
        assert!(validate_source(&exe).is_err());
        fs::create_dir(dir.path().join("resources")).unwrap();
        assert!(validate_source(&exe).is_ok());
    }
}
