use anyhow::{Context, Result};
use bip39::{Language, Mnemonic, MnemonicType};

pub fn generate_mnemonic(mt: MnemonicType) -> Mnemonic {
    Mnemonic::new(mt, Language::English)
}

pub fn mnemonic_to_str(mnemonic: &Mnemonic) -> &str {
    mnemonic.phrase()
}

pub fn mnemonic_from_phrase(phrase: &str) -> Result<Mnemonic> {
    let mnemonic = Mnemonic::from_phrase(phrase, Language::English)
        .with_context(|| "Failed to get mnemonic from phrase: {phrase:?}")?;

    Ok(mnemonic)
}

pub fn valid_mnemonic(mnemonic: &str, mt: MnemonicType) -> bool {
    let words = mnemonic
        .split(char::is_whitespace)
        .filter_map(|w| {
            let w = w.trim();
            if w.is_empty() {
                None
            } else {
                Some(w)
            }
        })
        .collect::<Vec<_>>();

    match mt {
        MnemonicType::Words12 => words.len() == 12,
        MnemonicType::Words24 => words.len() == 24,
        _ => unimplemented!("only support 12 and 24 word counts mnemonic"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all() -> Result<()> {
        for ty in [MnemonicType::Words12, MnemonicType::Words24] {
            let mn = generate_mnemonic(ty);
            let phrase = mnemonic_to_str(&mn);
            assert!(valid_mnemonic(phrase, ty));
            println!("{phrase}\n");

            let mn2 = mnemonic_from_phrase(phrase)?;
            assert_eq!(mn.to_string(), mn2.to_string());
        }

        Ok(())
    }
}
