//! 仅复制和验证本地界面 ASAR，不修改安装、不制作可执行副本、不启动 Claude。
fn main() -> anyhow::Result<()> {
    assistant_core::localization::probe_local_resources()
}
