use std::io::Read;

use aes::Aes256;
use anyhow::{anyhow, Context, Result};
use base64::Engine;
use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
use percent_encoding::percent_decode_str;
use sha2::Sha256;

const PRODUCTION_PASSWORD: &str = "J*8sQ!p$7aD_fR2yW@gHn*3bVp#sAdLd_k";
const PRODUCTION_SALT: &str = "5b9a8f2c3e6d1a4b7c8e9d0f1a2b3c4d";
const RECENT_PASSWORD: &str = "%RtR8AB&nWsh=AQC+v!=pgAe@dSQG3kQ";
const RECENT_SALT: &str = "orieC_jQQWRmhkPvR6u2kzXeTube6aYupiOddsPortal";

pub fn decode_dat_payload(body: &str) -> Result<serde_json::Value> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(body.trim())
        .context("failed to base64-decode OddsPortal .dat payload")?;

    if let Ok(decoded) = decode_legacy_payload(&bytes) {
        return Ok(decoded);
    }

    decode_encrypted_payload(&bytes)
}

fn decode_legacy_payload(bytes: &[u8]) -> Result<serde_json::Value> {
    let inflated = inflate_payload(bytes)?;
    let encoded = String::from_utf8(inflated).context("OddsPortal .dat payload is not UTF-8")?;
    let decoded = percent_decode_str(&encoded)
        .decode_utf8()
        .context("failed to URL-decode OddsPortal .dat payload")?;

    parse_json_text(&decoded)
}

fn decode_encrypted_payload(bytes: &[u8]) -> Result<serde_json::Value> {
    let envelope =
        std::str::from_utf8(bytes).context("encrypted OddsPortal .dat envelope is not UTF-8")?;
    let (ciphertext_base64, iv_hex) = envelope
        .split_once(':')
        .ok_or_else(|| anyhow!("encrypted OddsPortal .dat envelope is missing IV separator"))?;
    let ciphertext = base64::engine::general_purpose::STANDARD
        .decode(ciphertext_base64)
        .context("failed to base64-decode encrypted OddsPortal ciphertext")?;
    let iv = decode_hex_iv(iv_hex)?;

    let mut last_error = None;
    for (password, salt) in [
        (PRODUCTION_PASSWORD, PRODUCTION_SALT),
        (RECENT_PASSWORD, RECENT_SALT),
    ] {
        match decrypt_aes_cbc(&ciphertext, &iv, password, salt) {
            Ok(decrypted) => {
                let payload = if decrypted.starts_with(&[0x1f, 0x8b]) {
                    try_read(GzDecoder::new(decrypted.as_slice()))
                        .context("failed to gzip-decompress OddsPortal decrypted payload")?
                } else {
                    decrypted
                };
                let text = String::from_utf8(payload)
                    .context("decrypted OddsPortal .dat payload is not UTF-8")?;
                return parse_json_text(&text);
            }
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error
        .unwrap_or_else(|| anyhow!("failed to decrypt OddsPortal .dat payload with known keys")))
}

fn decrypt_aes_cbc(ciphertext: &[u8], iv: &[u8], password: &str, salt: &str) -> Result<Vec<u8>> {
    let mut key = [0_u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(password.as_bytes(), salt.as_bytes(), 1_000, &mut key);

    cbc::Decryptor::<Aes256>::new_from_slices(&key, iv)
        .context("failed to initialize OddsPortal AES-CBC decoder")?
        .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
        .map_err(|error| anyhow!("failed to AES-CBC decrypt OddsPortal .dat payload: {error}"))
}

fn decode_hex_iv(hex: &str) -> Result<Vec<u8>> {
    if hex.len() % 2 != 0 {
        return Err(anyhow!("OddsPortal IV has odd hex length"));
    }

    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for index in (0..hex.len()).step_by(2) {
        let byte = u8::from_str_radix(&hex[index..index + 2], 16)
            .with_context(|| format!("OddsPortal IV contains invalid hex at byte {index}"))?;
        bytes.push(byte);
    }

    if bytes.len() != 16 {
        return Err(anyhow!(
            "OddsPortal IV must be 16 bytes, got {} bytes",
            bytes.len()
        ));
    }

    Ok(bytes)
}

fn parse_json_text(text: &str) -> Result<serde_json::Value> {
    if let Ok(value) = serde_json::from_str(text.trim()) {
        return Ok(value);
    }

    let end = text
        .rfind('}')
        .ok_or_else(|| anyhow!("decoded OddsPortal .dat payload does not contain JSON object"))?;
    serde_json::from_str(text[..=end].trim())
        .context("failed to parse decoded OddsPortal .dat JSON")
}

fn inflate_payload(bytes: &[u8]) -> Result<Vec<u8>> {
    try_read(ZlibDecoder::new(bytes))
        .or_else(|_| try_read(GzDecoder::new(bytes)))
        .or_else(|_| try_read(DeflateDecoder::new(bytes)))
        .map_err(|error| anyhow!("failed to inflate OddsPortal .dat payload: {error}"))
}

fn try_read(mut reader: impl Read) -> std::io::Result<Vec<u8>> {
    let mut output = Vec::new();
    reader.read_to_end(&mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    use std::io::Write;

    #[test]
    fn decodes_base64_zlib_urlencoded_json() {
        let json = r#"{"d":{"oddsdata":{"back":{}}}}"#;
        let encoded = encode_test_payload(json);

        let decoded = decode_dat_payload(&encoded).unwrap();

        assert_eq!(decoded["d"]["oddsdata"]["back"], serde_json::json!({}));
    }

    #[test]
    fn decodes_encrypted_aes_cbc_envelope() {
        let encoded = "dERXejJKcmlJMGIyWlExWEhCYm5EZ3VDSy9YUGZ6NUJpalNRd3VLeVlhWT06MDAwMTAyMDMwNDA1MDYwNzA4MDkwYTBiMGMwZDBlMGY=";

        let decoded = decode_dat_payload(encoded).unwrap();

        assert_eq!(decoded["d"]["oddsdata"]["back"], serde_json::json!({}));
    }

    fn encode_test_payload(json: &str) -> String {
        let url_encoded = utf8_percent_encode(json, NON_ALPHANUMERIC).to_string();
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(url_encoded.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();
        base64::engine::general_purpose::STANDARD.encode(compressed)
    }
}
