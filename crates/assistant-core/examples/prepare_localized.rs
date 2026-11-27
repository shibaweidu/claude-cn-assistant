//! 显式执行受管副本制作，不启动客户端；运行中会拒绝。
fn main() -> anyhow::Result<()> {
    let service = assistant_core::localization::Localizer::new(assistant_core::data_dir()?)?;
    let state = service.prepare(false, |s| {
        println!("{} {} / {}", s.stage, s.copied, s.total)
    })?;
    println!("{}", state.message);
    anyhow::ensure!(state.stage == "prepared", "未完成副本制作");
    Ok(())
}
