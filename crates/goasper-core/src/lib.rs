use thiserror::Error;

#[derive(Debug, Error)]
pub enum GoasperError {
    #[error("I/O error: {0}")]
    Io(String),
}

impl From<goasper_io::IoError> for GoasperError {
    fn from(e: goasper_io::IoError) -> Self { GoasperError::Io(e.to_string()) }
}

#[derive(Default)]
pub struct Layout {
    cells: Vec<String>,
}

impl Layout {
    pub fn new() -> Self { Self::default() }

    /// load a GDS and populate the internal cell list
    pub fn load_gds<P: AsRef<std::path::Path>>(&mut self, p: P) -> Result<(), GoasperError> {
        let list = goasper_io::read_gds_cell_names(p)?;
        self.cells = list;
        Ok(())
    }

    /// return a snapshot of known cell names
    pub fn cell_names(&self) -> &[String] { &self.cells }

    /// TODO: stub: will implement later
    /// exists so the Python API compiles
    pub fn save_oas<P: AsRef<std::path::Path>>(&self, _p: P) -> Result<(), GoasperError> {
        Ok(())
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
