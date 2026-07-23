use super::pos::{load_tablebase, Status};

pub fn load_replacing_unknowns(path: &str) -> std::io::Result<[Vec<Status>; 100]> {
    let mut tb = load_tablebase(path)?;
    for file in tb.iter_mut() {
        for s in file.iter_mut() {
            if *s == Status::UNKOWN {
                *s = Status(0);
            }
        }
    }
    Ok(tb)
}

fn flatten(tb: &[Vec<Status>; 100]) -> Vec<u8> {
    tb.iter().flat_map(|f| f.iter().map(|s| s.0 as u8)).collect()
}

fn byte_counts(data: &[u8]) -> [u64; 256] {
    let mut counts = [0u64; 256];
    for &b in data { counts[b as usize] += 1; }
    counts
}

fn print_distribution(data: &[u8]) {
    let counts = byte_counts(data);
    let total = data.len() as f64;

    println!("Total:   {} bytes  ({:.2} MB)", data.len(), data.len() as f64 / 1_048_576.0);
    println!("Zeros:   {} ({:.2}%)", counts[0], counts[0] as f64 / total * 100.0);

    let wins: u64    = counts[1..=0x7e].iter().sum();
    let losses: u64  = counts[0x80..].iter().sum();
    let unknowns_left = counts[0x7f];
    println!("Wins:    {} ({:.2}%)", wins,   wins   as f64 / total * 100.0);
    println!("Losses:  {} ({:.2}%)", losses, losses as f64 / total * 100.0);
    if unknowns_left > 0 {
        println!("WARNING: {} unreplaced unknowns (0x7f)", unknowns_left);
    }

    println!("\nTop 15 non-zero values (as i8):");
    let mut nz: Vec<(i8, u64)> = counts.iter().enumerate().skip(1)
        .filter(|(_, &c)| c > 0)
        .map(|(i, &c)| (i as u8 as i8, c))
        .collect();
    nz.sort_by_key(|&(_, c)| std::cmp::Reverse(c));
    for (val, count) in nz.iter().take(15) {
        println!("  {:5}: {:>12}  ({:.4}%)", val, count, *count as f64 / total * 100.0);
    }
}

// Shannon entropy lower bound — what a perfect Huffman/arithmetic coder achieves.
fn entropy_size(data: &[u8]) -> usize {
    let counts = byte_counts(data);
    let total = data.len() as f64;
    let bits: f64 = counts.iter()
        .filter(|&&c| c > 0)
        .map(|&c| c as f64 * -((c as f64 / total).log2()))
        .sum();
    (bits / 8.0).ceil() as usize
}

// Separate WDL (draw/win/loss) from DTZ magnitude, then compute entropy of each.
// WDL has only 3 symbols → ~1.5 bits/entry.
// DTZ has values 1..=23 with a tighter distribution → ~3–4 bits/entry.
fn wdl_dtm_entropy_sizes(data: &[u8]) -> (usize, usize) {
    let (mut draw, mut win, mut loss) = (0u64, 0u64, 0u64);
    let mut dtm_counts = [0u64; 128];
    for &b in data {
        let v = b as i8;
        if v == 0 { draw += 1; }
        else if v > 0 { win += 1; dtm_counts[v as usize] += 1; }
        else          { loss += 1; dtm_counts[(-v) as usize] += 1; }
    }
    let n = data.len() as f64;
    let wdl_bits: f64 = [draw, win, loss].iter().filter(|&&c| c > 0)
        .map(|&c| c as f64 * -((c as f64 / n).log2()))
        .sum();
    let dtm_total: f64 = dtm_counts.iter().sum::<u64>() as f64;
    let dtm_bits: f64 = dtm_counts.iter().filter(|&&c| c > 0)
        .map(|&c| c as f64 * -((c as f64 / dtm_total).log2()))
        .sum();
    (
        (wdl_bits / 8.0).ceil() as usize,
        (dtm_bits / 8.0).ceil() as usize,
    )
}

// Simple (count u8, value u8) RLE.
fn rle_size(data: &[u8]) -> usize {
    if data.is_empty() { return 0; }
    let mut size = 0usize;
    let mut i = 0;
    while i < data.len() {
        let byte = data[i];
        let mut run = 1usize;
        while i + run < data.len() && data[i + run] == byte { run += 1; }
        size += ((run + 254) / 255) * 2;
        i += run;
    }
    size
}

// Zero-specific RLE: non-zero bytes emitted literally (1 byte each),
// zero runs as (0x00, u16 count) = 3 bytes per chunk.
fn zero_rle_size(data: &[u8]) -> usize {
    let mut size = 0;
    let mut i = 0;
    while i < data.len() {
        if data[i] == 0 {
            let mut run = 1usize;
            while i + run < data.len() && data[i + run] == 0 { run += 1; }
            size += ((run + 65534) / 65535) * 3;
            i += run;
        } else {
            size += 1;
            i += 1;
        }
    }
    size
}

