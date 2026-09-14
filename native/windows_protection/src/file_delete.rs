//! 呼出し側が排他的に開いて検証したfileの削除予定を設定する。
use std::os::windows::io::AsRawHandle;
use windows_sys::Win32::Storage::FileSystem::{
    FileDispositionInfo, SetFileInformationByHandle, FILE_DISPOSITION_INFO,
};

/// fileの取得権限・対象検証・監査は呼出し側が担う。閉じるまでfileは保持される。
pub fn mark(file: &std::fs::File) -> std::io::Result<()> {
    let info = FILE_DISPOSITION_INFO { DeleteFile: true };
    // SAFETY: 借用fileのhandleは呼出し中生存する。固定構造体は正しい長さで生存し、OSは保持しない。
    let ok = unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle(),
            FileDispositionInfo,
            (&info as *const FILE_DISPOSITION_INFO).cast(),
            std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    };
    if ok == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
