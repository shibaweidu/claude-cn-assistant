//! 只读安装环境与 MSIX 校验探测，不打开安装器。
fn main() -> anyhow::Result<()> {
    let environment = assistant_core::installation::environment()?;
    println!("{}", serde_json::to_string_pretty(&environment)?);
    if let Some(path) = std::env::args_os().nth(1) {
        let info = assistant_core::installation::inspect_package(std::path::Path::new(&path))?;
        println!("{}", serde_json::to_string_pretty(&info)?);
    }
    Ok(())
}
