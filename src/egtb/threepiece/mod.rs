use std::io::{BufReader, BufWriter, Read, Write};

use crate::egtb::threepiece::{generator::Status, pos::Pos};

pub mod generator;
pub mod makerevmove;
pub mod pos;
pub mod reflection;
pub mod revmove;
pub mod revmovegen;

pub(crate) type Files<T> = [Vec<T>; Pos::NUM_FILES];

pub fn save_tablebase(status: &Files<Status>, path: &str) -> std::io::Result<()> {
    let mut w = BufWriter::new(std::fs::File::create(path)?);
    for s in status {
        w.write_all(&(s.len() as u32).to_le_bytes())?;
    }
    for s in status {
        let bytes: &[u8] = unsafe { std::slice::from_raw_parts(s.as_ptr() as *const u8, s.len()) };
        w.write_all(bytes)?;
    }
    Ok(())
}

pub fn load_tablebase(path: &str) -> std::io::Result<Files<Status>> {
    let mut r = BufReader::new(std::fs::File::open(path)?);
    let mut lengths = [0u32; Pos::NUM_FILES];
    for l in &mut lengths {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        *l = u32::from_le_bytes(buf);
    }
    let mut status: Files<Status> = std::array::from_fn(|_| Vec::new());
    for (s, &len) in status.iter_mut().zip(lengths.iter()) {
        let mut buf = vec![0u8; len as usize];
        r.read_exact(&mut buf)?;
        *s = unsafe { std::mem::transmute(buf) };
    }
    Ok(status)
}