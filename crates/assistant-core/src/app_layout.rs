//! Layout shared by preparation, launch and recovery. Receipts and runtime stay outside .app.
use std::path::{Path, PathBuf};

pub(crate) struct CopyLayout {
    pub exe: PathBuf,
    pub resources: PathBuf,
    pub fuse: PathBuf,
    pub bundle: Option<PathBuf>,
}
impl CopyLayout {
    pub fn at(root: &Path) -> Self {
        let bundle = root.join("Claude.app");
        if bundle.is_dir() {
            Self::mac(&bundle)
        } else {
            Self {
                exe: root.join("Claude.exe"),
                resources: root.join("resources"),
                fuse: root.join("Claude.exe"),
                bundle: None,
            }
        }
    }
    pub fn mac(bundle: &Path) -> Self {
        Self {
            exe: bundle.join("Contents/MacOS/Claude"),
            resources: bundle.join("Contents/Resources"),
            fuse: bundle.join(
                "Contents/Frameworks/Electron Framework.framework/Versions/A/Electron Framework",
            ),
            bundle: Some(bundle.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mac_fuses_belong_to_framework_and_windows_fuses_to_executable() {
        let dir = tempfile::tempdir().unwrap();
        let windows = CopyLayout::at(dir.path());
        assert_eq!(windows.exe, windows.fuse);
        std::fs::create_dir(dir.path().join("Claude.app")).unwrap();
        let mac = CopyLayout::at(dir.path());
        assert_ne!(mac.exe, mac.fuse);
        assert!(mac.resources.ends_with("Contents/Resources"));
    }
}
