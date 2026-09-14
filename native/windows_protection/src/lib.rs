//! Windowsの保護bytes変換と検証済みhandleの削除確定を担う。Permission、IPC、表示を所有しない。
#![cfg(windows)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::ptr::null_mut;
use windows_sys::Win32::Foundation::{GetLastError, LocalFree};
use windows_sys::Win32::Security::Cryptography::{
    CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
};

pub const MAX_PLAINTEXT: usize = 65536;
pub const MAX_CIPHERTEXT: usize = 131072;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    Windows(u32),
    InvalidOutput,
}

/// 平文の複製・Debug・Serializeを提供せず、所有するbytesをdrop時に消去する。
pub struct Secret(Vec<u8>);
impl Secret {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
impl Drop for Secret {
    fn drop(&mut self) {
        wipe(&mut self.0);
    }
}

fn wipe(bytes: &mut [u8]) {
    for byte in bytes {
        // SAFETY: 生存中の排他的slice内だけを書き、最適化による消去除去を防ぐ。
        unsafe {
            std::ptr::write_volatile(byte, 0);
        }
    }
}

struct Output(CRYPT_INTEGER_BLOB);
impl Drop for Output {
    fn drop(&mut self) {
        if !self.0.pbData.is_null() {
            // SAFETY: DPAPIが返したLocalAlloc領域を、その長さ内で消去し一度だけ解放する。
            unsafe {
                let bytes = std::slice::from_raw_parts_mut(self.0.pbData, self.0.cbData as usize);
                wipe(bytes);
                LocalFree(self.0.pbData.cast());
            }
        }
    }
}

/// contextはGUI-Shell側の用途・対象への結合であり、秘密鍵や承認ではない。
pub fn protect(context: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, Error> {
    transform(context, plaintext, true)
}

pub fn unprotect(context: &[u8], ciphertext: &[u8]) -> Result<Secret, Error> {
    transform(context, ciphertext, false).map(Secret)
}

fn transform(context: &[u8], data: &[u8], protect: bool) -> Result<Vec<u8>, Error> {
    let max = if protect {
        MAX_PLAINTEXT
    } else {
        MAX_CIPHERTEXT
    };
    if context.is_empty() || context.len() > 1024 || data.is_empty() || data.len() > max {
        return Err(Error::InvalidInput);
    }
    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr().cast_mut(),
    };
    let entropy = CRYPT_INTEGER_BLOB {
        cbData: context.len() as u32,
        pbData: context.as_ptr().cast_mut(),
    };
    let mut output = Output(CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    });
    // SAFETY: 入力・entropyは呼出し中生存し、APIは入力を変更しない。長さはu32範囲内。
    // 出力の所有権はOutputに限る。説明・promptはnull、UIは禁止、machine共有は指定しない。
    let ok = unsafe {
        if protect {
            CryptProtectData(
                &input,
                null_mut(),
                &entropy,
                null_mut(),
                null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output.0,
            )
        } else {
            CryptUnprotectData(
                &input,
                null_mut(),
                &entropy,
                null_mut(),
                null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output.0,
            )
        }
    };
    if ok == 0 {
        // SAFETY: 失敗直後のthread-localエラーを数値だけで取得する。
        return Err(Error::Windows(unsafe { GetLastError() }));
    }
    let max_output = if protect {
        MAX_CIPHERTEXT
    } else {
        MAX_PLAINTEXT
    };
    if output.0.pbData.is_null() || output.0.cbData == 0 || output.0.cbData as usize > max_output {
        return Err(Error::InvalidOutput);
    }
    // SAFETY: 成功したDPAPIの所有領域を解放前に複写する。元領域はDropで消去する。
    Ok(unsafe { std::slice::from_raw_parts(output.0.pbData, output.0.cbData as usize) }.to_vec())
}

pub mod file_delete;
