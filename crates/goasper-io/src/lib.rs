//! Minimal GDSII reader: extracts cell names by scanning BGNSTR..ENDSTR and STRNAME.

// GDS record types
const RT_BGNSTR: u8 = 0x05;
const RT_STRNAME: u8 = 0x06;
const RT_ENDSTR: u8 = 0x07;
const RT_ENDLIB: u8 = 0x04;
const RT_LIBNAME: u8 = 0x02;
const RT_BOUNDARY: u8 = 0x08;
const RT_LAYER: u8 = 0x0D;
const RT_DATATYPE: u8 = 0x0E;
const RT_XY: u8 = 0x10;
const RT_ENDEL: u8 = 0x11;

// datatypes
const DT_INT2: u8 = 0x02;
const DT_INT4: u8 = 0x03;
const DT_ASCII: u8 = 0x06;

use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
    path::Path,
};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BBox {
    pub xmin: i32,
    pub ymin: i32,
    pub xmax: i32,
    pub ymax: i32,
}
impl BBox {
    pub fn include_pt(&mut self, x: i32, y: i32) {
        if self.xmin > self.xmax && self.ymin > self.ymax {
            self.xmin = x;
            self.xmax = x;
            self.ymin = y;
            self.ymax = y;
        } else {
            self.xmin = self.xmin.min(x);
            self.xmax = self.xmax.max(x);
            self.ymin = self.ymin.min(y);
            self.ymax = self.ymax.max(y);
        }
    }
    pub fn include_bbox(&mut self, o: &BBox) {
        self.include_pt(o.xmin, o.ymin);
        self.include_pt(o.xmax, o.ymax);
    }
    pub fn is_valid(&self) -> bool {
        self.xmin <= self.xmax && self.ymin <= self.ymax
    }
}

#[derive(Debug, Default)]
pub struct CellSummary {
    pub name: String,
    pub bbox: Option<BBox>, // DBU coordinates
    pub layer_poly_counts: HashMap<(u16, u16), usize>,
    pub total_polys: usize,
}

#[derive(Debug, Default)]
pub struct GdsSummary {
    pub libname: Option<String>,
    pub cells: Vec<CellSummary>,
}

#[derive(Debug, Error)]
pub enum IoError {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("Malformed GDS record at offset {offset} (len={len}, rectype={rectype:#04x}, dtype={dtype:#04x})")]
    Malformed {
        offset: u64,
        len: u16,
        rectype: u8,
        dtype: u8,
    },
    #[error("Unexpected EOF")]
    Eof,
}

#[derive(Debug, Clone)]
pub struct Polygon {
    pub layer: u16,
    pub datatype: u16,
    /// Closed polygon in DBU; last point NOT duplicated.
    pub xy: Vec<(i32, i32)>,
}

#[derive(Debug, Default)]
pub struct CellPolygons {
    pub name: String,
    pub polys: Vec<Polygon>,
}

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
            return Err(IoError::Malformed {
                offset,
                len,
                rectype,
                dtype,
            });
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

