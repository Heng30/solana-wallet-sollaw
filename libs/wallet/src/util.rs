use solana_sdk::native_token::lamports_to_sol;

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
        format!("{:.6}", sol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::native_token::LAMPORTS_PER_SOL;

    #[test]
    fn test_lamports_to_sol() {
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
    }
}
