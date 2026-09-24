//! 针对**随仓库分发的二进制**的完整性检查。
//!
//! 打包/安装都依赖 `lib/` 下的第三方二进制，CI 上跑这个能在漏提交文件时立刻失败。

use std::path::PathBuf;

fn lib_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib")
}

fn read_bundled(name: &str) -> Vec<u8> {
    let path = lib_dir().join(name);
    std::fs::read(&path).unwrap_or_else(|err| panic!("missing {}: {err}", path.display()))
}

fn assert_pe(bytes: &[u8], name: &str) {
    assert!(bytes.len() > 1024, "{name} looks truncated");
    assert_eq!(&bytes[..2], b"MZ", "{name} is not a PE image");
}

#[test]
fn lhm_native_library_is_bundled_and_is_a_pe_dll() {
    let bytes = read_bundled("LhmNative.dll");
    assert_pe(&bytes, "LhmNative.dll");
}

#[test]
fn pawnio_setup_is_bundled_and_is_a_pe_executable() {
    let bytes = read_bundled("PawnIO_setup.exe");
    assert_pe(&bytes, "PawnIO_setup.exe");
}

#[test]
fn installer_assets_are_present() {
    let installer = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("installer");
    for name in ["pulse.nsi", "pulse.ico", "welcome.bmp", "build.bat"] {
        let path = installer.join(name);
        assert!(path.is_file(), "missing {}", path.display());
    }
}
