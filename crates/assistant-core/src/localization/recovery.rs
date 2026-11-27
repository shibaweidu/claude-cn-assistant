//! 可恢复目录切换：始终保留原目录；不自动删除备份或用户配置。
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEntry {
    pub id: String,
    pub version: String,
    pub valid: bool,
    pub message: String,
    pub bytes: u64,
    pub modified: u64,
    pub kind: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryState {
    pub entries: Vec<BackupEntry>,
    pub pending: bool,
    pub transaction_message: String,
    pub current_valid: bool,
    pub current_message: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Transaction {
    source: String,
    backup: String,
    phase: String,
    had_current: bool,
}
fn atomic(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut file = tempfile::NamedTempFile::new_in(path.parent().context("事务目录缺失")?)?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| e.error)?;
    Ok(())
}
fn no_links(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path)?;
    ensure!(!meta.file_type().is_symlink(), "不允许符号链接");
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        ensure!(meta.file_attributes() & 0x400 == 0, "不允许重解析点");
    }
    Ok(())
}
fn valid_uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value)
}
fn transaction_paths(root: &Path, tx: &Transaction) -> Result<(PathBuf, PathBuf)> {
    let valid_source = tx.source.strip_prefix("staging-").is_some_and(valid_uuid)
        || tx.source.strip_prefix("backups/").is_some_and(valid_uuid);
    ensure!(
        valid_source
            && tx.backup.strip_prefix("backups/").is_some_and(valid_uuid)
            && tx.source != tx.backup,
        "切换记录路径无效，未执行文件操作"
    );
    Ok((root.join(&tx.source), root.join(&tx.backup)))
}
impl Localizer {
    pub(super) fn refresh_runtime(&self) -> Result<()> {
        let dir = self.root.join("managed");
        no_links(&dir)?;
        let mut files = Vec::new();
        collect(&dir, &dir, &mut files)?;
        let layout = CopyLayout::at(&dir);
        let receipt: LocalState = serde_json::from_slice(&fs::read(dir.join("cn-receipt.json"))?)?;
        if layout.bundle.is_some() {
            ensure!(
                !receipt.fuse_hash.is_empty() && digest(&layout.fuse)? == receipt.fuse_hash,
                "Electron Framework 已变化，不能更新启动脚本"
            );
        }
        ensure!(
            receipt.patch_version == 1
                && digest(&layout.exe.clone())? == receipt.exe_hash
                && digest(&layout.resources.join("app.asar"))? == receipt.app_hash
                && digest(&layout.resources.join("ion-dist.asar"))? == receipt.ion_hash,
            "副本关键文件变化，不能更新启动脚本"
        );
        for (name, bytes) in SCRIPTS {
            let mut temp = tempfile::NamedTempFile::new_in(dir.join("cn-runtime"))?;
            temp.write_all(bytes)?;
            temp.as_file().sync_all()?;
            temp.persist(dir.join("cn-runtime").join(name))
                .map_err(|e| e.error)?;
        }
        Ok(())
    }
    fn transaction_path(&self) -> PathBuf {
        self.root.join("localization-transaction.json")
    }
    pub(super) fn ensure_no_transaction(&self) -> Result<()> {
        if !self.transaction_path().exists() {
            return Ok(());
        }
        let tx: Transaction = serde_json::from_slice(&fs::read(self.transaction_path())?)
            .context("事务记录损坏，请保留数据目录进行检查")?;
        ensure!(
            matches!(tx.phase.as_str(), "complete" | "rolledBack"),
            "存在未完成的副本切换，请先到备份与恢复处理"
        );
        Ok(())
    }
    pub(super) fn validate_copy(&self, dir: &Path) -> Result<LocalState> {
        no_links(dir)?;
        ensure!(dir.is_dir(), "副本目录不存在");
        // 校验完整目录树，防止 resources 或 cn-runtime 经链接跳出管理目录。
        let mut files = Vec::new();
        collect(dir, dir, &mut files)?;
        let receipt_path = dir.join("cn-receipt.json");
        ensure!(
            fs::metadata(&receipt_path)?.len() < 1024 * 1024,
            "副本收据过大"
        );
        let layout = CopyLayout::at(dir);
        let mut receipt: LocalState =
            serde_json::from_slice(&fs::read(receipt_path)?).context("缺少有效副本收据")?;
        ensure!(
            receipt.patch_version == 1,
            "此副本补丁版本不兼容，请重新制作"
        );
        ensure!(
            digest(&layout.exe.clone())? == receipt.exe_hash
                && digest(&layout.resources.join("ion-dist.asar"))? == receipt.ion_hash
                && digest(&layout.resources.join("app.asar"))? == receipt.app_hash,
            "副本关键文件校验失败，不可恢复或启动"
        );
        if let Some(bundle) = &layout.bundle {
            ensure!(
                !receipt.fuse_hash.is_empty() && digest(&layout.fuse)? == receipt.fuse_hash,
                "Electron Framework 校验失败"
            );
            #[cfg(target_os = "macos")]
            crate::macos::verify_copy(bundle)?;
            #[cfg(not(target_os = "macos"))]
            let _ = bundle;
        }
        for (name, expected) in SCRIPTS {
            ensure!(
                fs::read(dir.join("cn-runtime").join(name))? == *expected,
                "副本脚本版本不匹配，请重新制作"
            );
        }
        crate::ion_asar::verify_ion_asar(&layout.resources.join("ion-dist.asar"))?;
        receipt.stage = "prepared".into();
        receipt.managed_exe = Some(
            self.root
                .join("managed")
                .join(layout.exe.strip_prefix(dir)?),
        );
        Ok(receipt)
    }
    pub(super) fn switch_directories(&self, source: &Path, backup: &Path) -> Result<()> {
        self.ensure_no_transaction()?;
        let source_rel = source
            .strip_prefix(&self.root)?
            .to_string_lossy()
            .replace('\\', "/");
        let backup_rel = backup
            .strip_prefix(&self.root)?
            .to_string_lossy()
            .replace('\\', "/");
        let mut tx = Transaction {
            source: source_rel,
            backup: backup_rel,
            phase: "planned".into(),
            had_current: self.root.join("managed").exists(),
        };
        transaction_paths(&self.root, &tx)?;
        self.validate_copy(source)?;
        no_links(&self.root)?;
        no_links(&self.root.join("backups"))?;
        ensure!(!backup.exists(), "目标备份已存在");
        let current = self.root.join("managed");
        if current.exists() {
            no_links(&current)?;
        }
        atomic(&self.transaction_path(), &tx)?;
        if tx.had_current {
            fs::rename(&current, backup).context("无法备份旧副本，未替换当前副本")?;
        }
        tx.phase = "oldMoved".into();
        atomic(&self.transaction_path(), &tx)?;
        if let Err(error) = fs::rename(source, &current) {
            if tx.had_current {
                fs::rename(backup, &current)
                    .context("切换失败；旧副本保留在 backups，请处理未完成事务")?;
            }
            tx.phase = "rolledBack".into();
            atomic(&self.transaction_path(), &tx)?;
            return Err(error.into());
        }
        tx.phase = "newMoved".into();
        atomic(&self.transaction_path(), &tx)?;
        let mut state = self.validate_copy(&current)?;
        state.backup = tx.had_current.then(|| backup.to_path_buf());
        state.message = "副本切换已完成，旧副本保留。".into();
        self.save(&state)?;
        *self.state.lock().unwrap() = state;
        tx.phase = "complete".into();
        atomic(&self.transaction_path(), &tx)?;
        Ok(())
    }
    pub fn recovery_state(&self) -> Result<RecoveryState> {
        let _gate = self
            .gate
            .try_lock()
            .map_err(|_| anyhow::anyhow!("汉化或恢复任务正在进行"))?;
        no_links(&self.root)?;
        let mut entries = Vec::new();
        for (kind, base) in [
            ("backup", self.root.join("backups")),
            ("staging", self.root.clone()),
        ] {
            if !base.exists() {
                continue;
            }
            no_links(&base)?;
            for entry in fs::read_dir(base)? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().to_string();
                if kind == "backup" && !valid_uuid(&name) {
                    continue;
                }
                if kind == "staging" && !name.strip_prefix("staging-").is_some_and(valid_uuid) {
                    continue;
                }
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let result = self.validate_copy(&path);
                let (valid, version, message) = match result {
                    Ok(s) => (true, s.source_version, "关键文件和补丁校验通过".into()),
                    Err(e) => (false, String::new(), format!("{e:#}")),
                };
                let mut files = Vec::new();
                let size = if no_links(&path).is_ok() && collect(&path, &path, &mut files).is_ok() {
                    files.iter().map(|(_, n)| n).sum()
                } else {
                    0
                };
                let modified = entry
                    .metadata()?
                    .modified()?
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                entries.push(BackupEntry {
                    id: if kind == "backup" {
                        format!("backups/{name}")
                    } else {
                        name
                    },
                    version,
                    valid,
                    message,
                    bytes: size,
                    modified,
                    kind: kind.into(),
                });
            }
        }
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.modified));
        let pending = self.ensure_no_transaction().is_err();
        let (current_valid, current_message) = match self.validate_copy(&self.root.join("managed"))
        {
            Ok(s) => (true, format!("当前副本 {} 已校验", s.source_version)),
            Err(_) if !self.root.join("managed").exists() => (
                false,
                "尚未制作中文副本。可先到界面汉化页制作，之后会自动保留旧副本。".into(),
            ),
            Err(e) => (false, format!("当前副本不可用：{e:#}")),
        };
        Ok(RecoveryState {
            entries,
            pending,
            transaction_message: if pending {
                "检测到未完成或损坏的切换记录。请先恢复中断流程；不会自动覆盖文件。"
            } else {
                "没有待处理的目录切换。"
            }
            .into(),
            current_valid,
            current_message,
        })
    }
    pub fn restore(&self, id: &str) -> Result<LocalState> {
        let _gate = self
            .gate
            .try_lock()
            .map_err(|_| anyhow::anyhow!("另一项副本操作正在进行"))?;
        self.ensure_no_transaction()?;
        ensure!(
            !crate::installation::environment()?.running,
            "请先正常退出 Claude，再恢复副本"
        );
        let tx = Transaction {
            source: id.into(),
            backup: format!("backups/{}", uuid::Uuid::new_v4()),
            phase: "planned".into(),
            had_current: false,
        };
        let (source, backup) = transaction_paths(&self.root, &tx)?;
        // source 的父级也必须是本工具直接管理的目录。
        no_links(source.parent().context("目录无效")?)?;
        self.validate_copy(&source)?;
        fs::create_dir_all(self.root.join("backups"))?;
        self.switch_directories(&source, &backup)?;
        let mut state = self.validate_copy(&self.root.join("managed"))?;
        state.backup = backup.exists().then_some(backup);
        state.message = "副本已恢复，替换前的当前副本已保留。用户对话和原版安装未回退。".into();
        self.save(&state)?;
        *self.state.lock().unwrap() = state.clone();
        Ok(state)
    }
    pub fn reconcile(&self) -> Result<LocalState> {
        let _gate = self
            .gate
            .try_lock()
            .map_err(|_| anyhow::anyhow!("另一项副本操作正在进行"))?;
        ensure!(
            !crate::installation::environment()?.running,
            "请先正常退出 Claude，再恢复中断流程"
        );
        self.reconcile_files()
    }
    fn reconcile_files(&self) -> Result<LocalState> {
        no_links(&self.root)?;
        let mut tx: Transaction = serde_json::from_slice(&fs::read(self.transaction_path())?)
            .context("切换记录损坏，未移动任何目录；请保留数据目录进行检查")?;
        ensure!(
            !matches!(tx.phase.as_str(), "complete" | "rolledBack"),
            "该切换已处理完毕"
        );
        let (source, backup) = transaction_paths(&self.root, &tx)?;
        let current = self.root.join("managed");
        for parent in [
            source.parent().context("目录无效")?,
            backup.parent().context("目录无效")?,
        ] {
            if parent.exists() {
                no_links(parent)?;
            }
        }
        let mut state = if current.exists() {
            let state = match self.validate_copy(&current) {
                Ok(state) => state,
                Err(error) => {
                    ensure!(
                        tx.had_current && backup.exists(),
                        "当前副本校验失败且没有可恢复备份：{error:#}"
                    );
                    self.validate_copy(&backup)?;
                    let archive = self.root.join("archive");
                    fs::create_dir_all(&archive)?;
                    no_links(&archive)?;
                    no_links(&current)?;
                    fs::rename(
                        &current,
                        archive.join(format!("damaged-{}", uuid::Uuid::new_v4())),
                    )?;
                    fs::rename(&backup, &current)?;
                    tx.phase = "rolledBack".into();
                    let mut state = self.validate_copy(&current)?;
                    state.message = "已保留损坏副本到 archive，并恢复校验通过的旧副本。".into();
                    self.save(&state)?;
                    atomic(&self.transaction_path(), &tx)?;
                    *self.state.lock().unwrap() = state.clone();
                    return Ok(state);
                }
            };
            // 源目录已消失意味着新副本已提交；否则当前仍是原有副本。
            tx.phase = if source.exists() {
                "rolledBack"
            } else {
                "complete"
            }
            .into();
            state
        } else if tx.had_current && backup.exists() {
            self.validate_copy(&backup)?;
            fs::rename(&backup, &current)?;
            tx.phase = "rolledBack".into();
            self.validate_copy(&current)?
        } else if !tx.had_current && source.exists() {
            self.validate_copy(&source)?;
            fs::rename(&source, &current)?;
            tx.phase = "complete".into();
            self.validate_copy(&current)?
        } else {
            anyhow::bail!(
                "无法确定可恢复的副本位置；文件均保留，请检查 managed、backups 与暂存目录"
            )
        };
        state.message = if tx.phase == "complete" {
            "已校验并完成中断的副本切换。"
        } else {
            "已恢复切换前可用副本，未完成的新副本保留。"
        }
        .into();
        state.backup = backup.exists().then_some(backup);
        self.save(&state)?;
        atomic(&self.transaction_path(), &tx)?;
        *self.state.lock().unwrap() = state.clone();
        Ok(state)
    }
    pub fn use_current(&self) -> Result<LocalState> {
        let _gate = self
            .gate
            .try_lock()
            .map_err(|_| anyhow::anyhow!("副本操作正在进行"))?;
        self.ensure_no_transaction()?;
        let mut state = self.validate_copy(&self.root.join("managed"))?;
        state.message = "已重新校验当前副本，恢复其可启动状态。".into();
        self.save(&state)?;
        *self.state.lock().unwrap() = state.clone();
        Ok(state)
    }
    pub fn archive_entry(&self, id: &str) -> Result<()> {
        let _gate = self
            .gate
            .try_lock()
            .map_err(|_| anyhow::anyhow!("副本操作正在进行"))?;
        self.ensure_no_transaction()?;
        ensure!(
            !crate::installation::environment()?.running,
            "请先退出 Claude，再归档旧目录"
        );
        let tx = Transaction {
            source: id.into(),
            backup: format!("backups/{}", uuid::Uuid::new_v4()),
            phase: String::new(),
            had_current: false,
        };
        let (source, _) = transaction_paths(&self.root, &tx)?;
        no_links(&self.root)?;
        no_links(source.parent().context("目录无效")?)?;
        no_links(&source)?;
        let archive = self.root.join("archive");
        fs::create_dir_all(&archive)?;
        no_links(&archive)?;
        let archive_id = uuid::Uuid::new_v4().to_string();
        atomic(
            &archive.join(format!("{archive_id}.json")),
            &serde_json::json!({"originalEntry":id,"archivedDirectory":archive_id}),
        )?;
        fs::rename(source, archive.join(archive_id))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn copy_fixture(root: &Path, dir: &Path, version: &str) {
        let layout = CopyLayout::at(dir);
        fs::create_dir_all(dir.join("resources")).unwrap();
        fs::create_dir_all(dir.join("cn-runtime")).unwrap();
        fs::write(layout.exe.clone(), b"fixture exe").unwrap();
        fs::write(layout.resources.join("app.asar"), b"fixture app").unwrap();
        let js=br#"const langs=["en-US","de-DE","fr-FR","ko-KR","ja-JP","es-419","es-ES","it-IT","hi-IN","pt-BR","id-ID"];"#;
        let ion = layout.resources.join("ion-dist.asar");
        fs::write(
            &ion,
            crate::ion_asar::build_test_asar(&[
                ("assets/app.js", js),
                ("index.html", b"<head></head>"),
            ]),
        )
        .unwrap();
        crate::ion_asar::localize_ion_asar(&ion, CATALOGS).unwrap();
        for (name, bytes) in SCRIPTS {
            fs::write(dir.join("cn-runtime").join(name), bytes).unwrap();
        }
        let state = LocalState {
            patch_version: 1,
            source_version: version.into(),
            managed_exe: Some(root.join("managed/Claude.exe")),
            exe_hash: digest(&layout.exe.clone()).unwrap(),
            app_hash: digest(&layout.resources.join("app.asar")).unwrap(),
            ion_hash: digest(&ion).unwrap(),
            ..Default::default()
        };
        fs::write(
            dir.join("cn-receipt.json"),
            serde_json::to_vec(&state).unwrap(),
        )
        .unwrap();
    }
    #[test]
    fn first_install_interruption_can_commit_verified_staging() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let svc = Localizer::new(root.into()).unwrap();
        fs::create_dir(root.join("backups")).unwrap();
        let source = format!("staging-{}", uuid::Uuid::new_v4());
        copy_fixture(root, &root.join(&source), "1.0");
        atomic(
            &svc.transaction_path(),
            &Transaction {
                source: source.clone(),
                backup: format!("backups/{}", uuid::Uuid::new_v4()),
                phase: "planned".into(),
                had_current: false,
            },
        )
        .unwrap();
        assert_eq!(svc.reconcile_files().unwrap().source_version, "1.0");
        assert!(!root.join(source).exists());
        assert!(svc.ensure_no_transaction().is_ok());
    }
    #[test]
    fn damaged_committed_copy_is_preserved_before_old_backup_restored() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let svc = Localizer::new(root.into()).unwrap();
        fs::create_dir(root.join("backups")).unwrap();
        copy_fixture(root, &root.join("managed"), "2.0");
        fs::write(root.join("managed/Claude.exe"), b"damaged").unwrap();
        let backup = format!("backups/{}", uuid::Uuid::new_v4());
        copy_fixture(root, &root.join(&backup), "1.0");
        atomic(
            &svc.transaction_path(),
            &Transaction {
                source: format!("staging-{}", uuid::Uuid::new_v4()),
                backup,
                phase: "newMoved".into(),
                had_current: true,
            },
        )
        .unwrap();
        assert_eq!(svc.reconcile_files().unwrap().source_version, "1.0");
        let saved = fs::read_dir(root.join("archive"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert_eq!(fs::read(saved.join("Claude.exe")).unwrap(), b"damaged");
    }
    #[test]
    fn listing_separates_valid_backup_from_incomplete_staging() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let svc = Localizer::new(root.into()).unwrap();
        fs::create_dir(root.join("backups")).unwrap();
        let backup = root.join("backups").join(uuid::Uuid::new_v4().to_string());
        copy_fixture(root, &backup, "1.0");
        let staging = root.join(format!("staging-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&staging).unwrap();
        fs::write(staging.join("part"), b"partial").unwrap();
        let state = svc.recovery_state().unwrap();
        assert_eq!(state.entries.len(), 2);
        assert_eq!(state.entries.iter().filter(|e| e.valid).count(), 1);
        assert!(state
            .entries
            .iter()
            .any(|e| e.kind == "staging" && !e.valid));
    }
    #[test]
    fn switch_preserves_old_copy_and_rejects_damage() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let svc = Localizer::new(root.into()).unwrap();
        fs::create_dir(root.join("backups")).unwrap();
        copy_fixture(root, &root.join("managed"), "1.0");
        let source = root.join(format!("staging-{}", uuid::Uuid::new_v4()));
        copy_fixture(root, &source, "2.0");
        let backup = root.join("backups").join(uuid::Uuid::new_v4().to_string());
        svc.switch_directories(&source, &backup).unwrap();
        assert_eq!(svc.validate_copy(&backup).unwrap().source_version, "1.0");
        assert_eq!(svc.use_current().unwrap().source_version, "2.0");
        fs::write(root.join("managed/Claude.exe"), b"bad").unwrap();
        assert!(svc.use_current().is_err());
        assert!(svc.validate_copy(&backup).is_ok());
    }
    #[test]
    fn interruption_after_old_move_restores_old_copy() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let svc = Localizer::new(root.into()).unwrap();
        fs::create_dir(root.join("backups")).unwrap();
        let backup = format!("backups/{}", uuid::Uuid::new_v4());
        copy_fixture(root, &root.join(&backup), "1.0");
        let source = format!("staging-{}", uuid::Uuid::new_v4());
        copy_fixture(root, &root.join(&source), "2.0");
        atomic(
            &svc.transaction_path(),
            &Transaction {
                source: source.clone(),
                backup,
                phase: "oldMoved".into(),
                had_current: true,
            },
        )
        .unwrap();
        assert!(svc.ensure_no_transaction().is_err());
        assert_eq!(svc.reconcile_files().unwrap().source_version, "1.0");
        assert!(root.join(source).exists());
        assert!(svc.ensure_no_transaction().is_ok());
    }
    #[test]
    fn interruption_after_new_move_accepts_verified_current() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let svc = Localizer::new(root.into()).unwrap();
        fs::create_dir(root.join("backups")).unwrap();
        copy_fixture(root, &root.join("managed"), "2.0");
        let backup = format!("backups/{}", uuid::Uuid::new_v4());
        copy_fixture(root, &root.join(&backup), "1.0");
        atomic(
            &svc.transaction_path(),
            &Transaction {
                source: format!("staging-{}", uuid::Uuid::new_v4()),
                backup,
                phase: "oldMoved".into(),
                had_current: true,
            },
        )
        .unwrap();
        assert_eq!(svc.reconcile_files().unwrap().source_version, "2.0");
        assert!(svc.ensure_no_transaction().is_ok());
    }
    #[test]
    fn unsafe_transaction_and_corrupt_receipt_never_move_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let svc = Localizer::new(root.into()).unwrap();
        copy_fixture(root, &root.join("managed"), "1.0");
        atomic(
            &svc.transaction_path(),
            &Transaction {
                source: "../outside".into(),
                backup: "backups/../../outside".into(),
                phase: "planned".into(),
                had_current: true,
            },
        )
        .unwrap();
        assert!(svc.reconcile_files().is_err());
        assert!(root.join("managed/Claude.exe").exists());
        fs::write(root.join("managed/cn-receipt.json"), b"bad").unwrap();
        assert!(svc.validate_copy(&root.join("managed")).is_err());
    }
}
