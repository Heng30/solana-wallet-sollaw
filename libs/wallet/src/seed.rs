use anyhow::{Context, Result};
use bip32::{DerivationPath, XPrv};
use bip39::{Mnemonic, Seed};
use std::str::FromStr;

pub fn generate_seed(mnemonic: &Mnemonic, passphrase: &str) -> Seed {
    Seed::new(mnemonic, passphrase)
}

pub fn derive_seed_bytes(seed_bytes: &[u8], index: usize) -> Result<Vec<u8>> {
    if index == 0 {
        return Ok(seed_bytes.to_vec());
    }

    let path = format!("m/44'/501'/0'/0/{}", index);
    let derivation_path =
        DerivationPath::from_str(&path).with_context(|| format!("derivation_path: {path}"))?;

    let mut master_xprv = XPrv::new(seed_bytes).with_context(|| "Generate master_xprv failed")?;

    for child_number in derivation_path {
        master_xprv = master_xprv
            .derive_child(child_number)
            .with_context(|| format!("derive_child: {child_number:?} failed"))?;
    }

    Ok(master_xprv.private_key().to_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::{super::mnemonic, *};

    #[test]
    fn test_all() -> Result<()> {
        let mn = mnemonic::generate_mnemonic(bip39::MnemonicType::Words12);
        let seed = generate_seed(&mn, "123456");

        for index in 0..3 {
            let sb = derive_seed_bytes(seed.as_bytes(), index)?;
            println!("index: {index}");
            println!("{sb:?}\n");
        }

        Ok(())
    }
}