// Sparse: non-zero entries only as (u32 index, u8 value).
fn sparse_size(data: &[u8]) -> usize {
    data.iter().filter(|&&b| b != 0).count() * 5
}

// Pair entropy: treat each consecutive pair as a 2-byte symbol.
// If adjacent values are correlated (e.g. (0, nonzero) pairs dominate),
// the pair entropy will be substantially less than 2× single-byte entropy,
// meaning a context model that conditions on the previous byte would help.
fn pair_entropy_size(data: &[u8]) -> usize {
    // Only sample up to 8M pairs to keep this fast.
    const MAX_PAIRS: usize = 8_000_000;
    let step = (data.len() / 2 / MAX_PAIRS).max(1);
    let mut counts = std::collections::HashMap::<(u8, u8), u64>::new();
    let mut sampled = 0u64;
    let mut i = 0;
    while i + 1 < data.len() {
        *counts.entry((data[i], data[i + 1])).or_insert(0) += 1;
        sampled += 1;
        i += 2 * step;
    }
    let total = sampled as f64;
    let bits_per_pair: f64 = counts.values()
        .map(|&c| c as f64 * -((c as f64 / total).log2()))
        .sum::<f64>() / total;
    // Scale bits_per_pair back to the full dataset (pairs cover data.len()/2 entries).
    let pairs_full = (data.len() / 2) as f64;
    (bits_per_pair * pairs_full / 8.0).ceil() as usize
}

fn mb(bytes: usize) -> f64 { bytes as f64 / 1_048_576.0 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn compression_analysis() {
        let tb = load_replacing_unknowns("tablebase").expect("failed to load tablebase");

        println!("\n=== Per-file breakdown ===");
        for (i, file) in tb.iter().enumerate() {
            if file.is_empty() { continue; }
            let zeros  = file.iter().filter(|s| s.0 == 0).count();
            let wins   = file.iter().filter(|s| s.0 > 0).count();
            let losses = file.iter().filter(|s| s.0 < 0).count();
            println!(
                "file {:2}: {:>10} entries  zeros={:>8} ({:5.1}%)  wins={:>7}  losses={:>7}",
                i, file.len(), zeros, zeros as f64 / file.len() as f64 * 100.0, wins, losses,
            );
        }

        let data = flatten(&tb);

        println!("\n=== Overall distribution ===");
        print_distribution(&data);

        let raw               = data.len();
        let rle               = rle_size(&data);
        let zero_rle          = zero_rle_size(&data);
        let sparse            = sparse_size(&data);
        let entropy           = entropy_size(&data);
        let (wdl_e, dtm_e)   = wdl_dtm_entropy_sizes(&data);
        let wdl_dtm_combined  = wdl_e + dtm_e;
        let pair_e            = pair_entropy_size(&data);

        println!("\n=== Compression results / estimates ===");
        println!("(* = theoretical minimum if we add arithmetic/Huffman coding on top)");
        println!();
        println!("Raw:                         {:>12} bytes  ({:.2} MB)  1.00x", raw,             mb(raw));
        println!("RLE (count,val):              {:>12} bytes  ({:.2} MB)  {:.2}x", rle,            mb(rle),            raw as f64 / rle as f64);
        println!("Zero-RLE:                    {:>12} bytes  ({:.2} MB)  {:.2}x", zero_rle,        mb(zero_rle),       raw as f64 / zero_rle as f64);
        println!("Sparse (idx+val):             {:>12} bytes  ({:.2} MB)  {:.2}x", sparse,         mb(sparse),         raw as f64 / sparse as f64);
        println!("Entropy lower bound *:        {:>12} bytes  ({:.2} MB)  {:.2}x", entropy,        mb(entropy),        raw as f64 / entropy as f64);
        println!("WDL entropy *:                {:>12} bytes  ({:.2} MB)  (for all {} entries)", wdl_e, mb(wdl_e), raw);
        println!("DTZ entropy *:                {:>12} bytes  ({:.2} MB)  (for non-zero entries only)", dtm_e, mb(dtm_e));
        println!("WDL + DTZ entropy combined *: {:>12} bytes  ({:.2} MB)  {:.2}x", wdl_dtm_combined, mb(wdl_dtm_combined), raw as f64 / wdl_dtm_combined as f64);
        println!("Pair entropy *:               {:>12} bytes  ({:.2} MB)  {:.2}x  (2x single would be {:.2} MB)", pair_e, mb(pair_e), raw as f64 / pair_e as f64, mb(entropy * 2));
        println!();
        println!("Note: if pair entropy << 2x single entropy, adjacent values are correlated");
        println!("      and a context-conditioned model (e.g. PPM, LSTM) would help further.");
    }
}
