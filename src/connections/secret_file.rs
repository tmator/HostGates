//! Short-lived client credential files. Restrict access BEFORE writing any data.
use crate::error::{AppError, Result};
use std::io::Write;
use tempfile::NamedTempFile;

pub fn create(suffix: &str, contents: &[u8]) -> Result<NamedTempFile> {
    let mut file = tempfile::Builder::new()
        .prefix("hostgates-auth-")
        .suffix(suffix)
        .tempfile()
        .map_err(|_| failure())?;
    #[cfg(windows)]
    restrict(file.path())?;
    // tempfile uses 0600 on Unix. On Windows, explicitly replace the inherited DACL.
    file.write_all(contents).map_err(|_| failure())?;
    file.flush().map_err(|_| failure())?;
    Ok(file)
}
fn failure() -> AppError {
    AppError::Operation("Impossible de préparer un fichier d'authentification protégé.".into())
}
#[cfg(windows)]
fn restrict(path: &std::path::Path) -> Result<()> {
    use std::{os::windows::ffi::OsStrExt, ptr::null_mut};
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::{Authorization::*, *},
    };
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // All pointers below are owned by Windows until LocalFree; owner borrows the first descriptor.
    unsafe {
        let mut owner = null_mut();
        let mut descriptor = null_mut();
        if GetNamedSecurityInfoW(
            path.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            &mut owner,
            null_mut(),
            null_mut(),
            null_mut(),
            &mut descriptor,
        ) != 0
        {
            return Err(failure());
        }
        let mut sid = null_mut();
        let converted = ConvertSidToStringSidW(owner, &mut sid);
        LocalFree(descriptor);
        if converted == 0 {
            return Err(failure());
        }
        let mut len = 0;
        while *sid.add(len) != 0 {
            len += 1;
        }
        let sid_text = String::from_utf16_lossy(std::slice::from_raw_parts(sid, len));
        LocalFree(sid.cast());
        let sddl: Vec<u16> = format!("D:P(A;;FA;;;{sid_text})")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut restricted = null_mut();
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut restricted,
            null_mut(),
        ) == 0
        {
            return Err(failure());
        }
        let result = SetFileSecurityW(
            path.as_ptr(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            restricted,
        );
        LocalFree(restricted);
        if result == 0 {
            return Err(failure());
        }
    }
    Ok(())
}
#[cfg(windows)]
pub fn protect_password(password: &str) -> Result<Vec<u8>> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::{Foundation::LocalFree, Security::Cryptography::*};
    let mut plain = zeroize::Zeroizing::new(
        password
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
    );
    let input = CRYPT_INTEGER_BLOB {
        cbData: plain.len() as u32,
        pbData: plain.as_mut_ptr(),
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };
    // DPAPI is scoped to the current Windows account (no LOCAL_MACHINE flag).
    unsafe {
        if CryptProtectData(
            &input,
            null(),
            null(),
            null(),
            null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        ) == 0
        {
            return Err(failure());
        }
        let encrypted = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(output.pbData.cast());
        Ok(encrypted)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn temporary_file_is_readable_and_removed() {
        let file = super::create(".test", b"fixture").expect("protected file");
        let path = file.path().to_owned();
        assert_eq!(std::fs::read(&path).expect("read"), b"fixture");
        drop(file);
        assert!(!path.exists());
    }
    #[cfg(windows)]
    #[test]
    fn dpapi_roundtrip_unicode() {
        use windows_sys::Win32::{Foundation::LocalFree, Security::Cryptography::*};
        let mut encrypted = super::protect_password("pàss🔐").expect("protect");
        let input = CRYPT_INTEGER_BLOB {
            cbData: encrypted.len() as u32,
            pbData: encrypted.as_mut_ptr(),
        };
        let mut output: CRYPT_INTEGER_BLOB = unsafe { std::mem::zeroed() };
        unsafe {
            assert_ne!(
                CryptUnprotectData(
                    &input,
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut output
                ),
                0
            );
            let bytes = std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize);
            assert_eq!(
                bytes,
                "pàss🔐"
                    .encode_utf16()
                    .flat_map(u16::to_le_bytes)
                    .collect::<Vec<_>>()
            );
            zeroize::Zeroize::zeroize(bytes);
            LocalFree(output.pbData.cast());
        }
    }
}