/// Read polygons (BOUNDARY only) grouped per cell.
/// Returns Vec<CellPolygons> in the order cells appear in the stream.
pub fn read_gds_polygons<P: AsRef<Path>>(path: P) -> Result<Vec<CellPolygons>, IoError> {
    let f = File::open(path)?;
    let mut r = BufReader::new(f);
    let mut buf = Vec::with_capacity(1 << 16);

    let mut out: Vec<CellPolygons> = Vec::new();

    // parser state
    let mut in_struct = false;
    let mut cur_cell: Option<CellPolygons> = None;

    let mut in_boundary = false;
    let mut cur_layer: u16 = 0;
    let mut cur_dtype: u16 = 0;
    let mut cur_xy: Vec<(i32, i32)> = Vec::new();

    loop {
        let mut hdr = [0u8; 4];
        if let Err(e) = r.read_exact(&mut hdr) {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                break;
            } else {
                return Err(IoError::Io(e));
            }
        }
        let len = u16::from_be_bytes([hdr[0], hdr[1]]);
        let rectype = hdr[2];
        let dtype = hdr[3];
        if len < 4 {
            return Err(IoError::Malformed {
                offset: 0,
                len,
                rectype,
                dtype,
            });
        }
        let pay = (len - 4) as usize;
        buf.resize(pay, 0);
        if pay > 0 {
            r.read_exact(&mut buf)?;
        }

        match rectype {
            RT_BGNSTR => { in_struct = true; cur_cell = Some(CellPolygons::default()); }
            RT_STRNAME if in_struct && dtype == DT_ASCII => {
                if let Some(c) = cur_cell.as_mut() {
                    c.name = trim_gds_ascii(&buf).to_string();
                }
            }
            RT_BOUNDARY => {
                in_boundary = true;
                cur_layer = 0;
                cur_dtype = 0;
                cur_xy.clear();
            }
            // layer/datatype (optional in some files, default 0)
            0x0D /* RT_LAYER */ if in_boundary && dtype == DT_INT2 => {
                if buf.len() >= 2 { cur_layer = u16::from_be_bytes([buf[0], buf[1]]); }
            }
            0x0E /* RT_DATATYPE */ if in_boundary && dtype == DT_INT2 => {
                if buf.len() >= 2 { cur_dtype = u16::from_be_bytes([buf[0], buf[1]]); }
            }
            0x10 /* RT_XY */ if in_boundary && dtype == 0x03 /* DT_INT4 */ => {
                if buf.len() % 8 == 0 {
                    cur_xy.clear();
                    let n = buf.len() / 8;
                    // Decode all pairs, drop the duplicate closing point if present
                    let x0 = i32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
                    let y0 = i32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
                    for i in 0..n {
                        let x = i32::from_be_bytes([buf[8*i], buf[8*i+1], buf[8*i+2], buf[8*i+3]]);
                        let y = i32::from_be_bytes([buf[8*i+4], buf[8*i+5], buf[8*i+6], buf[8*i+7]]);
                        if i + 1 == n && n >= 2 && x == x0 && y == y0 {
                            break; // drop duplicate close
                        }
                        cur_xy.push((x, y));
                    }
                }
            }
            0x11 /* RT_ENDEL */ if in_boundary => {
                if let Some(c) = cur_cell.as_mut() {
                    if !cur_xy.is_empty() {
                        c.polys.push(Polygon { layer: cur_layer, datatype: cur_dtype, xy: cur_xy.clone() });
                    }
                }
                in_boundary = false;
                cur_xy.clear();
            }
            RT_ENDSTR => {
                in_struct = false;
                if let Some(c) = cur_cell.take() {
                    if !c.name.is_empty() {
                        out.push(c);
                    }
                }
            }
            RT_ENDLIB => break,
            _ => {}
        }
    }

    Ok(out)
}

fn trim_gds_ascii(bytes: &[u8]) -> &str {
    // Strip trailing 0x00 padding and any trailing spaces.
    let mut end = bytes.len();
    while end > 0 && (bytes[end - 1] == 0 || bytes[end - 1] == b' ') {
        end -= 1;
    }
    std::str::from_utf8(&bytes[..end]).unwrap_or("")
}

