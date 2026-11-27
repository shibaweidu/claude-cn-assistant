//! 实际镜像联调。默认仅检查元数据；加 --download 才下载 Windows x64 安装包。
use assistant_core::downloads::Downloads;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let root = assistant_core::data_dir()?;
    let service = Downloads::new(root)?;
    let state = service.check(false).await?;
    if let Some(error) = state.last_error {
        anyhow::bail!(error);
    }
    let release = state
        .release
        .ok_or_else(|| anyhow::anyhow!("未获取发布信息"))?;
    println!(
        "release={} assets={}",
        release.version,
        release.assets.len()
    );
    if std::env::args().any(|s| s == "--download") {
        let (task, release, asset) = service.reserve("windows", "x64")?;
        let task = service
            .run(task, release, asset, |task| {
                println!("{} {} / {}", task.stage, task.downloaded, task.total)
            })
            .await;
        anyhow::ensure!(task.stage == "ready", "{}", task.error.unwrap_or_default());
        println!("verified={}", task.path.unwrap().display());
    }
    Ok(())
}
