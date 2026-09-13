#![cfg(windows)]
#![forbid(unsafe_code)]
use gui_shell_windows_protection::{protect, unprotect, MAX_CIPHERTEXT, MAX_PLAINTEXT};

#[test]
fn 実_dpapiの用途結合と改変拒否を確認する() {
    let context = b"GUI-Shell:history:request-a:v1";
    let plain = b"synthetic content without operational credentials";
    let ciphertext = protect(context, plain).unwrap();
    assert!(ciphertext.windows(plain.len()).all(|w| w != plain));
    assert_eq!(unprotect(context, &ciphertext).unwrap().as_bytes(), plain);
    assert!(unprotect(b"GUI-Shell:history:request-b:v1", &ciphertext).is_err());
    let mut corrupt = ciphertext.clone();
    let last = corrupt.len() - 1;
    corrupt[last] ^= 1;
    assert!(unprotect(context, &corrupt).is_err());
    assert!(unprotect(context, &ciphertext[..8]).is_err());
    let repeated = protect(context, plain).unwrap();
    assert_ne!(ciphertext, repeated);
}

#[test]
fn 入力と用途の上限を_os呼出し前に拒否する() {
    assert!(protect(b"", b"x").is_err());
    assert!(protect(&vec![1; 1025], b"x").is_err());
    assert!(protect(b"context", b"").is_err());
    assert!(protect(b"context", &vec![1; MAX_PLAINTEXT + 1]).is_err());
    assert!(unprotect(b"context", &vec![1; MAX_CIPHERTEXT + 1]).is_err());
    let limit = vec![42; MAX_PLAINTEXT];
    let encrypted = protect(b"limit", &limit).unwrap();
    assert_eq!(unprotect(b"limit", &encrypted).unwrap().as_bytes(), limit);
}
