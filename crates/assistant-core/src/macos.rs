//! macOS system integration. Only absolute system tool paths and argument arrays.
use crate::{Installation, Settings, Snapshot};
use anyhow::{bail, ensure, Context, Result};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub const BUNDLE_ID: &str = "com.anthropic.claudefordesktop";
const REQUIREMENT: &str = r#"anchor apple generic and identifier "com.anthropic.claudefordesktop" and certificate leaf[subject.O] = "Anthropic, PBC" and certificate leaf[field.1.2.840.113635.100.6.1.13] exists"#;

pub fn run(tool: &str, args: &[&std::ffi::OsStr]) -> Result<Vec<u8>> {
    use std::io::{Read, Seek};
    let mut stdout = tempfile::tempfile()?;
    let mut stderr = tempfile::tempfile()?;
    let mut child = Command::new(tool)
        .args(args)
        .stdin(Stdio::null())
        .stdout(stdout.try_clone()?)
        .stderr(stderr.try_clone()?)
        .spawn()
        .with_context(|| format!("无法启动 {tool}"))?;
    let deadline = Instant::now() + Duration::from_secs(180);
    loop {
        if let Some(status) = child.try_wait()? {
            stdout.rewind()?;
            stderr.rewind()?;
            let mut out = Vec::new();
            let mut err = String::new();
            stdout.take(8 * 1024 * 1024).read_to_end(&mut out)?;
            stderr.take(8192).read_to_string(&mut err)?;
            ensure!(status.success(), "{tool} 执行失败：{}", err.trim());
            return Ok(out);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("{tool} 执行超时；文件和备份已保留");
        }
        std::thread::sleep(Duration::from_millis(40));
    }
}
pub fn home() -> Result<PathBuf> {
    Ok(PathBuf::from(
        std::env::var_os("HOME").context("无法获取用户主目录")?,
    ))
}
pub fn plist(path: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&run(
        "/usr/bin/plutil",
        &[
            "-convert".as_ref(),
            "json".as_ref(),
            "-o".as_ref(),
            "-".as_ref(),
            path.as_os_str(),
        ],
    )?)?)
}
pub fn validate_bundle(path: &Path) -> Result<PathBuf> {
    ensure!(
        !fs::symlink_metadata(path)?.file_type().is_symlink(),
        "请选择真实的 Claude.app，不是符号链接"
    );
    let bundle = path.canonicalize()?;
    ensure!(
        bundle.extension().is_some_and(|ext| ext == "app"),
        "请选择 Claude.app 应用"
    );
    let info = plist(&bundle.join("Contents/Info.plist"))?;
    ensure!(
        info["CFBundleIdentifier"] == BUNDLE_ID && info["CFBundleExecutable"] == "Claude",
        "不是 Claude Desktop 应用"
    );
    ensure!(
        bundle.join("Contents/MacOS/Claude").is_file()
            && bundle.join("Contents/Resources/app.asar").is_file(),
        "Claude.app 文件不完整"
    );
    Ok(bundle)
}
pub fn verify_original(bundle: &Path) -> Result<()> {
    validate_bundle(bundle)?;
    run(
        "/usr/bin/codesign",
        &[
            "--verify".as_ref(),
            "--deep".as_ref(),
            "--strict".as_ref(),
            "-R".as_ref(),
            REQUIREMENT.as_ref(),
            bundle.as_os_str(),
        ],
    )?;
    run(
        "/usr/sbin/spctl",
        &[
            "--assess".as_ref(),
            "--type".as_ref(),
            "execute".as_ref(),
            bundle.as_os_str(),
        ],
    )?;
    Ok(())
}
pub fn inspect(bundle: &Path) -> Result<Installation> {
    let bundle = validate_bundle(bundle)?;
    let info = plist(&bundle.join("Contents/Info.plist"))?;
    let version = info["CFBundleShortVersionString"]
        .as_str()
        .context("应用缺少版本号")?
        .to_owned();
    crate::downloads::compare_versions(&version, &version)?;
    let archs = String::from_utf8(run(
        "/usr/bin/lipo",
        &[
            "-archs".as_ref(),
            bundle.join("Contents/MacOS/Claude").as_os_str(),
        ],
    )?)?;
    let architecture = if archs.contains("arm64") && archs.contains("x86_64") {
        "universal"
    } else if archs.contains("arm64") {
        "arm64"
    } else if archs.contains("x86_64") {
        "x64"
    } else {
        bail!("不支持的 Claude 架构")
    };
    let layout = if bundle.join("Contents/Resources/ion-dist.asar").is_file() {
        "asar"
    } else {
        "unknown"
    };
    Ok(Installation {
        path: bundle,
        version,
        architecture: architecture.into(),
        kind: "app".into(),
        layout: layout.into(),
    })
}

