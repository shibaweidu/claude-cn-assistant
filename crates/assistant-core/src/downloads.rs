//! 固定 GitHub Release 的下载与完整性校验；不访问公益托管加速端点。
use anyhow::{bail, ensure, Context, Result};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const REPO: &str = "https://github.com/Wangnov/claude-app-mirror";
const MIRROR:&str="https://claudeapp.agentsmirror.com";
#[derive(Clone,Debug,Default,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ChannelConfig {pub accelerated:bool,pub repository:String}
impl ChannelConfig {
    fn user_agent(&self)->Result<String>{
        let url=Url::parse(self.repository.trim()).context("请输入真实公开仓库地址")?;
        ensure!(url.scheme()=="https"&&matches!(url.host_str(),Some("github.com"|"gitlab.com"))&&url.username().is_empty()&&url.password().is_none()&&url.query().is_none()&&url.fragment().is_none(),"仓库地址必须是 GitHub 或 GitLab 的 HTTPS 项目地址");
        let parts:Vec<_>=url.path().trim_matches('/').split('/').collect();
        ensure!(parts.len()>=2&&parts.iter().all(|s|!s.is_empty()&&s.bytes().all(|b|b.is_ascii_alphanumeric()||b"._-".contains(&b)))&&self.repository.len()<180,"请填写完整公开仓库地址");
        ensure!(url.as_str().trim_end_matches('/')!=REPO,"请填写本助手真实仓库，不能使用镜像项目仓库冒充客户端");
        Ok(format!("ClaudeCnAssistant/0.1.0 (+{}; open-source-noncommercial)",url.as_str().trim_end_matches('/')))
    }
}
const CACHE_TTL: u64 = 6 * 3600;
const MANUAL_COOLDOWN: u64 = 60;
const META_LIMIT: usize = 2 * 1024 * 1024;
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub platform: String,
    pub architecture: String,
    pub name: String,
    pub version: String,
    pub size: u64,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    #[serde(default)]
    pub accelerated:bool,
    pub version: String,
    pub tag: String,
    pub checked_at: u64,
    pub assets: Vec<Asset>,
}
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub stage: String,
    pub asset_name: String,
    pub version: String,
    pub downloaded: u64,
    pub total: u64,
    pub sha256: String,
    pub path: Option<PathBuf>,
    pub error: Option<String>,
    pub cancellable: bool,
}
impl Task {
    pub fn active(&self) -> bool {
        matches!(
            self.stage.as_str(),
            "connecting" | "downloading" | "verifying" | "committing"
        )
    }
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadState {
    pub channel:ChannelConfig,
    pub release: Option<Release>,
    pub task: Option<Task>,
    pub last_error: Option<String>,
    pub retry_at: u64,
    pub stale: bool,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cache {
    release: Option<Release>,
    last_attempt: u64,
    retry_at: u64,
    last_error: Option<String>,
}
pub struct Downloads {
    channel:Mutex<ChannelConfig>,
    root: PathBuf,
    client: Client,
    cache: Mutex<Cache>,
    task: Mutex<Option<Task>>,
    cancel: AtomicBool,
    check_lock: tokio::sync::Mutex<()>,
}
fn allowed(url: &Url) -> bool {
    url.scheme() == "https"
        && matches!(
            url.host_str(),
            Some(
                "github.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
            )
        )
}
fn client() -> Result<Client> {
    Ok(Client::builder()
        .user_agent("ClaudeCnAssistant/0.1.0 (GitHub release client)")
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(1800))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 8 {
                attempt.error("重定向次数过多")
            } else if allowed(attempt.url()) {
                attempt.follow()
            } else {
                attempt.error("下载重定向到了未允许的地址")
            }
        }))
        .build()?)
}
fn accelerated_client(config:&ChannelConfig)->Result<Client>{
    Ok(Client::builder().user_agent(config.user_agent()?).connect_timeout(Duration::from_secs(15)).timeout(Duration::from_secs(1800))
        .redirect(reqwest::redirect::Policy::custom(|attempt|{
            let url=attempt.url();
            // 存储节点由公开路由动态选择，不自行拼接底层存储地址。
            let public_host=url.domain().is_some_and(|host|host.contains('.')&&!host.ends_with(".localhost")&&!host.ends_with(".local"));
            if attempt.previous().len()>=8{attempt.error("重定向过多")}
            else if url.scheme()=="https"&&url.username().is_empty()&&url.password().is_none()&&public_host&&(url.port().is_none()||url.port()==Some(443)){attempt.follow()}
            else{attempt.error("镜像返回了不允许的重定向地址")}
        })).build()?)
}
fn atomic_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path.parent().context("数据目录无效")?;
    fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.as_file().sync_all()?;
    // persist 在 Windows 使用支持替换的系统原子重命名。
    file.persist(path).map_err(|e| e.error)?;
    Ok(())
}
fn valid_hash(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
fn valid_version(s: &str) -> bool {
    !s.is_empty()
        && s.len() < 80
        && s.split('.')
            .all(|p| !p.is_empty() && p.bytes().all(|c| c.is_ascii_digit()))
}
fn valid_tag(s: &str) -> bool {
    s.starts_with("claude-app-v")
        && s.len() < 160
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
}
pub fn compare_versions(candidate: &str, current: &str) -> Result<std::cmp::Ordering> {
    ensure!(
        valid_version(candidate) && valid_version(current),
        "无法比较非标准版本号"
    );
    let mut a: Vec<u64> = candidate
        .split('.')
        .map(str::parse)
        .collect::<std::result::Result<_, _>>()?;
    let mut b: Vec<u64> = current
        .split('.')
        .map(str::parse)
        .collect::<std::result::Result<_, _>>()?;
    let len = a.len().max(b.len());
    a.resize(len, 0);
    b.resize(len, 0);
    Ok(a.cmp(&b))
}
pub fn parse_checksums(text: &str) -> Result<BTreeMap<String, String>> {
    let mut hashes = BTreeMap::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let split = line.find(char::is_whitespace).context("校验和格式不正确")?;
        let (hash, name) = line.split_at(split);
        let name = name.trim_start().trim_start_matches('*');
        ensure!(
            valid_hash(hash)
                && !name.is_empty()
                && !name.contains(['/', '\\'])
                && !name.contains(".."),
            "校验和包含无效条目"
        );
        ensure!(
            hashes
                .insert(name.to_string(), hash.to_lowercase())
                .is_none(),
            "校验和有重复文件名"
        );
    }
    ensure!(!hashes.is_empty(), "校验和为空");
    Ok(hashes)
}
pub fn parse_release(tag: &str, manifest: &[u8], checksums: &str) -> Result<Release> {
    ensure!(valid_tag(tag), "发布标签无效");
    let hashes = parse_checksums(checksums)?;
    let hash = format!("{:x}", Sha256::digest(manifest));
    ensure!(
        hashes.get("release-manifest.json") == Some(&hash),
        "发布清单的 SHA256 校验失败"
    );
    let value: serde_json::Value = serde_json::from_slice(manifest)?;
    ensure!(
        value["schemaVersion"].as_u64() == Some(2),
        "镜像清单版本不受支持"
    );
    let version = value["version"].as_str().context("清单缺少版本号")?;
    ensure!(valid_version(version), "清单版本号格式无效");
    // 老的正式清单不含 releaseTag；后续清单含此字段时必须一致。
    if let Some(own) = value.get("releaseTag") {
        ensure!(own.as_str() == Some(tag), "清单与发布标签不匹配");
    }
    ensure!(
        tag == format!("claude-app-v{version}")
            || tag.starts_with(&format!("claude-app-v{version}-")),
        "标签与版本不匹配"
    );
    let mut assets = Vec::new();
    for (platform, arch, name) in [
        ("windows", "x64", "Claude-win-x64.msix"),
        ("windows", "arm64", "Claude-win-arm64.msix"),
        ("macos", "universal", "Claude-mac-universal.dmg"),
        ("linux", "x64", "Claude-linux-x64.deb"),
        ("linux", "arm64", "Claude-linux-arm64.deb"),
    ] {
        let source = &value["sources"][platform][arch];
        ensure!(
            source["assetName"].as_str() == Some(name),
            "{name} 文件名不匹配"
        );
        let sha = source["sha256"]
            .as_str()
            .context("清单缺少 SHA256")?
            .to_lowercase();
        ensure!(
            valid_hash(&sha) && hashes.get(name) == Some(&sha),
            "{name} 校验和与清单不一致"
        );
        let size = source["contentLength"]
            .as_u64()
            .context("清单缺少有效文件大小")?;
        ensure!(
            size > 0 && size <= 8 * 1024 * 1024 * 1024,
            "安装包大小不受支持"
        );
        let asset_version = source["version"].as_str().context("安装包缺少版本号")?;
        ensure!(asset_version == version, "{name} 与发布版本不一致");
        assets.push(Asset {
            platform: platform.into(),
            architecture: arch.into(),
            name: name.into(),
            version: version.into(),
            size,
            sha256: sha,
        });
    }
    Ok(Release {
        accelerated:false,
        version: version.into(),
        tag: tag.into(),
        checked_at: now(),
        assets,
    })
}
impl Downloads {
    pub fn new(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root)?;
        let cache = fs::read(root.join("release-cache.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Cache>(&b).ok())
            .unwrap_or_default();
        let mut task = fs::read(root.join("download-task.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Task>(&b).ok());
        if let Some(t) = &mut task {
            if t.active() {
                t.stage = "failed".into();
                t.cancellable = false;
                t.path = None;
                t.error = Some("上次下载被中断，请重新下载；未完成文件不会作为安装包使用".into());
            }
            if t.stage == "ready" {
                t.stage = "unverified".into();
                t.error = Some("上次下载已保存，重新验证 SHA256 后可打开所在目录".into());
                t.cancellable = false;
            }
        }
        Ok(Self {
            channel:Mutex::new(fs::read(root.join("download-channel.json")).ok().and_then(|b|serde_json::from_slice::<ChannelConfig>(&b).ok()).unwrap_or_default()),
            root,
            client: client()?,
            cache: Mutex::new(cache),
            task: Mutex::new(task),
            cancel: AtomicBool::new(false),
            check_lock: tokio::sync::Mutex::new(()),
        })
    }
    pub fn state(&self) -> DownloadState {
        let cache = self.cache.lock().unwrap();
        DownloadState {
            channel:self.channel.lock().unwrap().clone(),
            release: cache.release.clone(),
            task: self.task.lock().unwrap().clone(),
            last_error: cache.last_error.clone(),
            retry_at: cache.retry_at,
            stale: cache
                .release
                .as_ref()
                .is_some_and(|r| now().saturating_sub(r.checked_at) >= CACHE_TTL),
        }
    }
    pub fn configure_channel(&self,config:ChannelConfig)->Result<DownloadState>{
        ensure!(!self.task.lock().unwrap().as_ref().is_some_and(Task::active),"请先取消或完成当前下载再切换渠道");
        let _check=self.check_lock.try_lock().map_err(|_|anyhow::anyhow!("版本检查正在进行"))?;
        if config.accelerated{config.user_agent()?;}
        atomic_json(&self.root.join("download-channel.json"),&config)?;
        *self.channel.lock().unwrap()=config;*self.cache.lock().unwrap()=Cache::default();self.save_cache()?;Ok(self.state())
    }
    fn channel_client(&self)->Result<Client>{let config=self.channel.lock().unwrap().clone();if config.accelerated{accelerated_client(&config)}else{Ok(self.client.clone())}}
    fn save_cache(&self) -> Result<()> {
        atomic_json(
            &self.root.join("release-cache.json"),
            &*self.cache.lock().unwrap(),
        )
    }
    async fn metadata(&self, url: String) -> Result<Vec<u8>> {
        tokio::time::timeout(Duration::from_secs(45), async {
            let mut response = self
                .channel_client()?
                .get(&url)
                .send()
                .await
                .context("无法连接镜像 GitHub 发布页，请检查网络后重试")?;
            self.check_response(&response)?;
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await? {
                ensure!(
                    bytes.len() + chunk.len() <= META_LIMIT,
                    "发布信息超过大小限制"
                );
                bytes.extend_from_slice(&chunk);
            }
            Ok(bytes)
        })
        .await
        .context("发布信息查询超时，请稍后重试")?
    }
    fn check_response(&self, response: &reqwest::Response) -> Result<()> {
        let status = response.status();
        if status == StatusCode::TOO_MANY_REQUESTS || status == StatusCode::FORBIDDEN {
            let wait = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1800)
                .clamp(60, 86400);
            self.cache.lock().unwrap().retry_at = now() + wait;
            bail!(
                "镜像返回 HTTP {}，访问受限；请等待冷却结束或打开发布页",
                status.as_u16()
            );
        }
        ensure!(
            status.is_success(),
            "镜像请求失败（HTTP {}），请稍后重试",
            status.as_u16()
        );
        Ok(())
    }
    pub async fn check(&self, manual: bool) -> Result<DownloadState> {
        let _gate = self.check_lock.lock().await;
        {
            let mut cache = self.cache.lock().unwrap();
            if now() < cache.retry_at {
                bail!("更新查询仍在冷却中，请稍后重试");
            }
            if let Some(release) = &cache.release {
                if !manual && now().saturating_sub(release.checked_at) < CACHE_TTL {
                    drop(cache);
                    return Ok(self.state());
                }
            }
            if cache.last_attempt > 0 && now().saturating_sub(cache.last_attempt) < MANUAL_COOLDOWN
            {
                drop(cache);
                return Ok(self.state());
            }
            cache.last_attempt = now();
        }
        // 先固化最新标签，然后安装包、manifest、checksums 都从该标签获取。
        let result = async {
            if self.channel.lock().unwrap().accelerated {
                let manifest=self.metadata(format!("{MIRROR}/latest/manifest")).await?;
                let checksums=self.metadata(format!("{MIRROR}/latest/checksums")).await?;
                let value:serde_json::Value=serde_json::from_slice(&manifest)?;
                let tag=value.get("releaseTag").and_then(|v|v.as_str()).map(str::to_owned).unwrap_or_else(||format!("claude-app-v{}",value["version"].as_str().unwrap_or_default()));
                let mut release=parse_release(&tag,&manifest,std::str::from_utf8(&checksums)?)?;release.accelerated=true;return Ok(release);
            }
            let response = tokio::time::timeout(
                Duration::from_secs(30),
                self.client.get(format!("{REPO}/releases/latest")).send(),
            )
            .await
            .context("获取最新发布超时")??;
            self.check_response(&response)?;
            let tag = response
                .url()
                .path()
                .strip_prefix("/Wangnov/claude-app-mirror/releases/tag/")
                .context("发布页没有返回有效标签")?
                .to_string();
            ensure!(valid_tag(&tag), "最新发布标签格式不受支持");
            drop(response);
            let base = format!("{REPO}/releases/download/{tag}");
            let manifest = self
                .metadata(format!("{base}/release-manifest.json"))
                .await?;
            let checksums = self.metadata(format!("{base}/SHA256SUMS.txt")).await?;
            parse_release(&tag, &manifest, std::str::from_utf8(&checksums)?)
        }
        .await;
        match result {
            Ok(release) => {
                let mut cache = self.cache.lock().unwrap();
                cache.release = Some(release);
                cache.last_error = None;
                cache.retry_at = 0;
            }
            Err(error) => {
                let message = format!("{error:#}");
                let mut cache = self.cache.lock().unwrap();
                cache.last_error = Some(message);
                cache.retry_at = cache.retry_at.max(now() + 60);
            }
        }
        self.save_cache()?;
        Ok(self.state())
    }
    pub fn reserve(&self, platform: &str, architecture: &str) -> Result<(Task, Release, Asset)> {
        let release = self
            .cache
            .lock()
            .unwrap()
            .release
            .clone()
            .context("请先检查版本并获取经过校验的发布清单")?;
        ensure!(!self.state().stale, "版本清单已过期，请先检查更新");
        ensure!(release.accelerated==self.channel.lock().unwrap().accelerated,"下载渠道已变化，请重新检查版本");
        let asset = release
            .assets
            .iter()
            .find(|a| a.platform == platform && a.architecture == architecture)
            .cloned()
            .context("所选平台或架构没有安装包")?;
        let expected_name = match (platform, architecture) {
            ("windows", "x64") => "Claude-win-x64.msix",
            ("windows", "arm64") => "Claude-win-arm64.msix",
            ("macos", "universal") => "Claude-mac-universal.dmg",
            ("linux", "x64") => "Claude-linux-x64.deb",
            ("linux", "arm64") => "Claude-linux-arm64.deb",
            _ => bail!("不支持的平台与架构"),
        };
        ensure!(
            valid_tag(&release.tag)
                && valid_version(&release.version)
                && asset.name == expected_name
                && valid_hash(&asset.sha256)
                && asset.size > 0
                && asset.size <= 8 * 1024 * 1024 * 1024,
            "缓存清单无效，请重新检查版本"
        );
        let mut current = self.task.lock().unwrap();
        ensure!(
            !current.as_ref().is_some_and(Task::active),
            "已有下载或校验任务正在进行"
        );
        let task = Task {
            id: uuid::Uuid::new_v4().to_string(),
            stage: "connecting".into(),
            asset_name: asset.name.clone(),
            version: release.version.clone(),
            total: asset.size,
            sha256: asset.sha256.clone(),
            cancellable: true,
            ..Default::default()
        };
        atomic_json(&self.root.join("download-task.json"), &task)?;
        self.cancel.store(false, Ordering::Release);
        *current = Some(task.clone());
        Ok((task, release, asset))
    }
    pub fn cancel(&self, id: &str) -> Result<()> {
        let task = self.task.lock().unwrap();
        let t = task.as_ref().context("没有进行中的任务")?;
        ensure!(
            t.id == id && t.active() && t.cancellable,
            "任务已结束或正在提交，暂时不可取消"
        );
        self.cancel.store(true, Ordering::Release);
        Ok(())
    }
    fn publish(&self, task: &Task, notify: &impl Fn(Task)) {
        *self.task.lock().unwrap() = Some(task.clone());
        notify(task.clone());
    }
    async fn cancellable<T>(&self, future: impl std::future::Future<Output = T>) -> Result<T> {
        tokio::pin!(future);
        let deadline = tokio::time::sleep(Duration::from_secs(30));
        tokio::pin!(deadline);
        loop {
            tokio::select! {
                result=&mut future=>return Ok(result),
                _=&mut deadline=>bail!("网络连续 30 秒没有响应，请重试"),
                _=tokio::time::sleep(Duration::from_millis(80))=>ensure!(!self.cancel.load(Ordering::Acquire),"下载已取消"),
            }
        }
    }
    pub async fn run(
        &self,
        task: Task,
        release: Release,
        asset: Asset,
        notify: impl Fn(Task),
    ) -> Task {
        let url = if release.accelerated {
            let suffix=match (asset.platform.as_str(),asset.architecture.as_str()) {("windows","x64")=>"win-x64",("windows","arm64")=>"win-arm64",("macos","universal")=>"mac",("linux","x64")=>"linux-x64",("linux","arm64")=>"linux-arm64",_=>"invalid"};
            format!("{MIRROR}/latest/{suffix}")
        }else{format!("{REPO}/releases/download/{}/{}", release.tag, asset.name)};
        self.transfer(task, release, asset, url, notify).await
    }
    async fn transfer(
        &self,
        mut task: Task,
        release: Release,
        asset: Asset,
        url: String,
        notify: impl Fn(Task),
    ) -> Task {
        let dir = self.root.join("downloads").join(&release.tag);
        let part = dir.join(format!("{}.part", task.id));
        let result = async {
            ensure!(valid_tag(&release.tag), "版本标签无效");
            fs::create_dir_all(&dir)?;
            let mut response = {
                let mut attempt = 0;
                loop {
                    let result = self.cancellable(self.channel_client()?.get(&url).send()).await?;
                    match result {
                        Ok(response) => break response,
                        Err(_) if attempt < 2 => {
                            attempt += 1;
                            self.cancellable(tokio::time::sleep(Duration::from_secs(1 << attempt)))
                                .await?;
                        }
                        Err(error) => {
                            return Err(error).context("安装包连接失败，请检查网络或稍后重试")
                        }
                    }
                }
            };
            self.check_response(&response)?;
            if let Some(length) = response.content_length() {
                ensure!(length == asset.size, "服务器文件大小与发布清单不一致");
            }
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&part)?;
            task.stage = "downloading".into();
            self.publish(&task, &notify);
            let mut last = std::time::Instant::now();
            while let Some(bytes) = self.cancellable(response.chunk()).await?? {
                ensure!(!self.cancel.load(Ordering::Acquire), "下载已取消");
                ensure!(
                    task.downloaded + bytes.len() as u64 <= asset.size,
                    "下载数据超过预期大小"
                );
                file.write_all(&bytes)
                    .context("写入失败，请检查磁盘空间和目录权限")?;
                task.downloaded += bytes.len() as u64;
                if last.elapsed() >= Duration::from_millis(150) {
                    self.publish(&task, &notify);
                    last = std::time::Instant::now();
                }
            }
            ensure!(
                task.downloaded == asset.size,
                "下载不完整：实际大小与清单不一致"
            );
            file.sync_all()?;
            drop(file);
            task.stage = "verifying".into();
            self.publish(&task, &notify);
            let hash = hash_file(&part, &self.cancel)?;
            ensure!(
                hash == asset.sha256,
                "SHA256 校验失败，文件不能用于安装，请重新下载"
            );
            {
                // 与 cancel 使用同一把锁，避免接受取消后仍提交文件。
                let mut current = self.task.lock().unwrap();
                ensure!(!self.cancel.load(Ordering::Acquire), "下载已取消");
                task.stage = "committing".into();
                task.cancellable = false;
                *current = Some(task.clone());
            }
            notify(task.clone());
            let final_path = dir.join(format!("{}-{}", task.id, asset.name));
            fs::rename(&part, &final_path)?;
            task.path = Some(final_path);
            task.stage = "ready".into();
            task.error = None;
            Ok::<(), anyhow::Error>(())
        }
        .await;
        if let Err(error) = result {
            task.stage = if self.cancel.load(Ordering::Acquire) {
                "cancelled"
            } else {
                "failed"
            }
            .into();
            task.path = None;
            task.error = Some(format!("{error:#}"));
        }
        task.cancellable = false;
        if part.is_file() {
            let _ = fs::remove_file(&part);
        }
        if let Err(error) = atomic_json(&self.root.join("download-task.json"), &task) {
            task.stage = "failed".into();
            task.error = Some(format!("保存任务记录失败：{error}"));
        }
        let _ = self.save_cache();
        self.publish(&task, &notify);
        task
    }
    pub fn verify_saved(&self) -> Result<Task> {
        let mut current = self.task.lock().unwrap();
        let mut task = current.clone().context("暂无下载记录")?;
        ensure!(!task.active(), "请等待当前下载完成");
        let path = task.path.as_ref().context("没有已完成的文件")?;
        let result = (|| {
            let canonical = path.canonicalize()?;
            ensure!(
                canonical.starts_with(self.root.join("downloads").canonicalize()?),
                "下载路径不在管理目录"
            );
            ensure!(
                fs::metadata(&canonical)?.len() == task.total,
                "文件大小已变化"
            );
            ensure!(
                valid_hash(&task.sha256)
                    && hash_file(&canonical, &AtomicBool::new(false))? == task.sha256,
                "SHA256 不匹配，文件已损坏或被修改"
            );
            Ok::<(), anyhow::Error>(())
        })();
        if let Err(error) = result {
            task.stage = "failed".into();
            task.path = None;
            task.error = Some(error.to_string());
        } else {
            task.stage = "ready".into();
            task.error = None;
        }
        atomic_json(&self.root.join("download-task.json"), &task)?;
        *current = Some(task.clone());
        Ok(task)
    }
    pub fn ready_path(&self) -> Result<PathBuf> {
        let task = self.verify_saved()?;
        ensure!(task.stage == "ready", "文件尚未通过完整性校验");
        task.path.context("下载路径丢失")
    }
}
fn hash_file(path: &Path, cancel: &AtomicBool) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut bytes = vec![0u8; 1024 * 1024];
    loop {
        ensure!(!cancel.load(Ordering::Acquire), "下载已取消");
        let read = file.read(&mut bytes)?;
        if read == 0 {
            break;
        }
        hash.update(&bytes[..read]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

#[cfg(test)]
mod tests;
