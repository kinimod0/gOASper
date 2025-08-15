use pyo3::prelude::*;
use goasper_core::Layout;

#[pyclass]
struct PyLayout { inner: Layout }

#[pymethods]
impl PyLayout {
    #[new]
    fn new() -> Self { Self { inner: Layout::new() } }

    fn load_gds(&mut self, path: &str) -> PyResult<()> {
        self.inner.load_gds(path).map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    fn save_oas(&self, path: &str) -> PyResult<()> {
        self.inner.save_oas(path).map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    /// Return the list of cell names as a Python list[str].
    fn cell_names(&self) -> Vec<String> {
        self.inner.cell_names().to_vec()
    }
}

#[pymodule]
fn _lowlevel(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyLayout>()?;
    Ok(())
}