pub fn parse_processes(text: &str) -> Vec<(u32, PathBuf)> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            let split = line.find(char::is_whitespace)?;
            let pid: u32 = line[..split].parse().ok()?;
            let path = PathBuf::from(line[split..].trim());
            (pid > 1
                && path.to_string_lossy().starts_with('/')
                && path.ends_with("Contents/MacOS/Claude"))
            .then_some((pid, path))
        })
        .collect()
}
pub fn processes() -> Result<Vec<(u32, PathBuf)>> {
    let bytes = run(
        "/bin/ps",
        &["-ww".as_ref(), "-axo".as_ref(), "pid=,comm=".as_ref()],
    )?;
    Ok(parse_processes(std::str::from_utf8(&bytes)?))
}
pub fn force_exit() -> Result<()> {
    for (pid, path) in processes()? {
        let bundle = path
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .context("进程路径无效")?;
        validate_bundle(bundle)?;
        // Re-read identity immediately before signalling; never target unrelated process names.
        if processes()?
            .iter()
            .any(|(p, exe)| *p == pid && *exe == path)
        {
            run("/bin/kill", &["-KILL".as_ref(), pid.to_string().as_ref()])?;
        }
    }
    let deadline = Instant::now() + Duration::from_secs(8);
    while !processes()?.is_empty() {
        ensure!(
            Instant::now() < deadline,
            "Claude 尚未退出，请手动关闭后重试"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}
pub fn snapshot(root: &Path, settings: Settings) -> Result<Snapshot> {
    let mut candidates = vec![
        home()?.join("Applications/Claude.app"),
        PathBuf::from("/Applications/Claude.app"),
    ];
    if let Some(path) = &settings.source_path {
        candidates.insert(0, path.clone());
    }
    let mut installations = Vec::new();
    let mut warnings = Vec::new();
    for path in candidates {
        if !path.exists() {
            continue;
        }
        match inspect(&path) {
            Ok(item)
                if !installations
                    .iter()
                    .any(|i: &Installation| i.path == item.path) =>
            {
                installations.push(item)
            }
            Ok(_) => (),
            Err(error) => warnings.push(format!("{}：{error:#}", path.display())),
        }
    }
    let selected = if let Some(path) = &settings.source_path {
        let canonical = path.canonicalize().ok();
        let found = installations
            .iter()
            .find(|i| Some(&i.path) == canonical.as_ref())
            .cloned();
        if found.is_none() {
            warnings.push("已选择的 Claude.app 不可用，请重新选择".into());
        }
        found
    } else {
        installations.first().cloned()
    };
    Ok(Snapshot {
        settings,
        installations,
        selected,
        running: !processes()?.is_empty(),
        warnings,
        data_dir: root.into(),
        platform: "macos".into(),
    })
}
pub fn copy_bundle(source: &Path, target: &Path) -> Result<()> {
    ensure!(!target.exists(), "副本目标已存在，未覆盖");
    run("/usr/bin/ditto", &[source.as_os_str(), target.as_os_str()])?;
    Ok(())
}

/// Framework bundles contain relative symlinks. Accept only links resolving inside the app;
/// never follow them while walking or signing, which also prevents cycles and duplicate signing.
pub fn tree_files(root: &Path) -> Result<Vec<PathBuf>> {
    fn visit(dir: &Path, base: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            let meta = fs::symlink_metadata(&path)?;
            if meta.file_type().is_symlink() {
                ensure!(
                    path.canonicalize()?.starts_with(base),
                    "应用内链接越界：{}",
                    path.display()
                );
            } else if meta.is_dir() {
                visit(&path, base, files)?;
            } else if meta.is_file() {
                files.push(path);
            } else {
                bail!("应用包含不支持的特殊文件");
            }
        }
        Ok(())
    }
    let base = root.canonicalize()?;
    let mut files = Vec::new();
    visit(&base, &base, &mut files)?;
    Ok(files)
}
pub fn verify_copy(bundle: &Path) -> Result<()> {
    validate_bundle(bundle)?;
    tree_files(bundle)?;
    run(
        "/usr/bin/codesign",
        &[
            "--verify".as_ref(),
            "--deep".as_ref(),
            "--strict".as_ref(),
            bundle.as_os_str(),
        ],
    )?;
    Ok(())
}
pub fn sign_copy(source: &Path, bundle: &Path) -> Result<()> {
    let source = source.canonicalize()?;
    let bundle_path = bundle.canonicalize()?;
    let bundle = bundle_path.as_path();
    use std::io::Read;
    let files = tree_files(bundle)?;
    let mut loose = Vec::new();
    let mut bundles = std::collections::BTreeSet::new();
    for path in files {
        let mut magic = [0u8; 4];
        if fs::File::open(&path)?.read(&mut magic)? == 4
            && matches!(
                magic,
                [0xcf, 0xfa, 0xed, 0xfe]
                    | [0xfe, 0xed, 0xfa, 0xcf]
                    | [0xce, 0xfa, 0xed, 0xfe]
                    | [0xfe, 0xed, 0xfa, 0xce]
                    | [0xca, 0xfe, 0xba, 0xbe]
                    | [0xbe, 0xba, 0xfe, 0xca]
                    | [0xca, 0xfe, 0xba, 0xbf]
                    | [0xbf, 0xba, 0xfe, 0xca]
            )
        {
            loose.push(path.clone());
        }
        for parent in path.ancestors().skip(1).take_while(|p| *p != bundle) {
            if parent
                .extension()
                .is_some_and(|s| s == "framework" || s == "app")
            {
                bundles.insert(parent.to_path_buf());
            }
        }
    }
    loose.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    let mut nested: Vec<_> = bundles.into_iter().collect();
    nested.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    let temp = tempfile::tempdir()?;
    for (index, target) in loose
        .into_iter()
        .chain(nested)
        .chain(std::iter::once(bundle.to_path_buf()))
        .enumerate()
    {
        let original = source.join(target.strip_prefix(bundle)?);
        let bytes = run(
            "/usr/bin/codesign",
            &[
                "--display".as_ref(),
                "--entitlements".as_ref(),
                ":-".as_ref(),
                "--xml".as_ref(),
                original.as_os_str(),
            ],
        )?;
        let ent_path = temp.path().join(format!("{index}.plist"));
        let mut ent = if bytes.is_empty() {
            serde_json::json!({})
        } else {
            fs::write(&ent_path, bytes)?;
            plist(&ent_path)?
        };
        let obj = ent.as_object_mut().context("签名权限格式无效")?;
        for key in [
            "com.apple.application-identifier",
            "com.apple.developer.team-identifier",
            "keychain-access-groups",
        ] {
            obj.remove(key);
        }
        obj.insert(
            "com.apple.security.cs.disable-library-validation".into(),
            Value::Bool(true),
        );
        fs::write(&ent_path, serde_json::to_vec(&ent)?)?;
        run(
            "/usr/bin/plutil",
            &["-convert".as_ref(), "xml1".as_ref(), ent_path.as_os_str()],
        )?;
        run(
            "/usr/bin/codesign",
            &[
                "--force".as_ref(),
                "--sign".as_ref(),
                "-".as_ref(),
                "--options".as_ref(),
                "runtime".as_ref(),
                "--timestamp=none".as_ref(),
                "--entitlements".as_ref(),
                ent_path.as_os_str(),
                target.as_os_str(),
            ],
        )?;
    }
    verify_copy(bundle)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn process_paths_with_spaces_are_not_split_or_confused_with_cli() {
        let rows = parse_processes(" 120 /Users/Some One/Applications/Claude.app/Contents/MacOS/Claude\n121 /usr/local/bin/claude\n122 /Applications/Claude.app/Contents/Frameworks/Helper\n");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, 120);
        assert!(rows[0].1.starts_with("/Users/Some One"));
    }
    #[cfg(unix)]
    #[test]
    fn framework_links_stay_inside_bundle_and_escape_is_rejected() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let bundle = root.path().join("Claude.app");
        fs::create_dir_all(bundle.join("Versions/A")).unwrap();
        fs::write(bundle.join("Versions/A/binary"), b"test").unwrap();
        symlink("A", bundle.join("Versions/Current")).unwrap();
        assert_eq!(tree_files(&bundle).unwrap().len(), 1);
        symlink(root.path(), bundle.join("escape")).unwrap();
        assert!(tree_files(&bundle).is_err());
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn native_copy_preserves_links_and_executable_permissions_and_can_be_resigned() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("Source.app");
        fs::create_dir_all(source.join("Contents/MacOS")).unwrap();
        fs::create_dir_all(source.join("Contents/Resources")).unwrap();
        let info = serde_json::json!({"CFBundleIdentifier":BUNDLE_ID,"CFBundleExecutable":"Claude","CFBundleShortVersionString":"1.0.0","CFBundlePackageType":"APPL","CFBundleName":"Claude"});
        fs::write(
            source.join("Contents/Info.plist"),
            serde_json::to_vec(&info).unwrap(),
        )
        .unwrap();
        run(
            "/usr/bin/plutil",
            &[
                "-convert".as_ref(),
                "xml1".as_ref(),
                source.join("Contents/Info.plist").as_os_str(),
            ],
        )
        .unwrap();
        fs::copy("/bin/echo", source.join("Contents/MacOS/Claude")).unwrap();
        fs::set_permissions(
            source.join("Contents/MacOS/Claude"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        fs::write(source.join("Contents/Resources/app.asar"), b"fixture").unwrap();
        symlink("app.asar", source.join("Contents/Resources/link")).unwrap();
        run(
            "/usr/bin/codesign",
            &[
                "--force".as_ref(),
                "--sign".as_ref(),
                "-".as_ref(),
                source.as_os_str(),
            ],
        )
        .unwrap();
        let copy = root.path().join("Copy.app");
        copy_bundle(&source, &copy).unwrap();
        assert!(fs::symlink_metadata(copy.join("Contents/Resources/link"))
            .unwrap()
            .file_type()
            .is_symlink());
        assert_ne!(
            fs::metadata(copy.join("Contents/MacOS/Claude"))
                .unwrap()
                .permissions()
                .mode()
                & 0o111,
            0
        );
        sign_copy(&source, &copy).unwrap();
        verify_copy(&copy).unwrap();
        assert!(
            verify_original(&copy).is_err(),
            "ad-hoc fixture must never pass original Developer ID verification"
        );
    }
}
