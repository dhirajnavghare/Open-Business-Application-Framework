// Copyright 2026 Dhiraj Navghare
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use hmac::{Hmac, Mac};
use sha1::Sha1;
use uuid::Uuid;

type HmacSha1 = Hmac<Sha1>;

const BASE32_ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// Encodes raw bytes into RFC 4648 Base32 string (unpadded).
pub fn base32_encode(data: &[u8]) -> String {
    let mut result = String::new();
    let mut buffer: u64 = 0;
    let mut bits_left = 0;

    for &b in data {
        buffer = (buffer << 8) | (b as u64);
        bits_left += 8;
        while bits_left >= 5 {
            bits_left -= 5;
            let index = ((buffer >> bits_left) & 0x1f) as usize;
            result.push(BASE32_ALPHABET[index] as char);
        }
    }

    if bits_left > 0 {
        let index = ((buffer << (5 - bits_left)) & 0x1f) as usize;
        result.push(BASE32_ALPHABET[index] as char);
    }

    result
}

/// Decodes an RFC 4648 Base32 string back to bytes.
pub fn base32_decode(input: &str) -> Option<Vec<u8>> {
    let clean = input.trim().to_uppercase().replace([' ', '-'], "");
    let mut result = Vec::new();
    let mut buffer: u64 = 0;
    let mut bits_left = 0;

    for c in clean.chars() {
        if c == '=' {
            break;
        }
        let val = match c {
            'A'..='Z' => (c as u8 - b'A') as u64,
            '2'..='7' => (c as u8 - b'2' + 26) as u64,
            _ => return None,
        };

        buffer = (buffer << 5) | val;
        bits_left += 5;

        if bits_left >= 8 {
            bits_left -= 8;
            result.push(((buffer >> bits_left) & 0xff) as u8);
        }
    }

    Some(result)
}

/// Generates a cryptographically random 20-byte (160-bit) Base32 secret for TOTP.
pub fn generate_totp_secret() -> String {
    let mut bytes = [0u8; 20];
    let (u1, u2) = (Uuid::new_v4(), Uuid::new_v4());
    bytes[..16].copy_from_slice(u1.as_bytes());
    bytes[16..].copy_from_slice(&u2.as_bytes()[..4]);
    base32_encode(&bytes)
}

/// Generates a set of 6 emergency single-use recovery codes.
pub fn generate_recovery_codes() -> Vec<String> {
    (0..6)
        .map(|_| {
            let u = Uuid::new_v4().simple().to_string();
            format!("{}-{}", &u[..4], &u[4..8])
        })
        .collect()
}

/// Computes the 6-digit TOTP code for a given timestamp and 30-second time step (RFC 6238 / RFC 4226).
pub fn compute_totp(secret_base32: &str, unix_timestamp: u64, time_step: u64) -> Option<String> {
    let key_bytes = base32_decode(secret_base32)?;
    let counter = unix_timestamp / time_step;
    let counter_bytes = counter.to_be_bytes();

    let mut mac = HmacSha1::new_from_slice(&key_bytes).ok()?;
    mac.update(&counter_bytes);
    let result = mac.finalize().into_bytes();

    // Dynamic truncation (RFC 4226 section 5.4)
    let offset = (result[result.len() - 1] & 0x0f) as usize;
    let binary = ((result[offset] as u32 & 0x7f) << 24)
        | ((result[offset + 1] as u32 & 0xff) << 16)
        | ((result[offset + 2] as u32 & 0xff) << 8)
        | (result[offset + 3] as u32 & 0xff);

    let otp = binary % 1_000_000;
    Some(format!("{:06}", otp))
}

/// Verifies a 6-digit TOTP code with standard drift allowance (+/- 1 time step, i.e. +/- 30s).
pub fn verify_totp(secret_base32: &str, code: &str, unix_timestamp: u64) -> bool {
    let clean_code = code.trim();
    if clean_code.len() != 6 || !clean_code.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    // Check t - 30, t, and t + 30 seconds for clock skew tolerance
    for offset in &[-30i64, 0, 30] {
        let test_time = (unix_timestamp as i64 + offset) as u64;
        if let Some(valid_code) = compute_totp(secret_base32, test_time, 30) {
            // Constant-time equality check to prevent timing attacks
            if constant_time_compare(&valid_code, clean_code) {
                return true;
            }
        }
    }

    false
}

/// Constant-time string equality check to prevent timing attacks.
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Builds an `otpauth://` URI suitable for QR codes and authenticator apps (RFC 6238).
pub fn build_otpauth_uri(issuer: &str, account_name: &str, secret_base32: &str) -> String {
    let encoded_label = format!("{}:{}", url_encode(issuer), url_encode(account_name));
    format!(
        "otpauth://totp/{}?secret={}&issuer={}&algorithm=SHA1&digits=6&period=30",
        encoded_label,
        secret_base32.trim(),
        url_encode(issuer)
    )
}

fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char)
            }
            _ => encoded.push_str(&format!("%{:02X}", b)),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base32_encode_decode_roundtrip() {
        let data = b"Hello, World! Security Audit 2026";
        let encoded = base32_encode(data);
        let decoded = base32_decode(&encoded).expect("Valid base32");
        assert_eq!(data.to_vec(), decoded);
    }

    #[test]
    fn test_totp_generation_and_verification() {
        let secret = generate_totp_secret();
        assert!(secret.len() >= 16);

        let now = 1700000000u64;
        let code = compute_totp(&secret, now, 30).expect("Compute TOTP");
        assert_eq!(code.len(), 6);

        // Exact timestamp matches
        assert!(verify_totp(&secret, &code, now));

        // Within 30s clock drift matches
        assert!(verify_totp(&secret, &code, now + 15));
        assert!(verify_totp(&secret, &code, now - 15));

        // Wrong code fails
        assert!(!verify_totp(&secret, "000000", now));

        // Far clock skew fails (>60s)
        assert!(!verify_totp(&secret, &code, now + 90));
    }

    #[test]
    fn test_recovery_codes_generation() {
        let codes = generate_recovery_codes();
        assert_eq!(codes.len(), 6);
        for code in codes {
            assert_eq!(code.len(), 9); // e.g. "abcd-efgh"
            assert!(code.contains('-'));
        }
    }

    #[test]
    fn test_otpauth_uri_format() {
        let secret = "JBSWY3DPEHPK3PXP";
        let uri = build_otpauth_uri("OBAF Platform", "admin@obaf.local", secret);
        assert!(uri.starts_with("otpauth://totp/"));
        assert!(uri.contains("secret=JBSWY3DPEHPK3PXP"));
        assert!(uri.contains("issuer=OBAF%20Platform"));
    }
}
