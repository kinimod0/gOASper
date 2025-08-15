//! Minimal GDSII reader: extracts cell names by scanning BGNSTR..ENDSTR and STRNAME.
use std::{fs::File, io::{Read, BufReader}, path::Path};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IoError {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("Malformed GDS record at offset {offset} (len={len}, rectype={rectype:#04x}, dtype={dtype:#04x})")]
    Malformed { offset: u64, len: u16, rectype: u8, dtype: u8 },
    #[error("Unexpected EOF")]
    Eof,
}

/// GDS record type constants (subset)
const RT_BGNSTR: u8 = 0x05;
const RT_STRNAME: u8 = 0x06;
const RT_ENDSTR:  u8 = 0x07;
const RT_ENDLIB:  u8 = 0x04;

/// Data type codes (see GDSII spec); we only need ASCII here.
const DT_ASCII: u8 = 0x06;

/// Read cell (structure) names from a GDSII file by scanning records.
/// This is tolerant to extra/unknown records and only relies on BGNSTR/STRNAME/ENDSTR.
pub fn read_gds_cell_names<P: AsRef<Path>>(path: P) -> Result<Vec<String>, IoError> {
    let f = File::open(path)?;
    let mut r = BufReader::new(f);
    let mut buf = Vec::with_capacity(1 << 16);

    let mut offset: u64 = 0;
    let mut in_struct = false;
    let mut cells = Vec::new();

    loop {
        // Header: 2 bytes length (BE), 1 byte rectype, 1 byte dtype
        let mut hdr = [0u8; 4];
        if let Err(e) = r.read_exact(&mut hdr) {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                break; // clean EOF
            } else {
                return Err(IoError::Io(e));
            }
        }
        offset += 4;
        let len = u16::from_be_bytes([hdr[0], hdr[1]]);
        let rectype = hdr[2];
        let dtype = hdr[3];

        if len < 4 {
            return Err(IoError::Malformed { offset, len, rectype, dtype });
        }
        let pay = (len - 4) as usize;
        buf.resize(pay, 0);
        if pay > 0 {
            r.read_exact(&mut buf)?;
            offset += pay as u64;
        }

        match rectype {
            RT_BGNSTR => {
                in_struct = true;
            }
            RT_STRNAME if in_struct && dtype == DT_ASCII => {
                // Strings are even-length, null-padded ASCII.
                let s = trim_gds_ascii(&buf);
                if !s.is_empty() {
                    cells.push(s.to_string());
                }
            }
            RT_ENDSTR => {
                in_struct = false;
            }
            RT_ENDLIB => {
                // Stop early when ENDLIB is found.
                break;
            }
            _ => {
                // ignore other records
            }
        }
    }

    Ok(cells)
}

fn trim_gds_ascii(bytes: &[u8]) -> &str {
    // Strip trailing 0x00 padding and any trailing spaces.
    let mut end = bytes.len();
    while end > 0 && (bytes[end - 1] == 0 || bytes[end - 1] == b' ') {
        end -= 1;
    }
    std::str::from_utf8(&bytes[..end]).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Helper to write a GDS record
    fn rec(rectype: u8, dtype: u8, data: &[u8]) -> Vec<u8> {
        let len = (4 + data.len()) as u16;
        let mut v = Vec::with_capacity(len as usize);
        v.extend_from_slice(&len.to_be_bytes());
        v.push(rectype);
        v.push(dtype);
        v.extend_from_slice(data);
        v
    }

    #[test]
    fn reads_cell_names() {
        let mut tmp = NamedTempFile::new().unwrap();
        // Minimal stream: BGNSTR, STRNAME("TOP"), ENDSTR, ENDLIB
        let mut bytes = Vec::new();
        bytes.extend(rec(RT_BGNSTR, 0x02, &[0; 24])); // BGNSTR with dummy dates (12 u16)
        let mut nm = b"TOP".to_vec();
        if nm.len() % 2 != 0 { nm.push(0); }
        bytes.extend(rec(RT_STRNAME, DT_ASCII, &nm));
        bytes.extend(rec(RT_ENDSTR, 0x00, &[]));
        bytes.extend(rec(RT_ENDLIB, 0x00, &[]));
        tmp.write_all(&bytes).unwrap();
        let cells = read_gds_cell_names(tmp.path()).unwrap();
        assert_eq!(cells, vec!["TOP"]);
    }
}
