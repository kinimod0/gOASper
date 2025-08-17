use goasper_io::{read_gds_polygons, read_gds_summary, CellPolygons, CellSummary, GdsSummary};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GoasperError {
    #[error("I/O error: {0}")]
    Io(String),
}
impl From<goasper_io::IoError> for GoasperError {
    fn from(e: goasper_io::IoError) -> Self {
        GoasperError::Io(e.to_string())
    }
}

#[derive(Default)]
pub struct Layout {
    summary: Option<GdsSummary>,
    polys: Option<Vec<CellPolygons>>,
}

impl Layout {
    pub fn new() -> Self {
        Self::default()
    }

    /// load a GDS and populate the internal cell list
    pub fn load_gds<P: AsRef<std::path::Path>>(&mut self, p: P) -> Result<(), GoasperError> {
        self.summary = Some(read_gds_summary(&p)?);
        self.polys = Some(read_gds_polygons(p)?);
        Ok(())
    }

    /// TODO: stub: will implement later
    /// exists so the Python API compiles
    pub fn save_oas<P: AsRef<std::path::Path>>(&self, p: P) -> Result<(), GoasperError> {
        let _ = p; // silence until implemented
        Ok(())
    }

    pub fn libname(&self) -> Option<&str> {
        self.summary.as_ref().and_then(|s| s.libname.as_deref())
    }

    pub fn cell_names(&self) -> Vec<String> {
        self.summary
            .as_ref()
            .map(|s| s.cells.iter().map(|c| c.name.clone()).collect())
            .unwrap_or_default()
    }

    pub fn cell_summaries(&self) -> &[CellSummary] {
        self.summary
            .as_ref()
            .map(|s| s.cells.as_slice())
            .unwrap_or(&[])
    }

    /// All polygons grouped per cell (DBU coordinates).
    pub fn polygons(&self) -> &[CellPolygons] {
        self.polys.as_deref().unwrap_or(&[])
    }

    /// Polygons for a single cell by name.
    pub fn polygons_for<'a>(&'a self, cell: &str) -> Option<&'a [goasper_io::Polygon]> {
        self.polys
            .as_ref()?
            .iter()
            .find(|c| c.name == cell)
            .map(|c| c.polys.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_layout() {
        let l = Layout::new();
        assert!(l.cell_names().is_empty());
    }
}
