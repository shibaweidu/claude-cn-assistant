fn main() -> anyhow::Result<()> {
    let snapshot = assistant_core::snapshot(&assistant_core::data_dir()?)?;
    println!("{}", serde_json::to_string_pretty(&snapshot)?);
    Ok(())
}
