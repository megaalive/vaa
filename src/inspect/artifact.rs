use std::path::Path;

use object::{Object, ObjectSegment, SegmentFlags};
use serde::{Deserialize, Serialize};

const PF_X: u32 = 1;
const PF_W: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactInfo {
    pub path: String,
    pub size_bytes: u64,
    pub format: String,
    pub architecture: String,
    pub is_executable: bool,
    pub has_executable_stack: bool,
    pub has_wxorx: bool,
    pub symbol_count: usize,
    pub import_count: usize,
    pub section_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum InspectError {
    #[error("failed to read file: {0}")]
    ReadFailed(String),
    #[error("unknown or unsupported object format: {0}")]
    UnknownFormat(String),
    #[error("parsing failed: {0}")]
    ParseFailed(String),
}

pub struct ArtifactInspector;

impl ArtifactInspector {
    pub fn inspect(path: &Path) -> Result<ArtifactInfo, InspectError> {
        let data =
            std::fs::read(path).map_err(|e| InspectError::ReadFailed(e.to_string()))?;

        let size_bytes = data.len() as u64;

        let file_format = object::FileKind::parse(&data[..]).map_err(|e| {
            InspectError::ParseFailed(format!("cannot determine format: {e}"))
        })?;

        let (format_name, architecture, is_executable, has_exec, has_wx, sym_count, imp_count, sec_count, warnings) =
            Self::read_object(&data, &file_format);

        Ok(ArtifactInfo {
            path: path.to_string_lossy().to_string(),
            size_bytes,
            format: format_name,
            architecture,
            is_executable,
            has_executable_stack: has_exec,
            has_wxorx: has_wx,
            symbol_count: sym_count,
            import_count: imp_count,
            section_count: sec_count,
            warnings,
        })
    }

    #[allow(clippy::too_many_lines)]
    fn read_object(
        data: &[u8],
        kind: &object::FileKind,
    ) -> (String, String, bool, bool, bool, usize, usize, usize, Vec<String>) {
        let format_name = format!("{kind:?}");
        let mut architecture = "unknown".to_owned();
        let mut is_executable = false;
        let mut has_executable_stack = false;
        let mut has_wxorx = false;
        let mut symbol_count = 0;
        let mut import_count = 0;
        let mut section_count = 0;
        let mut warnings = Vec::new();

        match object::read::File::parse(data) {
            Ok(obj_file) => {
                architecture = format!("{:?}", obj_file.architecture());
                is_executable = obj_file.is_little_endian();
                section_count = obj_file.sections().count();

                for segment in obj_file.segments() {
                    let flags = segment.flags();
                    let (w, x) = Self::segment_wx(&flags);
                    if w && x {
                        has_wxorx = true;
                    }
                }

                symbol_count = obj_file.symbols().count();

                if let Ok(imports) = obj_file.imports() {
                    import_count = imports.len();
                }

                is_executable = Self::is_executable_type(data, kind);
                has_executable_stack = Self::has_gnu_stack_exec(data, kind);
            }
            Err(e) => {
                warnings.push(format!("object parse: {e}"));
            }
        }

        (
            format_name,
            architecture,
            is_executable,
            has_executable_stack,
            has_wxorx,
            symbol_count,
            import_count,
            section_count,
            warnings,
        )
    }

    fn segment_wx(flags: &SegmentFlags) -> (bool, bool) {
        match flags {
            SegmentFlags::Elf { p_flags } => {
                let w = (*p_flags & PF_W) != 0;
                let x = (*p_flags & PF_X) != 0;
                (w, x)
            }
            SegmentFlags::Coff { characteristics } => {
                let w = (*characteristics & 0x8000_0000) != 0;
                let x = (*characteristics & 0x2000_0000) != 0;
                (w, x)
            }
            SegmentFlags::MachO { flags, .. } => {
                let w = (*flags & 0x02) != 0;
                let x = (*flags & 0x04) != 0;
                (w, x)
            }
            _ => (false, false),
        }
    }

    fn is_executable_type(data: &[u8], kind: &object::FileKind) -> bool {
        match kind {
            object::FileKind::Elf32 | object::FileKind::Elf64 => {
                if data.len() < 18 {
                    return false;
                }
                data[16] == 2
            }
            object::FileKind::Pe32 | object::FileKind::Pe64 => true,
            object::FileKind::MachOFat32 | object::FileKind::MachO32 | object::FileKind::MachO64 => {
                true
            }
            _ => false,
        }
    }

    fn has_gnu_stack_exec(data: &[u8], kind: &object::FileKind) -> bool {
        match kind {
            object::FileKind::Elf64 => {
                if data.len() < 64 {
                    return false;
                }
                let e_phoff = u64::from_ne_bytes(
                    data[32..40].try_into().unwrap_or([0; 8]),
                );
                let e_phentsize = u16::from_ne_bytes(
                    data[54..56].try_into().unwrap_or([0; 2]),
                );
                let e_phnum =
                    u16::from_ne_bytes(data[60..62].try_into().unwrap_or([0; 2]));
                let phent_size = e_phentsize as usize;

                for i in 0..e_phnum {
                    let offset = (e_phoff as usize) + (i as usize) * phent_size;
                    if offset + 16 > data.len() {
                        break;
                    }
                    let p_type = u32::from_ne_bytes(
                        data[offset..offset + 4].try_into().unwrap_or([0; 4]),
                    );
                    if p_type == 0x6474e551 {
                        let p_flags = u64::from_ne_bytes(
                            data[offset + 4..offset + 12]
                                .try_into()
                                .unwrap_or([0; 8]),
                        );
                        return (p_flags as u32 & PF_X) != 0;
                    }
                }
                false
            }
            object::FileKind::Elf32 => {
                if data.len() < 48 {
                    return false;
                }
                let e_phoff = u32::from_ne_bytes(
                    data[28..32].try_into().unwrap_or([0; 4]),
                );
                let e_phentsize = u16::from_ne_bytes(
                    data[42..44].try_into().unwrap_or([0; 2]),
                );
                let e_phnum =
                    u16::from_ne_bytes(data[44..46].try_into().unwrap_or([0; 2]));
                let phent_size = e_phentsize as usize;

                for i in 0..e_phnum {
                    let offset = (e_phoff as usize) + (i as usize) * phent_size;
                    if offset + 8 > data.len() {
                        break;
                    }
                    let p_type = u32::from_ne_bytes(
                        data[offset..offset + 4].try_into().unwrap_or([0; 4]),
                    );
                    if p_type == 0x6474e551 {
                        let p_flags = u32::from_ne_bytes(
                            data[offset + 4..offset + 8]
                                .try_into()
                                .unwrap_or([0; 4]),
                        );
                        return (p_flags & PF_X) != 0;
                    }
                }
                false
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn nonexistent_file_returns_error() {
        let result = ArtifactInspector::inspect(Path::new("nonexistent.xyz"));
        assert!(matches!(result, Err(InspectError::ReadFailed(_))));
    }

    #[test]
    fn empty_data_returns_error() {
        let dir = std::env::temp_dir();
        let path = dir.join("empty_test_file.bin");
        std::fs::write(&path, []).expect("write");
        let result = ArtifactInspector::inspect(&path);
        let _ = std::fs::remove_file(&path);
        assert!(result.is_err());
    }
}
