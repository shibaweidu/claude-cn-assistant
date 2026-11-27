//! Verified mirror DMG -> user Applications; old application kept in a sibling backup.
use super::*;
use crate::macos as mac;

struct MountedDmg {
    directory: Option<tempfile::TempDir>,
}
impl MountedDmg {
    fn open(path: &Path) -> Result<Self> {
        let directory = tempfile::Builder::new()
            .prefix("claude-cn-dmg-")
            .tempdir()?;
        let mount = Self {
            directory: Some(directory),
        };
        mac::run(
            "/usr/bin/hdiutil",
            &[
                "attach".as_ref(),
                "-readonly".as_ref(),
                "-nobrowse".as_ref(),
                "-noautoopen".as_ref(),
                "-mountpoint".as_ref(),
                mount.path().as_os_str(),
                path.as_os_str(),
            ],
        )?;
        Ok(mount)
    }
    fn path(&self) -> &Path {
        self.directory.as_ref().unwrap().path()
    }
}
impl Drop for MountedDmg {
    fn drop(&mut self) {
        if let Some(directory) = self.directory.take() {
            if mac::run(
                "/usr/bin/hdiutil",
                &["detach".as_ref(), directory.path().as_os_str()],
            )
            .is_err()
            {
                // Never recursively remove a still-mounted volume.
                let _ = directory.keep();
            }
        }
    }
}
fn package_info(bundle: &Path, version: &str) -> Result<PackageInfo> {
    mac::verify_original(bundle)?;
    mac::tree_files(bundle)?;
    let item = mac::inspect(bundle)?;
    ensure!(
        compare_versions(&item.version, version)?.is_eq(),
        "DMG 应用版本与镜像清单不一致"
    );
    let host = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        other => other,
    };
    ensure!(
        item.architecture == "universal" || item.architecture == host,
        "安装包不支持本机架构"
    );
    Ok(PackageInfo {
        name: mac::BUNDLE_ID.into(),
        publisher: "Anthropic, PBC".into(),
        version: item.version,
        architecture: item.architecture,
        signature_status: "Valid".into(),
        signer_matches: true,
    })
}
fn target() -> Result<PathBuf> {
    Ok(mac::home()?.join("Applications/Claude.app"))
}
fn check_no_downgrade(root: &Path, version: &str) -> Result<()> {
    let snapshot = crate::snapshot(root)?;
    for app in snapshot.installations {
        ensure!(
            !compare_versions(&app.version, version)?.is_gt(),
            "本机已安装更高版本，不能降级"
        );
    }
    Ok(())
}
impl Installer {
    pub(super) fn prepare_macos(&self, downloads: &Downloads, id: &str) -> Result<InstallState> {
        let mut state = self.state();
        ensure!(
            state.id == id && state.stage == "checking",
            "安装状态已变化"
        );
        let result = (|| {
            let task = downloads.verify_saved()?;
            ensure!(
                task.stage == "ready" && task.asset_name == "Claude-mac-universal.dmg",
                "请下载并校验 macOS 镜像 DMG"
            );
            let mounted = MountedDmg::open(task.path.as_deref().context("安装包路径缺失")?)?;
            ensure!(
                downloads.verify_saved()?.stage == "ready",
                "挂载后 DMG 完整性检查失败"
            );
            let info = package_info(&mounted.path().join("Claude.app"), &task.version)?;
            check_no_downgrade(&self.root, &info.version)?;
            state.task_id = task.id;
            state.info = Some(info);
            state.stage = "ready".into();
            state.message = "镜像 SHA256、应用身份、Developer ID 签名与系统信任检查通过。确认后安装到 ~/Applications，旧版保留为备份。".into();
            Ok::<(), anyhow::Error>(())
        })();
        if let Err(error) = result {
            state.stage = "failed".into();
            state.message = format!("{error:#}");
        }
        state.checked_at = now();
        self.set(state)
    }
    pub(super) fn launch_macos(
        &self,
        downloads: &Downloads,
        id: &str,
        force_exit: bool,
    ) -> Result<InstallState> {
        let mut state = self.state();
        ensure!(
            state.id == id && state.stage == "ready",
            "请先完成安装前检查"
        );
        state.stage = "launching".into();
        self.set(state.clone())?;
        let result = (|| {
            let task = downloads.verify_saved()?;
            ensure!(
                task.id == state.task_id && task.stage == "ready",
                "下载任务已变化，请重新检查"
            );
            let mounted = MountedDmg::open(task.path.as_deref().context("安装包路径缺失")?)?;
            let verified = downloads.verify_saved()?;
            ensure!(
                verified.id == task.id && verified.stage == "ready",
                "挂载期间安装包或下载任务变化"
            );
            let bundle = mounted.path().join("Claude.app");
            let info = package_info(&bundle, &task.version)?;
            let expected = state.info.as_ref().context("缺少安装信息")?;
            ensure!(
                info.version == expected.version && info.name == expected.name,
                "确认后安装信息变化"
            );
            check_no_downgrade(&self.root, &info.version)?;
            let destination = target()?;
            if destination.exists() {
                mac::verify_original(&destination)?;
                if compare_versions(&mac::inspect(&destination)?.version, &info.version)?.is_eq() {
                    return self.confirm_macos(&mut state, &destination);
                }
            }
            ensure!(
                mac::processes()?.is_empty() || force_exit,
                "Claude 正在运行，请确认强制退出后安装"
            );
            if force_exit {
                mac::force_exit()?;
            }
            let parent = destination.parent().context("安装目录无效")?;
            fs::create_dir_all(parent)?;
            ensure!(
                !fs::symlink_metadata(parent)?.file_type().is_symlink(),
                "Applications 不能是符号链接"
            );
            let staging = parent.join(format!(".ClaudeCnAssistant-stage-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&staging)?;
            let staged_app = staging.join("Claude.app");
            mac::copy_bundle(&bundle, &staged_app)?;
            package_info(&staged_app, &task.version)?;
            ensure!(
                mac::processes()?.is_empty(),
                "Claude 在复制期间启动，请关闭后重试；暂存应用保留"
            );
            let backup_dir = parent.join(format!(
                ".ClaudeCnAssistant-backup-{}",
                uuid::Uuid::new_v4()
            ));
            fs::create_dir(&backup_dir)?;
            let backup = backup_dir.join("Claude.app");
            state.target_path = Some(destination.clone());
            state.staging_path = Some(staging.clone());
            state.backup_path = Some(backup.clone());
            self.set(state.clone())?; // Journal before either rename.
            let had_current = destination.exists();
            if had_current {
                fs::rename(&destination, &backup).context("无法备份旧版，原应用未替换")?;
            }
            if let Err(error) = fs::rename(&staged_app, &destination) {
                if had_current {
                    fs::rename(&backup, &destination)
                        .context("安装切换失败，旧应用保留在备份目录")?;
                }
                return Err(error.into());
            }
            self.confirm_macos(&mut state, &destination)?;
            Ok::<(), anyhow::Error>(())
        })();
        if let Err(error) = result {
            state.stage = "failed".into();
            state.message = format!("{error:#}");
        }
        state.checked_at = now();
        self.set(state)
    }
    fn confirm_macos(&self, state: &mut InstallState, destination: &Path) -> Result<()> {
        mac::verify_original(destination)?;
        let installed = mac::inspect(destination)?;
        ensure!(
            !compare_versions(
                &installed.version,
                &state.info.as_ref().context("缺少目标版本")?.version
            )?
            .is_lt(),
            "安装后的版本不符"
        );
        let mut settings = crate::load_settings(&self.root)?;
        settings.source_path = Some(destination.into());
        crate::save_settings(&self.root, &settings)?;
        state.stage = "installed".into();
        state.needs_rebuild = self.root.join("managed").exists();
        state.target_path = Some(destination.into());
        state.message = if state.needs_rebuild {
            "macOS 更新已确认，旧原版已保留；请重新制作中文副本以使用新版本。"
        } else {
            "macOS 应用安装与签名校验成功，可制作中文副本。"
        }
        .into();
        Ok(())
    }
    pub(super) fn recheck_macos(&self) -> Result<InstallState> {
        let mut state = self.state();
        ensure!(
            matches!(
                state.stage.as_str(),
                "interrupted" | "unconfirmed" | "stopped" | "awaitingInstaller" | "failed"
            ),
            "当前没有待检测的安装"
        );
        let destination = target()?;
        if destination.exists() {
            if let Err(error) = self.confirm_macos(&mut state, &destination) {
                state.stage = "unconfirmed".into();
                state.message = format!("{error:#}");
            }
        } else if let Some(backup) = &state.backup_path {
            // Only accept the exact managed sibling naming scheme, never arbitrary persisted paths.
            let parent = destination.parent().context("安装目录无效")?;
            let backup_dir = backup.parent().context("备份目录无效")?;
            let valid_name = backup_dir
                .file_name()
                .and_then(|s| s.to_str())
                .and_then(|s| s.strip_prefix(".ClaudeCnAssistant-backup-"))
                .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok());
            ensure!(
                backup_dir.parent() == Some(parent)
                    && backup.file_name().is_some_and(|s| s == "Claude.app")
                    && valid_name,
                "备份路径无效，未移动目录"
            );
            ensure!(mac::processes()?.is_empty(), "请退出 Claude 后恢复中断安装");
            mac::verify_original(backup)?;
            fs::rename(backup, &destination)?;
            state.stage = "stopped".into();
            state.message = "已从中断安装恢复旧版原应用，请重新检查并安装更新。".into();
        } else {
            state.stage = "unconfirmed".into();
            state.message = "尚未检测到安装结果".into();
        }
        state.checked_at = now();
        self.set(state)
    }
}
