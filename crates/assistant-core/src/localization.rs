use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};
const CATALOGS: &[(&str, &[u8])] = &[
    (
        "zh-Hans.json",
        include_bytes!("../../../assets/localization/catalogs/zh-Hans.json"),
    ),
    (
        "dynamic/zh-Hans.json",
        include_bytes!("../../../assets/localization/catalogs/dynamic/zh-Hans.json"),
    ),
    (
        "zh-Hans.overrides.json",
        include_bytes!("../../../assets/localization/catalogs/zh-Hans.overrides.json"),
    ),
];
const SCRIPTS: &[(&str, &[u8])] = &[
    (
        "hook.cjs",
        include_bytes!("../../../assets/localization/hook.cjs"),
    ),
    (
        "launch.cjs",
        include_bytes!("../../../assets/localization/launch.cjs"),
    ),
    (
        "page-shim.js",
        include_bytes!("../../../assets/localization/page-shim.js"),
    ),
];
const SENTINEL: &[u8] = b"dL7pKGdnNz796PbbjQWNKmHXBZaB9tsX";
mod recovery;
pub use recovery::{BackupEntry, RecoveryState};
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocalState {
    pub stage: String,
    pub message: String,
    pub copied: u64,
    pub total: u64,
    pub source_version: String,
    pub source_path: Option<PathBuf>,
    pub managed_exe: Option<PathBuf>,
    pub backup: Option<PathBuf>,
    pub app_hash: String,
    pub exe_hash: String,
    pub ion_hash: String,
    pub patch_version: u32,
}
impl LocalState {
    pub fn active(&self) -> bool {
        matches!(
            self.stage.as_str(),
            "copying" | "patching" | "verifying" | "committing" | "launching"
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeState {
    pub managed_running: bool,
    pub original_running: bool,
    pub unknown_running: bool,
}
fn classify_processes(paths: &[String], managed: &Path) -> RuntimeState {
    let normalize = |value: &str| {
        value
            .replace('/', "\\")
            .trim_start_matches("\\\\?\\")
            .to_lowercase()
    };
    let expected = normalize(&managed.to_string_lossy());
    let mut state = RuntimeState::default();
    for path in paths {
        if path.is_empty() {
            state.unknown_running = true;
        } else if normalize(path) == expected {
            state.managed_running = true;
        } else {
            state.original_running = true;
        }
    }
    state
}
pub struct Localizer {
    root: PathBuf,
    state: Mutex<LocalState>,
    gate: Mutex<()>,
    cancel: AtomicBool,
}
fn digest(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut bytes = vec![0; 1024 * 1024];
    loop {
        let n = file.read(&mut bytes)?;
        if n == 0 {
            break;
        }
        hash.update(&bytes[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn flags(bytes: &[u8]) -> Result<Vec<usize>> {
    let mut tables = Vec::new();
    for (offset, window) in bytes.windows(SENTINEL.len()).enumerate() {
        if window != SENTINEL {
            continue;
        }
        let at = offset + SENTINEL.len();
        let count = *bytes.get(at + 1).context("Electron fuse 截断")? as usize;
        ensure!(
            bytes.get(at) == Some(&1) && count > 4 && count <= 32,
            "不支持的 Electron fuse 格式"
        );
        let values = bytes
            .get(at + 2..at + 2 + count)
            .context("Electron fuse 截断")?;
        ensure!(
            values.iter().all(|b| matches!(b, b'0' | b'1' | b'r'))
                && matches!(values[3], b'0' | b'1')
                && matches!(values[4], b'0' | b'1'),
            "不支持的 Electron fuse 值"
        );
        tables.push(at + 2);
    }
    ensure!(
        !tables.is_empty(),
        "找不到 Electron fuse，不会强行修改该版本"
    );
    Ok(tables)
}
fn patch_exe(path: &Path) -> Result<()> {
    let mut bytes = fs::read(path)?;
    for at in flags(&bytes)? {
        bytes[at + 3] = b'1';
        bytes[at + 4] = b'0';
    }
    fs::write(path, bytes)?;
    Ok(())
}
fn collect(root: &Path, base: &Path, out: &mut Vec<(PathBuf, u64)>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let meta = fs::symlink_metadata(&path)?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            ensure!(
                meta.file_attributes() & 0x400 == 0,
                "源目录包含重解析点，无法安全复制：{}",
                path.display()
            );
        }
        ensure!(
            !meta.file_type().is_symlink(),
            "源目录含符号链接，无法安全复制"
        );
        if meta.is_dir() {
            collect(&path, base, out)?;
        } else if meta.is_file() {
            out.push((path.strip_prefix(base)?.to_path_buf(), meta.len()));
        }
    }
    Ok(())
}
pub fn probe_local_resources() -> Result<()> {
    let snapshot = crate::snapshot(&crate::data_dir()?)?;
    let source = snapshot.selected.context("没有选中的原版安装")?;
    let ion = source
        .path
        .parent()
        .context("源目录无效")?
        .join("resources/ion-dist.asar");
    let before = digest(&ion)?;
    let dir = tempfile::tempdir()?;
    let target = dir.path().join("ion-dist.asar");
    fs::copy(&ion, &target)?;
    crate::ion_asar::localize_ion_asar(&target, CATALOGS)?;
    crate::ion_asar::verify_ion_asar(&target)?;
    ensure!(digest(&ion)? == before, "源资源发生变化");
    println!(
        "本地资源副本汉化与校验通过，来源版本 {}；原资源 SHA256 未变化。",
        source.version
    );
    Ok(())
}
impl Localizer {
    pub fn runtime_state(&self) -> Result<RuntimeState> {
        let output = crate::powershell(
            r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$paths = @(Get-Process -ErrorAction Stop | Where-Object { $_.ProcessName -eq 'Claude' } | ForEach-Object {
    try { if ($_.Path) { [string]$_.Path } else { '' } } catch { '' }
})
ConvertTo-Json -InputObject $paths -Compress
"#,
            None,
        )?;
        let paths: Vec<String> =
            serde_json::from_str(output.trim()).context("无法读取 Claude 运行状态")?;
        Ok(classify_processes(
            &paths,
            &self.root.join("managed/Claude.exe"),
        ))
    }

    pub fn new(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root)?;
        let mut state = fs::read(root.join("localization-state.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<LocalState>(&b).ok())
            .unwrap_or_default();
        if state.active() {
            state.stage = "interrupted".into();
            state.message =
                "上次汉化任务中断，暂存目录保留；请重新制作副本。旧副本和备份不会自动删除。".into();
        }
        Ok(Self {
            root,
            state: Mutex::new(state),
            gate: Mutex::new(()),
            cancel: AtomicBool::new(false),
        })
    }
    pub fn state(&self) -> LocalState {
        self.state.lock().unwrap().clone()
    }
    fn publish(&self, state: &LocalState, notify: &impl Fn(LocalState)) {
        *self.state.lock().unwrap() = state.clone();
        notify(state.clone());
    }
    fn save(&self, state: &LocalState) -> Result<()> {
        let mut file = tempfile::NamedTempFile::new_in(&self.root)?;
        file.write_all(&serde_json::to_vec_pretty(state)?)?;
        file.as_file().sync_all()?;
        file.persist(self.root.join("localization-state.json"))
            .map_err(|e| e.error)?;
        Ok(())
    }
    pub fn cancel(&self) -> Result<()> {
        ensure!(
            self.state().stage == "copying",
            "当前阶段不可取消，请等待校验或替换完成"
        );
        self.cancel.store(true, Ordering::Release);
        Ok(())
    }
    pub fn prepare(&self, force_exit: bool, notify: impl Fn(LocalState)) -> Result<LocalState> {
        let _gate = self
            .gate
            .try_lock()
            .map_err(|_| anyhow::anyhow!("另一项汉化操作正在进行"))?;
        self.cancel.store(false, Ordering::Release);
        self.ensure_no_transaction()?;
        let snapshot = crate::snapshot(&self.root)?;
        let source = snapshot.selected.context("请先选择一个有效的原版安装")?;
        let exe = crate::validate_source(&source.path)?;
        let source_root = exe.parent().context("源目录无效")?;
        let managed = self.root.join("managed");
        ensure!(
            !source_root.starts_with(&self.root.canonicalize()?),
            "不能把受管副本作为原版来源"
        );
        ensure!(
            !snapshot.running || force_exit,
            "Claude 正在运行，请确认强制退出并汉化，或正常退出后重试"
        );
        if force_exit {
            crate::installation::force_exit_claude()?;
        }
        ensure!(
            !crate::installation::environment()?.running,
            "Claude 仍在运行，请退出后重试"
        );
        let mut state = LocalState {
            stage: "copying".into(),
            message: "正在复制原版客户端到独立暂存目录…".into(),
            source_version: source.version,
            source_path: Some(source.path),
            patch_version: 1,
            ..Default::default()
        };
        let staging = self.root.join(format!("staging-{}", uuid::Uuid::new_v4()));
        let result = (|| {
            let source_app = source_root.join("resources/app.asar");
            let source_hash = digest(&source_app)?;
            let source_exe_hash = digest(&exe)?;
            let source_ion_hash = digest(&source_root.join("resources/ion-dist.asar"))?;
            let mut files = Vec::new();
            collect(source_root, source_root, &mut files)?;
            state.total = files.iter().map(|(_, n)| n).sum();
            self.save(&state)?;
            self.publish(&state, &notify);
            fs::create_dir(&staging)?;
            let mut buffer = vec![0; 1024 * 1024];
            let mut last = Instant::now();
            for (relative, _) in files {
                ensure!(
                    !self.cancel.load(Ordering::Acquire),
                    "副本制作已取消；暂存文件保留，原版与旧副本未更改"
                );
                let target = staging.join(&relative);
                fs::create_dir_all(target.parent().context("目标目录无效")?)?;
                let mut input = fs::File::open(source_root.join(&relative))?;
                let mut output = fs::File::create(target)?;
                loop {
                    ensure!(!self.cancel.load(Ordering::Acquire), "副本制作已取消");
                    let n = input.read(&mut buffer)?;
                    if n == 0 {
                        break;
                    }
                    output.write_all(&buffer[..n])?;
                    state.copied += n as u64;
                    if last.elapsed() > Duration::from_millis(150) {
                        self.publish(&state, &notify);
                        last = Instant::now();
                    }
                }
            }
            state.stage = "patching".into();
            state.message = "正在应用中文词表和本地界面补丁…".into();
            self.save(&state)?;
            self.publish(&state, &notify);
            let ion = staging.join("resources/ion-dist.asar");
            ensure!(
                ion.is_file(),
                "当前仅支持 ion-dist.asar 布局；目录布局暂不修改"
            );
            ensure!(
                digest(&staging.join("Claude.exe"))? == source_exe_hash
                    && digest(&ion)? == source_ion_hash,
                "源客户端在复制中更新或文件不完整，请重试"
            );
            crate::ion_asar::localize_ion_asar(&ion, CATALOGS)?;
            patch_exe(&staging.join("Claude.exe"))?;
            let runtime = staging.join("cn-runtime");
            fs::create_dir(&runtime)?;
            for (name, bytes) in SCRIPTS {
                fs::write(runtime.join(name), bytes)?;
            }
            state.stage = "verifying".into();
            state.message = "正在校验副本与未修改的 app.asar…".into();
            self.publish(&state, &notify);
            crate::ion_asar::verify_ion_asar(&ion)?;
            ensure!(
                digest(&source_app)? == source_hash
                    && digest(&staging.join("resources/app.asar"))? == source_hash,
                "源版本在复制中变化或 app.asar 不一致，请重新制作"
            );
            ensure!(
                digest(&exe)? == source_exe_hash
                    && digest(&source_root.join("resources/ion-dist.asar"))? == source_ion_hash,
                "源客户端在汉化期间发生更新，请重新制作"
            );
            state.app_hash = source_hash;
            state.exe_hash = digest(&staging.join("Claude.exe"))?;
            state.ion_hash = digest(&ion)?;
            ensure!(
                !crate::installation::environment()?.running,
                "Claude 在制作期间启动，请退出后重新制作"
            );
            state.stage = "committing".into();
            state.message = "校验通过，正在切换受管副本…".into();
            state.managed_exe = Some(managed.join("Claude.exe"));
            fs::create_dir_all(self.root.join("backups"))?;
            let backup = self
                .root
                .join("backups")
                .join(uuid::Uuid::new_v4().to_string());
            if managed.exists() {
                state.backup = Some(backup.clone());
            }
            let mut receipt = state.clone();
            receipt.stage = "prepared".into();
            fs::write(
                staging.join("cn-receipt.json"),
                serde_json::to_vec_pretty(&receipt)?,
            )?;
            self.save(&state)?;
            self.publish(&state, &notify);
            self.switch_directories(&staging, &backup)?;
            state.stage = "prepared".into();
            state.message =
                "中文副本已制作并校验。原版安装文件未修改；启动前需退出所有 Claude 进程。".into();
            Ok::<(), anyhow::Error>(())
        })();
        if let Err(error) = result {
            state.stage = if self.cancel.load(Ordering::Acquire) {
                "cancelled"
            } else {
                "failed"
            }
            .into();
            state.message = format!("{error:#}。暂存目录：{}", staging.display());
        }
        self.save(&state)?;
        self.publish(&state, &notify);
        Ok(state)
    }
    pub fn launch(&self) -> Result<LocalState> {
        let _gate = self
            .gate
            .try_lock()
            .map_err(|_| anyhow::anyhow!("汉化操作正在进行"))?;
        self.ensure_no_transaction()?;
        self.refresh_runtime()?;
        let mut state = self.validate_copy(&self.root.join("managed"))?;
        ensure!(
            matches!(state.stage.as_str(), "prepared" | "launched"),
            "请先成功制作中文副本"
        );
        ensure!(
            !crate::installation::environment()?.running,
            "请先正常退出所有 Claude 进程，再启动中文版"
        );
        let exe = self.root.join("managed/Claude.exe");
        ensure!(
            digest(&exe)? == state.exe_hash
                && digest(&self.root.join("managed/resources/ion-dist.asar"))? == state.ion_hash
                && digest(&self.root.join("managed/resources/app.asar"))? == state.app_hash,
            "副本文件已变化，请重新制作"
        );
        let mut command = std::process::Command::new("node");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let runtime = self.root.join("managed/cn-runtime");
        for (name, expected) in SCRIPTS {
            ensure!(
                fs::read(runtime.join(name))? == *expected,
                "启动脚本或页面汉化脚本已变化，请重新制作副本"
            );
        }
        let log = fs::File::create(self.root.join("localization-launch.log"))?;
        command
            .arg(runtime.join("launch.cjs"))
            .env_remove("NODE_OPTIONS")
            .env_remove("NODE_PATH")
            .env_remove("ELECTRON_RUN_AS_NODE")
            .env("CLAUDE_CN_MANAGED_EXE", &exe)
            .stdin(std::process::Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log);
        let mut child = command
            .spawn()
            .context("未找到 Node.js，请安装 Node 22 或更新版本；当前开发版尚未内置运行时")?;
        let deadline = Instant::now() + Duration::from_secs(65);
        let success = loop {
            if let Some(status) = child.try_wait()? {
                break status.success();
            }
            if Instant::now() > deadline {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
            std::thread::sleep(Duration::from_millis(100));
        };
        ensure!(success,"中文启动未确认，请查看数据目录 localization-launch.log；副本可能已启动，请先退出后重试");
        state.stage = "launched".into();
        state.message = "受管副本启动与汉化页面注入已确认。请在窗口中核对中文覆盖情况。".into();
        self.save(&state)?;
        *self.state.lock().unwrap() = state.clone();
        Ok(state)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fuse_patch_changes_only_required_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Claude.exe");
        let mut bytes = b"MZ".to_vec();
        bytes.extend_from_slice(SENTINEL);
        bytes.extend_from_slice(&[1, 9]);
        bytes.extend_from_slice(b"010011011");
        let before = bytes.clone();
        fs::write(&path, bytes).unwrap();
        patch_exe(&path).unwrap();
        let after = fs::read(path).unwrap();
        let changes: Vec<_> = before
            .iter()
            .zip(after.iter())
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, _)| i)
            .collect();
        let at = 2 + SENTINEL.len() + 2;
        assert_eq!(changes, vec![at + 3, at + 4]);
    }
    #[test]
    fn rejects_unknown_fuse_without_modifying_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Claude.exe");
        fs::write(&path, b"unknown").unwrap();
        assert!(patch_exe(&path).is_err());
        assert_eq!(fs::read(path).unwrap(), b"unknown");
    }
    #[test]
    fn embedded_catalogs_parse() {
        for (_, bytes) in CATALOGS {
            serde_json::from_slice::<serde_json::Value>(bytes).unwrap();
        }
    }
}

#[cfg(test)]
mod runtime_tests {
    use super::*;
    #[test]
    fn distinguishes_managed_original_and_unreadable_processes() {
        let managed = Path::new(r"C:\Data\managed\Claude.exe");
        assert_eq!(classify_processes(&[], managed), RuntimeState::default());
        let paths = vec![
            "c:/data/managed/CLAUDE.exe".into(),
            r"C:\Other\Claude.exe".into(),
            String::new(),
        ];
        assert_eq!(
            classify_processes(&paths, managed),
            RuntimeState {
                managed_running: true,
                original_running: true,
                unknown_running: true
            }
        );
        assert!(
            !classify_processes(&[r"C:\Data\managed-old\Claude.exe".into()], managed)
                .managed_running
        );
    }
}
