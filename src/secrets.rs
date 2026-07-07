use anyhow::{Context, Result};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

pub const PASSPHRASE_ENV: &str = "NIDO_PASSPHRASE";

/// Passphrase source: env var for automation (CI, containers), interactive
/// prompt otherwise. `confirm` asks twice when creating new secrets.
pub fn passphrase(confirm: bool) -> Result<String> {
    if let Ok(pass) = std::env::var(PASSPHRASE_ENV) {
        return Ok(pass);
    }
    let mut prompt = dialoguer::Password::new().with_prompt("Secrets passphrase");
    if confirm {
        prompt = prompt.with_confirmation("Repeat passphrase", "Passphrases don't match");
    }
    prompt.interact().context("could not read passphrase from terminal")
}

pub fn encrypt(plaintext: &[u8], pass: &str) -> Result<Vec<u8>> {
    let recipient = age::scrypt::Recipient::new(pass.to_owned().into());
    age::encrypt(&recipient, plaintext).context("age encryption failed")
}

pub fn decrypt(ciphertext: &[u8], pass: &str) -> Result<Vec<u8>> {
    let identity = age::scrypt::Identity::new(pass.to_owned().into());
    age::decrypt(&identity, ciphertext)
        .context("age decryption failed (wrong passphrase or corrupted file)")
}

/// Write decrypted secret material with owner-only permissions; parent
/// directories we create get 0700 (think ~/.ssh).
pub fn write_secret(dest: &Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = dest.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
            let mut dir = parent.to_path_buf();
            loop {
                std::fs::set_permissions(
                    &dir,
                    std::os::unix::fs::PermissionsExt::from_mode(0o700),
                )?;
                match dir.parent() {
                    Some(p) if !p.exists() => dir = p.to_path_buf(),
                    _ => break,
                }
            }
        }
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(dest)
        .with_context(|| format!("cannot write {}", dest.display()))?;
    file.write_all(data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let secret = b"-----BEGIN OPENSSH PRIVATE KEY-----";
        let encrypted = encrypt(secret, "hunter2").unwrap();
        assert_ne!(&encrypted[..], &secret[..]);
        assert_eq!(decrypt(&encrypted, "hunter2").unwrap(), secret);
        assert!(decrypt(&encrypted, "wrong").is_err());
    }
}
