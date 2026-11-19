//! 確認済みfile handleを、確定済みdirectory handle内の未使用名へ移す。
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::{Dir, File, OpenOptions, OpenOptionsExt};
use std::io::{Error, ErrorKind, Result};
use std::mem::{offset_of, size_of};
use std::os::windows::io::AsRawHandle;
use windows_sys::Wdk::Storage::FileSystem::{
    FileRenameInformation, NtSetInformationFile, FILE_RENAME_INFORMATION,
};
use windows_sys::Win32::Foundation::{RtlNtStatusToDosError, HANDLE};
use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;

#[repr(C)]
struct RenameInfo {
    flags: u32,
    root: HANDLE,
    name_bytes: u32,
    name: [u16; 256],
}

const _: () = {
    assert!(offset_of!(RenameInfo, root) == offset_of!(FILE_RENAME_INFORMATION, RootDirectory));
    assert!(
        offset_of!(RenameInfo, name_bytes) == offset_of!(FILE_RENAME_INFORMATION, FileNameLength)
    );
    assert!(offset_of!(RenameInfo, name) == offset_of!(FILE_RENAME_INFORMATION, FileName));
};

/// 検証と移動の間、同じ排他的かつ同期I/Oのfile handleを保持する。
pub struct MoveFile(File);

impl MoveFile {
    pub fn open(directory: &Dir, name: &str) -> Result<Self> {
        validate_name(name)?;
        let mut options = OpenOptions::new();
        // GENERIC_READ | DELETE。FILE_FLAG_OVERLAPPEDは指定しない。
        options
            .read(true)
            .access_mode(0x80000000 | 0x00010000)
            .share_mode(0)
            .follow(FollowSymlinks::No);
        directory.open_with(name, &options).map(Self)
    }

    pub fn file(&self) -> &File {
        &self.0
    }

    /// 既存の宛先を置換しない。版の検証・認可・監査は呼出し側が担う。
    pub fn move_to(&self, directory: &Dir, name: &str) -> Result<()> {
        validate_name(name)?;
        let mut info = RenameInfo {
            flags: 0,
            root: directory.as_raw_handle(),
            name_bytes: 0,
            name: [0; 256],
        };
        for (index, unit) in name.encode_utf16().enumerate() {
            info.name[index] = unit;
            info.name_bytes += 2;
        }
        let mut completion = IO_STATUS_BLOCK::default();
        // SAFETY: openが同期I/Oのhandleを取得し、両handleと入出力構造体は返却まで生存する。
        // 配列の位置と長さはABIに従い、flags=0で既存の宛先を置換しない。
        let status = unsafe {
            NtSetInformationFile(
                self.0.as_raw_handle(),
                &mut completion,
                (&info as *const RenameInfo).cast(),
                size_of::<RenameInfo>() as u32,
                FileRenameInformation,
            )
        };
        if status != 0 {
            // SAFETY: 直前に得たNTSTATUSをエラー番号へ変換する。メモリへの参照を渡さない。
            return Err(Error::from_raw_os_error(
                unsafe { RtlNtStatusToDosError(status) } as i32,
            ));
        }
        Ok(())
    }
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 255
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':'))
    {
        return Err(Error::new(ErrorKind::InvalidInput, "移動する名前が不正"));
    }
    Ok(())
}
