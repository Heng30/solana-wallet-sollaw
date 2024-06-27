use anyhow::{bail, Context, Result};
use solana_sdk::{bs58, native_token::lamports_to_sol};

pub fn lamports_to_sol_str(lamports: u64) -> String {
    pretty_sol_str(lamports_to_sol(lamports))
}

fn pretty_sol_str(sol: f64) -> String {
    if sol >= 100_f64 {
        format!("{:.2}", sol)
    } else if sol >= 0.001_f64 {
        format!("{:.3}", sol)
    } else if sol >= 0.000_01_f64 {
        format!("{:.5}", sol)
    } else {
        format!("{:.8}", sol)
    }
}

pub fn format_number_with_commas(number_str: &str) -> String {
    if number_str.is_empty() {
        return String::default();
    }

    let chars: Vec<char> = number_str.chars().collect();
    let decimal_index = chars.iter().position(|&c| c == '.').unwrap_or(chars.len());

    let left_part = &mut chars[0..decimal_index]
        .iter()
        .rev()
        .copied()
        .collect::<Vec<char>>();

    let right_part = &number_str[decimal_index..];

    let mut chs = vec![];
    for (i, ch) in left_part.iter().enumerate() {
        chs.push(*ch);
        if (i + 1) % 3 == 0 {
            chs.push(',');
        }
    }

    if chs[chs.len() - 1] == ',' {
        chs.pop();
    }

    format!("{}{}", chs.iter().rev().collect::<String>(), right_part)
}

pub fn is_valid_solana_address(address: &str) -> Result<()> {
    if address.len() != 44 {
        bail!("Address {address} length is not correct. The correct length is 44");
    }

    bs58::decode(&address[..])
        .into_vec()
        .with_context(|| format!("Base58 decode {address} failed"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::native_token::LAMPORTS_PER_SOL;

    #[test]
    fn test_lamports_to_sol() -> Result<()> {
        let verify = vec![
            "100.00",
            "100.12",
            "0.100",
            "0.120",
            "0.003",
            "0.004",
            "0.00006",
            "0.00007",
            "0.00000800",
            "0.00000090",
        ];

        let output = [
            100_f64,
            100.123,
            0.10,
            0.12,
            0.003,
            0.004_3,
            0.000_06,
            0.000_073,
            0.000_008,
            0.000_000_9,
        ]
        .into_iter()
        .map(|item| lamports_to_sol_str((item * LAMPORTS_PER_SOL as f64) as u64))
        .collect::<Vec<_>>();

        assert_eq!(verify, output);

        Ok(())
    }

    #[test]
    fn test_format_number_with_commas() {
        let verify = vec![
            "", "1.23", "12.12", "123.12", "1,234.12", "1", "12", "123", "1,234", "123,456",
        ];

        let output = [
            "", "1.23", "12.12", "123.12", "1234.12", "1", "12", "123", "1234", "123456",
        ]
        .into_iter()
        .map(|item| format_number_with_commas(&item))
        .collect::<Vec<_>>();

        assert_eq!(verify, output);
    }

    #[test]
    fn test_is_valid_solana_address() -> Result<()> {
        assert!(is_valid_solana_address("5p6rcsWRHpkZmsusbuhsz9rgcPJWnbHcaDLgENTSUxY8").is_ok());
        assert!(is_valid_solana_address("5p6rcsWRHpkZmsusbuhsz9rgcPJWnbHcaDLgENTSUxY").is_err());
        assert!(is_valid_solana_address("5p6rcsWRHpkZmsusbuhsz9rgcPJWnbHcaDLgENTSUxY0").is_err());
        assert!(is_valid_solana_address("5p6rcsWRHpkZmsusbuhsz9rgcPJWnbHcaDLgENTSUxY01").is_err());

        Ok(())
    }
}
