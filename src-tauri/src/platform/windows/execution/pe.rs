use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const DOS_HEADER_LENGTH: usize = 64;
const PE_SIGNATURE_LENGTH: usize = 4;
const FILE_HEADER_LENGTH: usize = 20;
const OPTIONAL_HEADER_SUBSYSTEM_OFFSET: usize = 68;
const PE_HEADER_LENGTH: usize = PE_SIGNATURE_LENGTH
    + FILE_HEADER_LENGTH
    + OPTIONAL_HEADER_SUBSYSTEM_OFFSET
    + std::mem::size_of::<u16>();
const IMAGE_FILE_MACHINE_I386: u16 = 0x014c;
const IMAGE_FILE_MACHINE_AMD64: u16 = 0x8664;
const IMAGE_FILE_MACHINE_ARM64: u16 = 0xAA64;
const IMAGE_SUBSYSTEM_WINDOWS_CUI: u16 = 3;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AppArchitecture {
    X86,
    X64,
    Arm64,
    NotApplicable,
    #[default]
    Unknown,
}

fn read_pe_header(path: &Path) -> Option<[u8; PE_HEADER_LENGTH]> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut dos = [0_u8; DOS_HEADER_LENGTH];
    file.read_exact(&mut dos).ok()?;
    if &dos[0..2] != b"MZ" {
        return None;
    }

    let pe_offset = u32::from_le_bytes([dos[60], dos[61], dos[62], dos[63]]) as u64;
    file.seek(SeekFrom::Start(pe_offset)).ok()?;
    let mut header = [0_u8; PE_HEADER_LENGTH];
    file.read_exact(&mut header).ok()?;
    (&header[0..PE_SIGNATURE_LENGTH] == b"PE\0\0").then_some(header)
}

pub(crate) fn read_architecture(path: &Path) -> AppArchitecture {
    let Some(header) = read_pe_header(path) else {
        return AppArchitecture::Unknown;
    };
    let machine = u16::from_le_bytes([header[4], header[5]]);
    match machine {
        IMAGE_FILE_MACHINE_I386 => AppArchitecture::X86,
        IMAGE_FILE_MACHINE_AMD64 => AppArchitecture::X64,
        IMAGE_FILE_MACHINE_ARM64 => AppArchitecture::Arm64,
        _ => AppArchitecture::Unknown,
    }
}

pub(crate) fn is_console_subsystem(path: &Path) -> bool {
    let Some(header) = read_pe_header(path) else {
        return false;
    };
    let offset = PE_SIGNATURE_LENGTH + FILE_HEADER_LENGTH + OPTIONAL_HEADER_SUBSYSTEM_OFFSET;
    u16::from_le_bytes([header[offset], header[offset + 1]]) == IMAGE_SUBSYSTEM_WINDOWS_CUI
}

#[cfg(test)]
mod tests {
    use super::{read_architecture, AppArchitecture};

    fn minimal_pe(machine: u16, subsystem: u16) -> Vec<u8> {
        let mut buffer = vec![0_u8; 64];
        buffer[0..2].copy_from_slice(b"MZ");
        buffer[60..64].copy_from_slice(&64_u32.to_le_bytes());
        buffer.extend_from_slice(b"PE\0\0");
        buffer.extend_from_slice(&machine.to_le_bytes());
        buffer.extend_from_slice(&[0_u8; 18]);
        buffer.extend_from_slice(&[0_u8; 70]);
        buffer[64 + 4 + 20 + 68..64 + 4 + 20 + 70].copy_from_slice(&subsystem.to_le_bytes());
        buffer
    }

    fn write_fixture(name: &str, contents: &[u8]) -> tempfile::NamedTempFile {
        let file = tempfile::Builder::new().suffix(name).tempfile().unwrap();
        std::fs::write(file.path(), contents).unwrap();
        file
    }

    #[test]
    fn reads_supported_pe_machine_types() {
        let x86 = write_fixture("-x86.exe", &minimal_pe(0x014c, 2));
        let x64 = write_fixture("-x64.exe", &minimal_pe(0x8664, 2));
        let arm64 = write_fixture("-arm64.exe", &minimal_pe(0xAA64, 2));

        assert_eq!(read_architecture(x86.path()), AppArchitecture::X86);
        assert_eq!(read_architecture(x64.path()), AppArchitecture::X64);
        assert_eq!(read_architecture(arm64.path()), AppArchitecture::Arm64);
    }

    #[test]
    fn rejects_malformed_or_missing_pe_files() {
        let bad_dos = write_fixture("-bad-dos.exe", b"not a pe");
        let mut bad_signature = minimal_pe(0x8664, 2);
        bad_signature[64..68].copy_from_slice(b"PX\0\0");
        let bad_signature = write_fixture("-bad-signature.exe", &bad_signature);
        let missing = bad_dos.path().with_file_name("missing.exe");

        assert_eq!(read_architecture(bad_dos.path()), AppArchitecture::Unknown);
        assert_eq!(
            read_architecture(bad_signature.path()),
            AppArchitecture::Unknown
        );
        assert_eq!(read_architecture(&missing), AppArchitecture::Unknown);
    }

    #[test]
    fn recognizes_the_console_subsystem_and_rejects_gui_and_bad_files() {
        let console = write_fixture("-console.exe", &minimal_pe(0x8664, 3));
        let gui = write_fixture("-gui.exe", &minimal_pe(0x8664, 2));
        let junk = write_fixture("-junk.exe", b"not a pe file");

        assert!(super::is_console_subsystem(console.path()));
        assert!(!super::is_console_subsystem(gui.path()));
        assert!(!super::is_console_subsystem(junk.path()));
    }
}
