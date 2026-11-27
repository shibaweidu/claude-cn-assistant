//! Windows MSIX 安装协调。启动系统安装器不等于安装成功。
use crate::downloads::{compare_versions, Downloads};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg_attr(target_os = "macos", allow(dead_code))]
pub const PUBLISHER: &str = r#"CN="Anthropic, PBC", O="Anthropic, PBC", L=San Francisco, S=California, C=US, SERIALNUMBER=4860621, OID.2.5.4.15=Private Organization, OID.1.3.6.1.4.1.311.60.2.1.2=Delaware, OID.1.3.6.1.4.1.311.60.2.1.3=US"#;
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PackageInfo {
    pub name: String,
    pub publisher: String,
    pub version: String,
    pub architecture: String,
    pub signature_status: String,
    pub signer_matches: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredPackage {
    pub name: String,
    pub publisher: String,
    pub version: String,
    pub architecture: String,
    pub family: String,
    pub status: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub architecture: String,
    pub running: bool,
    pub packages: Vec<RegisteredPackage>,
}
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstallState {
    pub id: String,
    pub stage: String,
    pub task_id: String,
    pub info: Option<PackageInfo>,
    pub message: String,
    pub started_at: u64,
    pub checked_at: u64,
    pub needs_rebuild: bool,
    #[serde(default)]
    pub target_path: Option<PathBuf>,
    #[serde(default)]
    pub backup_path: Option<PathBuf>,
    #[serde(default)]
    pub staging_path: Option<PathBuf>,
}
impl InstallState {
    pub fn active(&self) -> bool {
        matches!(
            self.stage.as_str(),
            "checking" | "ready" | "launching" | "awaitingInstaller"
        )
    }
}
pub struct Installer {
    root: PathBuf,
    state: Mutex<InstallState>,
}
#[derive(Deserialize)]
struct Envelope<T> {
    ok: bool,
    data: Option<T>,
    error: Option<String>,
}
fn probe<T: serde::de::DeserializeOwned>(script: &str, path: Option<&Path>) -> Result<T> {
    let output = crate::powershell(script, path)?;
    let envelope: Envelope<T> =
        serde_json::from_str(&output).context("Windows 安装检测返回格式无效")?;
    ensure!(
        envelope.ok,
        "{}",
        envelope.error.unwrap_or_else(|| "系统检测失败".into())
    );
    envelope.data.context("Windows 安装检测缺少结果")
}
pub(crate) fn force_exit_claude() -> Result<()> {
    #[cfg(target_os = "macos")]
    return crate::macos::force_exit();
    #[cfg(not(target_os = "macos"))]
    {
        probe::<bool>(r#"
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$ErrorActionPreference='Stop'
try {
    $processes = @(Get-Process | Where-Object { $_.ProcessName -eq 'Claude' })
    foreach ($process in $processes) {
        try { Stop-Process -InputObject $process -Force -ErrorAction Stop }
        catch { if (-not $process.HasExited) { throw } }
    }
    foreach ($process in $processes) { if (-not $process.WaitForExit(5000)) { throw 'Claude 退出超时' } }
    @{ok=$true;data=$true}|ConvertTo-Json -Compress
} catch { @{ok=$false;error=$_.Exception.Message}|ConvertTo-Json -Compress }
"#, None).context("无法退出 Claude，请手动关闭后重试")?;
        Ok(())
    }
}
const INSPECT: &str = r#"
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false);$ErrorActionPreference='Stop'
try {
 Add-Type -AssemblyName System.IO.Compression.FileSystem
 $zip=[IO.Compression.ZipFile]::OpenRead($env:CLAUDE_CN_PROBE_PATH)
 try {
  $entries=@($zip.Entries | Where-Object { $_.FullName -ceq 'AppxManifest.xml' })
  if($entries.Count -ne 1 -or $entries[0].Length -gt 1048576){ throw 'MSIX 清单不存在、重复或过大' }
  $reader=[Xml.XmlReaderSettings]::new();$reader.DtdProcessing=[Xml.DtdProcessing]::Prohibit;$reader.XmlResolver=$null;$reader.MaxCharactersInDocument=1048576
  $stream=$entries[0].Open();$xmlReader=[Xml.XmlReader]::Create($stream,$reader)
  try {$xml=[Xml.XmlDocument]::new();$xml.XmlResolver=$null;$xml.Load($xmlReader)} finally {$xmlReader.Dispose();$stream.Dispose()}
  $identity=$xml.DocumentElement.SelectSingleNode('*[local-name()="Identity"]')
  if(-not $identity){throw 'MSIX 缺少 Identity'}
  $name=$identity.GetAttribute('Name');$publisher=$identity.GetAttribute('Publisher');$version=$identity.GetAttribute('Version');$arch=$identity.GetAttribute('ProcessorArchitecture')
 } finally {$zip.Dispose()}
 $sig=Get-AuthenticodeSignature -LiteralPath $env:CLAUDE_CN_PROBE_PATH
 $matches=$false
 if($sig.SignerCertificate){
  $expected=[Security.Cryptography.X509Certificates.X500DistinguishedName]::new($publisher)
  # DER 字符串类型可能不同；比较 Windows 解码后的完整 X.500 名称。
  $matches=[string]::Equals($expected.Decode([Security.Cryptography.X509Certificates.X500DistinguishedNameFlags]::None),$sig.SignerCertificate.SubjectName.Decode([Security.Cryptography.X509Certificates.X500DistinguishedNameFlags]::None),[StringComparison]::Ordinal)
 }
 @{ok=$true;data=@{name=$name;publisher=$publisher;version=$version;architecture=$arch;signatureStatus="$($sig.Status)";signerMatches=$matches}} | ConvertTo-Json -Depth 5 -Compress
} catch {@{ok=$false;error=$_.Exception.Message} | ConvertTo-Json -Compress}
"#;
#[cfg(not(target_os = "macos"))]
const ENVIRONMENT: &str = r#"
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false);$ErrorActionPreference='Stop'
try {
 $arch=$env:PROCESSOR_ARCHITEW6432;if(-not $arch){$arch=$env:PROCESSOR_ARCHITECTURE}
 $arch=switch($arch.ToUpperInvariant()){'AMD64' {'x64'} 'ARM64' {'arm64'} 'X86' {'x86'} default {'unknown'}}
 $packages=@(Get-AppxPackage -Name 'Claude' -ErrorAction Stop | ForEach-Object {
  @{name=$_.Name;publisher=$_.Publisher;version="$($_.Version)";family=$_.PackageFamilyName;architecture="$($_.Architecture)".ToLowerInvariant();status="$($_.Status)"}
 })
 @{ok=$true;data=@{architecture=$arch;running=(@(Get-Process -Name 'Claude' -ErrorAction SilentlyContinue).Count -gt 0);packages=$packages}} | ConvertTo-Json -Depth 5 -Compress
} catch {@{ok=$false;error=$_.Exception.Message} | ConvertTo-Json -Compress}
"#;
pub fn inspect_package(path: &Path) -> Result<PackageInfo> {
    probe(INSPECT, Some(path))
}
pub fn environment() -> Result<Environment> {
    #[cfg(target_os = "macos")]
    return Ok(Environment {
        architecture: std::env::consts::ARCH.into(),
        running: !crate::macos::processes()?.is_empty(),
        packages: Vec::new(),
    });
    #[cfg(not(target_os = "macos"))]
    probe(ENVIRONMENT, None)
}
pub fn validate_package(
    info: &PackageInfo,
    expected_version: &str,
    expected_arch: &str,
    env: &Environment,
) -> Result<()> {
    ensure!(cfg!(windows), "当前平台不能安装 Windows MSIX");
    ensure!(
        info.name == "Claude" && info.publisher == PUBLISHER,
        "安装包身份或发布者不匹配，不会启动安装器"
    );
    ensure!(
        info.signature_status == "Valid" && info.signer_matches,
        "安装包签名未通过 Windows 信任校验或签名者与清单不一致（{}）",
        info.signature_status
    );
    ensure!(
        compare_versions(&info.version, expected_version)?.is_eq(),
        "安装包版本与下载清单不一致"
    );
    ensure!(
        info.architecture == expected_arch && env.architecture == expected_arch,
        "安装包与本机处理器架构不匹配，请下载本机架构的安装包"
    );
    ensure!(
        !env.running,
        "Claude 正在运行，请确认强制退出后继续安装，或正常退出后重试"
    );
    for package in &env.packages {
        ensure!(
            package.name == "Claude"
                && package.publisher == PUBLISHER
                && package.family == "Claude_pzs8sxrjxfjjc",
            "本机有同名但身份不一致的包，请先检查安装来源"
        );
        ensure!(
            !compare_versions(&package.version, &info.version)?.is_gt(),
            "本机已安装更高版本，不支持在此降级"
        );
    }
    Ok(())
}
pub fn installation_confirmed(info: &PackageInfo, env: &Environment) -> bool {
    env.packages.iter().any(|p| {
        p.name == "Claude"
            && p.publisher == PUBLISHER
            && p.family == "Claude_pzs8sxrjxfjjc"
            && p.status == "Ok"
            && p.architecture == info.architecture
            && compare_versions(&p.version, &info.version).is_ok_and(|order| !order.is_lt())
    })
}
impl Installer {
    pub fn new(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root)?;
        let mut state = fs::read(root.join("install-state.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<InstallState>(&b).ok())
            .unwrap_or_default();
        if state.active() {
            state.stage = "interrupted".into();
            state.message =
                "上次安装流程已中断。系统安装可能仍在进行，请重新检测实际安装结果。".into();
        }
        Ok(Self {
            root,
            state: Mutex::new(state),
        })
    }
    pub fn state(&self) -> InstallState {
        self.state.lock().unwrap().clone()
    }
    fn save(&self, state: &InstallState) -> Result<()> {
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new_in(&self.root)?;
        file.write_all(&serde_json::to_vec_pretty(state)?)?;
        file.as_file().sync_all()?;
        file.persist(self.root.join("install-state.json"))
            .map_err(|e| e.error)?;
        Ok(())
    }
    fn set(&self, state: InstallState) -> Result<InstallState> {
        self.save(&state)?;
        *self.state.lock().unwrap() = state.clone();
        Ok(state)
    }
    pub fn reserve(&self) -> Result<String> {
        let mut current = self.state.lock().unwrap();
        ensure!(!current.active(), "已有安装流程，请先完成或结束等待");
        let state = InstallState {
            id: uuid::Uuid::new_v4().to_string(),
            stage: "checking".into(),
            message: "正在核对下载、签名、包身份和本机环境…".into(),
            ..Default::default()
        };
        self.save(&state)?;
        let id = state.id.clone();
        *current = state;
        Ok(id)
    }
    pub fn prepare(&self, downloads: &Downloads, id: &str) -> Result<InstallState> {
        #[cfg(target_os = "macos")]
        return self.prepare_macos(downloads, id);
        #[cfg(not(target_os = "macos"))]
        {
            let mut state = self.state();
            ensure!(
                state.id == id && state.stage == "checking",
                "安装检查状态已变化"
            );
            let result = (|| {
                let task = downloads.verify_saved()?;
                ensure!(task.stage == "ready", "下载文件未通过 SHA256 校验");
                let arch = match task.asset_name.as_str() {
                    "Claude-win-x64.msix" => "x64",
                    "Claude-win-arm64.msix" => "arm64",
                    _ => anyhow::bail!("仅支持 Windows MSIX，其他平台安装包只能保存"),
                };
                let path = task.path.as_ref().context("安装包路径缺失")?;
                let _guard = lock_package(path)?;
                ensure!(
                    downloads.verify_saved()?.stage == "ready",
                    "锁定后安装包完整性校验失败"
                );
                let info = inspect_package(path)?;
                let mut env = environment()?;
                // 检查包时允许客户端运行，真正安装前仍必须退出。
                env.running = false;
                validate_package(&info, &task.version, arch, &env)?;
                self.check_standard_versions(&info.version)?;
                state.task_id = task.id;
                state.info = Some(info);
                state.stage = "ready".into();
                state.message =
                    "下载完整性、Windows 签名、包身份和本机环境检查通过。请确认后打开系统安装器。"
                        .into();
                Ok::<(), anyhow::Error>(())
            })();
            if let Err(error) = result {
                state.stage = "failed".into();
                state.message = format!("{error:#}");
            }
            state.checked_at = now();
            self.set(state)
        }
    }
    pub fn launch(
        &self,
        downloads: &Downloads,
        id: &str,
        force_exit: bool,
    ) -> Result<InstallState> {
        #[cfg(target_os = "macos")]
        return self.launch_macos(downloads, id, force_exit);
        #[cfg(not(target_os = "macos"))]
        {
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
                    "下载任务已变化，请重新检查安装包"
                );
                let path = task.path.as_ref().context("安装包路径丢失")?;
                let _guard = lock_package(path)?;
                ensure!(
                    downloads.verify_saved()?.stage == "ready",
                    "锁定后安装包完整性校验失败"
                );
                let info = inspect_package(path)?;
                let mut env = environment()?;
                let was_running = env.running;
                env.running = false;
                let expected = state.info.as_ref().context("没有安装包信息")?;
                validate_package(&info, &task.version, &expected.architecture, &env)?;
                self.check_standard_versions(&info.version)?;
                ensure!(
                    info.name == expected.name && info.version == expected.version,
                    "确认后安装包信息发生变化"
                );
                if installation_confirmed(&info, &env) {
                    state.stage = "installed".into();
                    state.needs_rebuild = self.root.join("managed").exists();
                    state.message = "本机已注册该版本或更高版本，无需重复安装。".into();
                    return Ok(());
                }
                ensure!(
                    !was_running || force_exit,
                    "Claude 正在运行，请确认强制退出后继续安装"
                );
                if force_exit {
                    // 仅在本次安装明确授权且包重新验证通过后关闭 Claude。
                    force_exit_claude()?;
                }
                let final_env = environment()?;
                validate_package(&info, &task.version, &expected.architecture, &final_env)?;
                // ShellExecute 通过文件关联交给 Windows 应用安装程序，不拼接文件路径。
                probe::<bool>(r#"[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false);$ErrorActionPreference='Stop';try {Start-Process -FilePath $env:CLAUDE_CN_PROBE_PATH -ErrorAction Stop;@{ok=$true;data=$true}|ConvertTo-Json -Compress}catch{@{ok=$false;error=$_.Exception.Message}|ConvertTo-Json -Compress}"#,Some(path)).context("无法打开 Windows 应用安装程序，请检查 MSIX 文件关联或安装策略")?;
                state.stage = "awaitingInstaller".into();
                state.started_at = now();
                state.message="系统安装器已打开，请在系统窗口完成安装。助手将重新检测包版本，打开安装器本身不算安装成功。".into();
                Ok::<(), anyhow::Error>(())
            })();
            if let Err(error) = result {
                state.stage = "failed".into();
                state.message = format!("{error:#}");
            }
            state.checked_at = now();
            self.set(state)
        }
    }
    pub fn recheck(&self) -> Result<InstallState> {
        #[cfg(target_os = "macos")]
        return self.recheck_macos();
        #[cfg(not(target_os = "macos"))]
        {
            let mut state = self.state();
            ensure!(
                matches!(
                    state.stage.as_str(),
                    "awaitingInstaller" | "interrupted" | "unconfirmed" | "stopped"
                ),
                "当前没有待确认的安装结果"
            );
            let info = state
                .info
                .as_ref()
                .context("没有安装包信息，请重新执行安装前检查")?;
            match environment() {
                Ok(env) => {
                    if installation_confirmed(info, &env) {
                        state.stage = "installed".into();
                        state.needs_rebuild = self.root.join("managed").exists();
                        state.message=if state.needs_rebuild{"已检测到目标版本或更高版本。已有受管副本需要重新校验并重建；当前不会自动执行汉化。"}else{"已检测到目标版本或更高版本，安装结果已确认。"}.into();
                    } else if state.stage == "awaitingInstaller"
                        && now().saturating_sub(state.started_at) < 600
                    {
                        state.message="尚未检测到目标版本，请继续完成系统安装。若系统提示失败，请按系统说明处理；关闭安装窗口不代表成功。".into();
                    } else {
                        state.stage = "unconfirmed".into();
                        state.message="未确认安装成功。可能尚未完成、已取消或安装失败；请检查系统安装器后重新检测。".into();
                    }
                }
                Err(error) => {
                    if state.stage == "awaitingInstaller"
                        && now().saturating_sub(state.started_at) >= 600
                    {
                        state.stage = "unconfirmed".into();
                    }
                    state.message = format!("暂时无法检测安装结果：{error:#}。可以稍后重新检测。");
                }
            }
            state.checked_at = now();
            self.set(state)
        }
    }
    #[cfg(not(target_os = "macos"))]
    fn check_standard_versions(&self, expected: &str) -> Result<()> {
        let snapshot = crate::snapshot(&self.root)?;
        for installation in snapshot
            .installations
            .iter()
            .filter(|i| i.kind == "standard")
        {
            ensure!(
                !compare_versions(&installation.version, expected)?.is_gt(),
                "检测到更高版本的普通安装，不支持通过此流程降级"
            );
        }
        Ok(())
    }
    pub fn stop(&self) -> Result<InstallState> {
        let mut state = self.state();
        ensure!(
            matches!(
                state.stage.as_str(),
                "ready" | "awaitingInstaller" | "interrupted" | "unconfirmed"
            ),
            "正在执行检查或打开安装器，请稍候"
        );
        state.stage = "stopped".into();
        state.message="助手已结束等待。此操作不会关闭系统安装器或取消 Windows 正在执行的安装，仍可重新检测结果。".into();
        self.set(state)
    }
}
#[cfg(not(target_os = "macos"))]
fn lock_package(path: &Path) -> Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1);
    }
    options
        .open(path)
        .context("无法锁定安装包，文件可能正被修改或已移走")
}
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod macos;
#[cfg(test)]
mod tests;