/// Stream a GDS and summarize: libname, per-cell bbox, per-layer polygon counts
/// Coordinates are raw DBU (GDS integer units); Only BOUNDARY is counted in this MVP
pub fn read_gds_summary<P: AsRef<Path>>(path: P) -> Result<GdsSummary, IoError> {
    let f = File::open(path)?;
    let mut r = BufReader::new(f);
    let mut buf = Vec::with_capacity(1 << 16);

    let mut offset: u64 = 0;
    let mut s = GdsSummary::default();

    // parser state
    let mut in_struct = false;
    let mut cur: Option<CellSummary> = None;

    let mut in_boundary = false;
    let mut cur_layer: Option<u16> = None;
    let mut cur_dtype: Option<u16> = None;
    let mut cur_poly_bbox: Option<BBox> = None;

    loop {
        let mut hdr = [0u8; 4];
        if let Err(e) = r.read_exact(&mut hdr) {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                break;
            } else {
                return Err(IoError::Io(e));
            }
        }
        offset += 4;
        let len = u16::from_be_bytes([hdr[0], hdr[1]]);
        let rectype = hdr[2];
        let dtype = hdr[3];
        if len < 4 {
            return Err(IoError::Malformed {
                offset,
                len,
                rectype,
                dtype,
            });
        }

        let pay = (len - 4) as usize;
        buf.resize(pay, 0);
        if pay > 0 {
            r.read_exact(&mut buf)?;
            offset += pay as u64;
        }

        match rectype {
            RT_LIBNAME if dtype == DT_ASCII => {
                s.libname = Some(trim_gds_ascii(&buf).to_string());
            }
            RT_BGNSTR => {
                in_struct = true;
                cur = Some(CellSummary::default());
            }
            RT_STRNAME if in_struct && dtype == DT_ASCII => {
                if let Some(c) = cur.as_mut() {
                    c.name = trim_gds_ascii(&buf).to_string();
                }
            }
            RT_BOUNDARY => {
                in_boundary = true;
                cur_layer = None;
                cur_dtype = None;
                cur_poly_bbox = None;
            }
            RT_LAYER if in_boundary && dtype == DT_INT2 => {
                if buf.len() >= 2 {
                    cur_layer = Some(u16::from_be_bytes([buf[0], buf[1]]));
                }
            }
            RT_DATATYPE if in_boundary && dtype == DT_INT2 => {
                if buf.len() >= 2 {
                    cur_dtype = Some(u16::from_be_bytes([buf[0], buf[1]]));
                }
            }
            RT_XY if in_boundary && dtype == DT_INT4 => {
                // parse i32 pairs; last point duplicates first → skip it
                if buf.len() % 8 == 0 && !buf.is_empty() {
                    let n = buf.len() / 8;
                    let x0 = i32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
                    let y0 = i32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
                    let mut bb = BBox {
                        xmin: i32::MAX,
                        ymin: i32::MAX,
                        xmax: i32::MIN,
                        ymax: i32::MIN,
                    };
                    for i in 0..n {
                        let x = i32::from_be_bytes([
                            buf[8 * i],
                            buf[8 * i + 1],
                            buf[8 * i + 2],
                            buf[8 * i + 3],
                        ]);
                        let y = i32::from_be_bytes([
                            buf[8 * i + 4],
                            buf[8 * i + 5],
                            buf[8 * i + 6],
                            buf[8 * i + 7],
                        ]);
                        if i + 1 == n && n >= 2 && x == x0 && y == y0 {
                            break;
                        }
                        bb.include_pt(x, y);
                    }
                    if bb.is_valid() {
                        cur_poly_bbox = Some(bb);
                    }
                }
            }
            RT_ENDEL if in_boundary => {
                if let Some(c) = cur.as_mut() {
                    let lay = cur_layer.unwrap_or(0);
                    let dt = cur_dtype.unwrap_or(0);
                    *c.layer_poly_counts.entry((lay, dt)).or_insert(0) += 1;
                    c.total_polys += 1;
                    if let Some(pb) = cur_poly_bbox {
                        if let Some(cb) = c.bbox.as_mut() {
                            cb.include_bbox(&pb);
                        } else {
                            c.bbox = Some(pb);
                        }
                    }
                }
                in_boundary = false;
                cur_layer = None;
                cur_dtype = None;
                cur_poly_bbox = None;
            }
            RT_ENDSTR => {
                in_struct = false;
                if let Some(c) = cur.take() {
                    if !c.name.is_empty() {
                        s.cells.push(c);
                    }
                }
            }
            RT_ENDLIB => break,
            _ => {}
        }
    }

    Ok(s)
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
        if nm.len() % 2 != 0 {
            nm.push(0);
        }
        bytes.extend(rec(RT_STRNAME, DT_ASCII, &nm));
        bytes.extend(rec(RT_ENDSTR, 0x00, &[]));
        bytes.extend(rec(RT_ENDLIB, 0x00, &[]));
        tmp.write_all(&bytes).unwrap();
        let cells = read_gds_cell_names(tmp.path()).unwrap();
        assert_eq!(cells, vec!["TOP"]);
    }

    #[test]
    fn summary_has_lib_bbox_and_counts() {
        use std::io::Write;
        let mut tmp = NamedTempFile::new().unwrap();

        // helpers
        fn rec(rectype: u8, dtype: u8, data: &[u8]) -> Vec<u8> {
            let len = (4 + data.len()) as u16;
            let mut v = Vec::with_capacity(len as usize);
            v.extend_from_slice(&len.to_be_bytes());
            v.push(rectype);
            v.push(dtype);
            v.extend_from_slice(data);
            v
        }
        fn be_i32(x: i32) -> [u8; 4] {
            x.to_be_bytes()
        }

        // craft a tiny stream: LIBNAME, one struct "TOP" with a rectangle on (1,0)
        let mut bytes = Vec::new();
        let mut lib = b"LIB".to_vec();
        if lib.len() % 2 != 0 {
            lib.push(0);
        }
        bytes.extend(rec(RT_LIBNAME, DT_ASCII, &lib));
        bytes.extend(rec(RT_BGNSTR, 0x02, &[0; 24]));
        let mut nm = b"TOP".to_vec();
        if nm.len() % 2 != 0 {
            nm.push(0);
        }
        bytes.extend(rec(RT_STRNAME, DT_ASCII, &nm));
        bytes.extend(rec(RT_BOUNDARY, 0, &[]));
        bytes.extend(rec(RT_LAYER, DT_INT2, &1u16.to_be_bytes()));
        bytes.extend(rec(RT_DATATYPE, DT_INT2, &0u16.to_be_bytes()));
        // box (0,0)-(10,5)
        let mut xy = Vec::new();
        for (x, y) in &[(0, 0), (10, 0), (10, 5), (0, 5), (0, 0)] {
            xy.extend_from_slice(&be_i32(*x));
            xy.extend_from_slice(&be_i32(*y));
        }
        bytes.extend(rec(RT_XY, DT_INT4, &xy));
        bytes.extend(rec(RT_ENDEL, 0, &[]));
        bytes.extend(rec(RT_ENDSTR, 0, &[]));
        bytes.extend(rec(RT_ENDLIB, 0, &[]));
        tmp.write_all(&bytes).unwrap();

        let s = read_gds_summary(tmp.path()).unwrap();
        assert_eq!(s.libname.as_deref(), Some("LIB"));
        assert_eq!(s.cells.len(), 1);
        let c = &s.cells[0];
        assert_eq!(c.name, "TOP");
        assert_eq!(c.total_polys, 1);
        assert_eq!(c.layer_poly_counts.get(&(1, 0)).copied(), Some(1));
        assert_eq!(
            c.bbox.unwrap(),
            BBox {
                xmin: 0,
                ymin: 0,
                xmax: 10,
                ymax: 5
            }
        );
    }

    #[test]
    fn polys_roundtrip_minimal() {
        use std::io::Write;
        let mut tmp = NamedTempFile::new().unwrap();

        fn rec(rt: u8, dt: u8, data: &[u8]) -> Vec<u8> {
            let len = (4 + data.len()) as u16;
            let mut v = Vec::with_capacity(len as usize);
            v.extend_from_slice(&len.to_be_bytes());
            v.push(rt);
            v.push(dt);
            v.extend_from_slice(data);
            v
        }
        fn be_i32(x: i32) -> [u8; 4] {
            x.to_be_bytes()
        }

        let mut bytes = Vec::new();
        // one struct TOP with one rectangle on layer 1/0
        bytes.extend(rec(RT_BGNSTR, 0x02, &[0; 24]));
        let mut nm = b"TOP".to_vec();
        if nm.len() % 2 != 0 {
            nm.push(0)
        };
        bytes.extend(rec(RT_STRNAME, DT_ASCII, &nm));
        bytes.extend(rec(RT_BOUNDARY, 0, &[]));
        bytes.extend(rec(0x0D, 0x02, &1u16.to_be_bytes())); // LAYER
        bytes.extend(rec(0x0E, 0x02, &0u16.to_be_bytes())); // DATATYPE
        let mut xy = Vec::new();
        for (x, y) in &[(0, 0), (10, 0), (10, 5), (0, 5), (0, 0)] {
            xy.extend_from_slice(&be_i32(*x));
            xy.extend_from_slice(&be_i32(*y));
        }
        bytes.extend(rec(0x10, 0x03, &xy)); // XY
        bytes.extend(rec(0x11, 0, &[])); // ENDEL
        bytes.extend(rec(RT_ENDSTR, 0, &[]));
        bytes.extend(rec(RT_ENDLIB, 0, &[]));
        tmp.write_all(&bytes).unwrap();

        let cells = read_gds_polygons(tmp.path()).unwrap();
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].name, "TOP");
        assert_eq!(cells[0].polys.len(), 1);
        assert_eq!(cells[0].polys[0].layer, 1);
        assert_eq!(cells[0].polys[0].datatype, 0);
        assert_eq!(cells[0].polys[0].xy, vec![(0, 0), (10, 0), (10, 5), (0, 5)]);
    }
}
