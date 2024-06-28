use anyhow::{Context, Result};
use solana_sdk::{
    signature::{keypair_from_seed, write_keypair_file},
    signer::keypair::Keypair,
};
use std::fs;

pub fn generate_keypair(seed_bytes: &[u8]) -> Result<Keypair> {
    keypair_from_seed(seed_bytes)
        .map_err(|e| anyhow::anyhow!("{e:?}"))
        .with_context(|| "Failed to generate keypair to file")
}

pub fn write_keypair_to_file(keypair: &Keypair, file_path: &str) -> Result<String> {
    write_keypair_file(keypair, file_path)
        .map_err(|e| anyhow::anyhow!("{e:?}"))
        .with_context(|| format!("Failed to write keypair to file: {file_path}"))
}

pub fn read_keypair_from_file(file_path: &str) -> Result<Keypair> {
    let content =
        fs::read_to_string(file_path).with_context(|| format!("read {file_path:?} failed"))?;

    let cleaned_content = content.trim_matches(|c: char| c == '[' || c == ']' || c.is_whitespace());
    let bytes: Result<Vec<u8>, _> = cleaned_content
        .split(',')
        .map(|s| s.trim().parse::<u8>())
        .collect();

    match bytes {
        Ok(bytes) => Keypair::from_bytes(&bytes)
            .with_context(|| format!("Failed to parse keypair from file: {file_path:?}")),
        Err(e) => Err(e).with_context(|| format!("Invalid byte format in file: {file_path:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{*, super::{mnemonic, seed}};

    #[test]
    fn test_all() -> Result<()> {
        let mn = mnemonic::generate_mnemonic(bip39::MnemonicType::Words12);
        let seed = seed::generate_seed(&mn, "123456");

        for index in 0..3 {
            let sb = seed::derive_seed_bytes(seed.as_bytes(), index)?;
            let kp = generate_keypair(sb.as_slice())?;
            let path = format!("/tmp/keypair-{index}");
            write_keypair_to_file(&kp, &path)?;
            let kp2 = read_keypair_from_file(&path)?;
            assert_eq!(kp.to_base58_string(), kp2.to_base58_string());
        }

        Ok(())
    }
}
