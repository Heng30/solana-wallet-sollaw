#[allow(dead_code)]
pub fn split_string_to_fixed_length_parts(input: &str, length: usize) -> Vec<String> {
    input
        .chars()
        .collect::<Vec<_>>()
        .chunks(length)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect()
}

#[allow(dead_code)]
pub fn pretty_size_string(size: u64) -> String {
    match size {
        s if s < 1024 => format!("{}B", size),
        s if s < 1024 * 1024 => format!("{}K", size / 1024),
        s if s < 1024 * 1024 * 1024 => format!("{}M", size / (1024 * 1024)),
        _ => format!("{}G", size / (1024 * 1024 * 1024)),
    }
}
