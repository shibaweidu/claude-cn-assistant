use super::*;
fn info() -> PackageInfo {
    PackageInfo {
        name: "Claude".into(),
        publisher: PUBLISHER.into(),
        version: "6.20.1.0".into(),
        architecture: "x64".into(),
        signature_status: "Valid".into(),
        signer_matches: true,
    }
}
fn env() -> Environment {
    Environment {
        architecture: "x64".into(),
        ..Default::default()
    }
}
fn package(version: &str) -> RegisteredPackage {
    RegisteredPackage {
        name: "Claude".into(),
        publisher: PUBLISHER.into(),
        version: version.into(),
        architecture: "x64".into(),
        family: "Claude_pzs8sxrjxfjjc".into(),
        status: "Ok".into(),
    }
}
#[cfg(windows)]
#[test]
fn validates_identity_signature_architecture_and_version() {
    let base = info();
    let environment = env();
    assert!(validate_package(&base, "6.20.1", "x64", &environment).is_ok());
    for field in [
        "name",
        "publisher",
        "signature",
        "signer",
        "architecture",
        "version",
    ] {
        let mut bad = base.clone();
        match field {
            "name" => bad.name = "Other".into(),
            "publisher" => bad.publisher = "CN=Fake".into(),
            "signature" => bad.signature_status = "NotTrusted".into(),
            "signer" => bad.signer_matches = false,
            "architecture" => bad.architecture = "arm64".into(),
            _ => bad.version = "6.21.0.0".into(),
        }
        assert!(
            validate_package(&bad, "6.20.1", "x64", &environment).is_err(),
            "{field}"
        );
    }
}
#[cfg(windows)]
#[test]
fn running_and_downgrade_are_blocked() {
    let mut environment = env();
    environment.running = true;
    assert!(validate_package(&info(), "6.20.1", "x64", &environment).is_err());
    environment.running = false;
    environment.packages.push(package("6.21.0.0"));
    assert!(validate_package(&info(), "6.20.1", "x64", &environment).is_err());
    environment.packages[0].version = "6.19.0.0".into();
    assert!(validate_package(&info(), "6.20.1", "x64", &environment).is_ok());
}
#[test]
fn actual_registration_is_required_to_confirm_success() {
    let mut environment = env();
    assert!(!installation_confirmed(&info(), &environment));
    environment.packages.push(package("6.20.0.0"));
    assert!(!installation_confirmed(&info(), &environment));
    environment.packages[0].version = "6.20.1.0".into();
    assert!(installation_confirmed(&info(), &environment));
    environment.packages[0].version = "6.21.0.0".into();
    assert!(installation_confirmed(&info(), &environment));
    environment.packages[0].status = "Modified".into();
    assert!(!installation_confirmed(&info(), &environment));
    environment.packages[0].status = "Ok".into();
    environment.packages[0].family = "Different".into();
    assert!(!installation_confirmed(&info(), &environment));
}

#[cfg(windows)]
#[test]
fn windows_rejects_unsigned_msix_fixture() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("未签名 test.msix");
    fs::write(
        &path,
        include_bytes!("../../../../tests/fixtures/unsigned-fixture.msix"),
    )
    .unwrap();
    let inspected = inspect_package(&path).unwrap();
    assert_eq!(inspected.name, "Claude");
    assert_eq!(inspected.publisher, "CN=Fake");
    assert_ne!(inspected.signature_status, "Valid");
    assert!(!inspected.signer_matches);
    assert!(validate_package(&inspected, "6.20.1", "x64", &env()).is_err());
}
#[test]
fn interrupted_install_restarts_unconfirmed() {
    let dir = tempfile::tempdir().unwrap();
    let service = Installer::new(dir.path().into()).unwrap();
    let id = service.reserve().unwrap();
    assert!(service.reserve().is_err());
    service
        .set(InstallState {
            id,
            stage: "awaitingInstaller".into(),
            info: Some(info()),
            ..Default::default()
        })
        .unwrap();
    let restarted = Installer::new(dir.path().into()).unwrap();
    assert_eq!(restarted.state().stage, "interrupted");
    assert_eq!(restarted.stop().unwrap().stage, "stopped");
    assert!(restarted.reserve().is_ok());
}
#[test]
fn unverified_or_other_platform_file_cannot_be_installed() {
    let dir = tempfile::tempdir().unwrap();
    let downloads = Downloads::new(dir.path().into()).unwrap();
    let service = Installer::new(dir.path().into()).unwrap();
    let id = service.reserve().unwrap();
    let state = service.prepare(&downloads, &id).unwrap();
    assert_eq!(state.stage, "failed");
    assert!(state.info.is_none());
    assert!(service.launch(&downloads, &id, false).is_err());
}

#[cfg(windows)]
#[test]
fn signer_comparison_accepts_equivalent_x500_encodings() {
    let result: bool = probe(r#"
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
try {
 $flags=[Security.Cryptography.X509Certificates.X500DistinguishedNameFlags]
 $name='CN="Anthropic, PBC", O="Anthropic, PBC", C=US'
 $a=[Security.Cryptography.X509Certificates.X500DistinguishedName]::new($name,$flags::ForceUTF8Encoding)
 $b=[Security.Cryptography.X509Certificates.X500DistinguishedName]::new($name,$flags::None)
 if([Convert]::ToBase64String($a.RawData) -ceq [Convert]::ToBase64String($b.RawData)){throw '测试没有产生不同编码'}
 $equal=[string]::Equals($a.Decode($flags::None),$b.Decode($flags::None),[StringComparison]::Ordinal)
 $other=[Security.Cryptography.X509Certificates.X500DistinguishedName]::new('CN=Other, O="Anthropic, PBC", C=US')
 $different=-not [string]::Equals($a.Decode($flags::None),$other.Decode($flags::None),[StringComparison]::Ordinal)
 @{ok=$true;data=($equal -and $different)}|ConvertTo-Json -Compress
} catch {@{ok=$false;error=$_.Exception.Message}|ConvertTo-Json -Compress}
"#, None).unwrap();
    assert!(result);
}
