use crate::{
    error::{AppError, Result},
    models::{Protocol, SessionInput},
};
pub fn arguments(s: &SessionInput) -> Result<Vec<String>> {
    s.validate()?;
    if s.protocol != Protocol::VNC {
        return Err(AppError::Validation("Session VNC attendue.".into()));
    }
    let host = if s.hostname.contains(':') && !s.hostname.starts_with('[') {
        format!("[{}]", s.hostname)
    } else {
        s.hostname.clone()
    };
    Ok(vec![format!("{host}::{}", s.port)])
}
pub fn launch(
    s: &SessionInput,
    configured: &str,
    password: &str,
) -> Result<super::rdp::RdpSession> {
    let program = super::resolve_program(configured, super::ProgramKind::Vnc)?;
    let mut args = arguments(s)?;
    let file = if password.is_empty() {
        None
    } else {
        let encoded = password_file_bytes(password)?;
        let file = super::secret_file::create(".vnc-passwd", &*encoded)?;
        args.push("-PasswordFile".into());
        args.push(file.path().to_string_lossy().into_owned());
        Some(file)
    };
    let mut client = super::rdp::RdpSession::launch_external(&program, args, "VNC")?;
    if let Some(file) = file {
        client.retain_credential_file(file);
    }
    Ok(client)
}

// Classic VNC password-file format. DES here is protocol obfuscation, NOT vault encryption.
fn password_file_bytes(password: &str) -> Result<zeroize::Zeroizing<[u8; 8]>> {
    use des::cipher::{BlockEncrypt, KeyInit};
    use zeroize::Zeroize;
    if password.len() > 8 || password.contains('\0') || !password.is_ascii() {
        return Err(AppError::Validation("VNC : la transmission automatique utilise le format TigerVNC classique (1 à 8 caractères ASCII). Pour un autre mode, saisissez le mot de passe dans le client.".into()));
    }
    let mut plain = zeroize::Zeroizing::new([0u8; 8]);
    plain[..password.len()].copy_from_slice(password.as_bytes());
    // VNC's DES bit order is reversed within each key byte.
    let key = [23u8, 82, 107, 6, 35, 78, 88, 7].map(u8::reverse_bits);
    let cipher = des::Des::new(&key.into());
    let mut block = des::cipher::Block::<des::Des>::clone_from_slice(&*plain);
    cipher.encrypt_block(&mut block);
    plain.copy_from_slice(&block);
    block.as_mut_slice().zeroize();
    Ok(plain)
}
#[cfg(test)]
mod tests {
    #[test]
    fn classic_vnc_known_vector_and_no_truncation() {
        assert_eq!(
            *super::password_file_bytes("password").expect("encode"),
            [0xdb, 0xd8, 0x3c, 0xfd, 0x72, 0x7a, 0x14, 0x58]
        );
        assert!(super::password_file_bytes("password9").is_err());
        assert!(super::password_file_bytes("é").is_err());
    }
}
