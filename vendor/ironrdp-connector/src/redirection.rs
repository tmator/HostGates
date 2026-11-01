//! TLS-protected RDS broker redirection (MS-RDPBCGR 2.2.13).
use crate::{ConnectorResult, reason_err};

#[derive(Clone)]
pub struct RedirectionSecret(pub zeroize::Zeroizing<Vec<u8>>);
impl core::fmt::Debug for RedirectionSecret {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("[redacted]")
    }
}

#[derive(Clone)]
pub struct ServerRedirection {
    pub session_id: u32,
    pub flags: u32,
    pub address: Option<String>,
    pub fqdn: Option<String>,
    pub netbios: Option<String>,
    pub routing: Option<String>,
    pub username: Option<String>,
    pub domain: Option<String>,
    pub password: Option<RedirectionSecret>,
    pub tsv_url: Option<Vec<u8>>,
}
impl core::fmt::Debug for ServerRedirection {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("ServerRedirection { [redacted] }")
    }
}

fn invalid() -> crate::ConnectorError {
    reason_err!(
        "RDS redirection",
        "malformed or unsupported server redirection packet"
    )
}
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> ConnectorResult<&'a [u8]> {
        let bytes = self.0.get(..n).ok_or_else(invalid)?;
        self.0 = &self.0[n..];
        Ok(bytes)
    }
    fn u32(&mut self) -> ConnectorResult<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn field(&mut self) -> ConnectorResult<&'a [u8]> {
        let n = usize::try_from(self.u32()?).map_err(|_| invalid())?;
        self.take(n)
    }
    fn text(&mut self) -> ConnectorResult<String> {
        let b = self.field()?;
        if b.len() < 2 || b.len() > 2048 || b.len() % 2 != 0 || !b.ends_with(&[0, 0]) {
            return Err(invalid());
        }
        let words: Vec<_> = b[..b.len() - 2]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let s = String::from_utf16(&words).map_err(|_| invalid())?;
        if s.chars().any(char::is_control) {
            return Err(invalid());
        }
        Ok(s)
    }
}

/// Inspect only an already TLS-decrypted MCS I/O-channel message. No secrets are logged.
pub fn decode(input: &[u8], io_channel_id: u16) -> ConnectorResult<Option<ServerRedirection>> {
    let Ok(ctx) = ironrdp_pdu::mcs::decode_send_data_indication(input) else {
        return Ok(None);
    };
    let b = ctx.user_data;
    if ctx.channel_id != io_channel_id || b.len() < 4 || u16::from_le_bytes([b[2], b[3]]) != 0x000a
    {
        return Ok(None);
    }
    decode_body(b).map(Some)
}

fn decode_body(b: &[u8]) -> ConnectorResult<ServerRedirection> {
    if b.len() < 20 || usize::from(u16::from_le_bytes([b[0], b[1]])) != b.len() {
        return Err(invalid());
    }
    let mut r = Reader(&b[8..]); // six-byte share header, two padding bytes
    if r.take(2)? != [0, 4] {
        return Err(invalid());
    }
    let len = r.take(2)?;
    let len = usize::from(u16::from_le_bytes([len[0], len[1]]));
    if len < 12 || len > b.len() - 8 || b.len() - 8 - len > 1 {
        return Err(invalid());
    }
    r.0 = &b[12..8 + len];
    let session_id = r.u32()?;
    let flags = r.u32()?;
    // V4 fields, plus known later flags. Reject unknown layouts, smart cards and RDSTLS.
    if flags & !0x1fbff != 0 || flags & (0x40 | 0x4000 | 0x8000 | 0x10000) != 0 {
        return Err(reason_err!(
            "RDS redirection",
            "unsupported redirection flags (smart card or RDSTLS)"
        ));
    }
    let address = if flags & 1 != 0 {
        Some(r.text()?)
    } else {
        None
    };
    let routing = if flags & 2 != 0 {
        let bytes = r.field()?;
        let bytes = bytes.strip_suffix(b"\r\n").unwrap_or(bytes);
        if bytes.len() > 200 || !bytes.is_ascii() || bytes.iter().any(|b| b.is_ascii_control()) {
            return Err(invalid());
        }
        Some(String::from_utf8(bytes.to_vec()).map_err(|_| invalid())?)
    } else {
        None
    };
    let username = if flags & 4 != 0 {
        Some(r.text()?)
    } else {
        None
    };
    let domain = if flags & 8 != 0 {
        Some(r.text()?)
    } else {
        None
    };
    let password = if flags & 16 != 0 {
        let b = r.field()?;
        // Opaque redirection cookie, including its two terminal bytes; never a UTF-8 password.
        if b.len() < 2 || b.len() > 4096 || !b.ends_with(&[0, 0]) {
            return Err(invalid());
        }
        Some(RedirectionSecret(zeroize::Zeroizing::new(b.to_vec())))
    } else {
        None
    };
    let fqdn = if flags & 0x100 != 0 {
        Some(r.text()?)
    } else {
        None
    };
    let netbios = if flags & 0x200 != 0 {
        Some(r.text()?)
    } else {
        None
    };
    let tsv_url = if flags & 0x1000 != 0 {
        Some(r.field()?.to_vec())
    } else {
        None
    };
    let mut address = address;
    if flags & 0x800 != 0 {
        let mut list = Reader(r.field()?);
        let count = list.u32()?;
        if count > 32 {
            return Err(invalid());
        }
        for _ in 0..count {
            let candidate = list.text()?;
            if address.is_none() {
                address = Some(candidate);
            }
        }
        if !list.0.is_empty() {
            return Err(invalid());
        }
    }
    if r.0.len() > 8 {
        return Err(invalid());
    } // optional protocol padding
    Ok(ServerRedirection {
        session_id,
        flags,
        address,
        fqdn,
        netbios,
        routing,
        username,
        domain,
        password,
        tsv_url,
    })
}

/// Replace only the Client Info password bytes with the opaque broker cookie.
/// CredSSP continues to use the original password on the freshly validated TLS connection.
pub fn client_info_with_cookie(
    info: &ironrdp_pdu::rdp::ClientInfoPdu,
    cookie: &RedirectionSecret,
) -> ConnectorResult<zeroize::Zeroizing<Vec<u8>>> {
    use crate::ConnectorErrorExt as _;
    let mut bytes = zeroize::Zeroizing::new(
        ironrdp_core::encode_vec(info).map_err(crate::ConnectorError::encode)?,
    );
    if bytes.len() < 22 || cookie.0.len() < 2 || cookie.0.len() > 4096 {
        return Err(invalid());
    }
    let domain_len = usize::from(u16::from_le_bytes([bytes[12], bytes[13]]));
    let user_len = usize::from(u16::from_le_bytes([bytes[14], bytes[15]]));
    let password_len = usize::from(u16::from_le_bytes([bytes[16], bytes[17]]));
    let start = 22 + domain_len + 2 + user_len + 2;
    let end = start + password_len + 2;
    if end > bytes.len() {
        return Err(invalid());
    }
    // Zero the displaced secret before Vec::splice moves or reallocates it.
    use zeroize::Zeroize;
    bytes[start..end].zeroize();
    bytes.splice(start..end, cookie.0.iter().copied());
    bytes[16..18].copy_from_slice(
        &u16::try_from(cookie.0.len() - 2)
            .map_err(|_| invalid())?
            .to_le_bytes(),
    );
    Ok(bytes)
}
