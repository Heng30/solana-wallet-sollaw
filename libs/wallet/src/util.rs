use solana_sdk::native_token::lamports_to_sol;

pub fn lamports_to_sol_str(lamports: u64) -> String {
    pretty_sol_str(lamports_to_sol(lamports))
}

fn pretty_sol_str(sol: f64) -> String {
    if sol >= 100_f64 {
        format!("{:.3}", sol)
    } else if sol >= 0.000_001_f64 {
        format!("{:.6}", sol)
    } else {
        format!("{:.9}", sol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
