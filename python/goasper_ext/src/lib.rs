use goasper_core::{GoasperError, Layout};
use pyo3::{exceptions::PyIOError, prelude::*};

fn to_pyerr(e: GoasperError) -> PyErr {
    PyIOError::new_err(e.to_string())
}

#[pyclass]
struct PyLayout {
    inner: Layout,
}

#[pymethods]
impl PyLayout {
    #[new]
    fn new() -> Self {
        Self {
            inner: Layout::new(),
        }
    }

    fn load_gds(&mut self, path: &str) -> PyResult<()> {
        self.inner.load_gds(path).map_err(to_pyerr)?; // GoasperError -> PyErr
        Ok(())
    }

    fn save_oas(&self, path: &str) -> PyResult<()> {
        self.inner.save_oas(path).map_err(to_pyerr)?;
        Ok(())
    }

    fn cell_names(&self) -> Vec<String> {
        self.inner.cell_names().to_vec()
    }
}

#[pymodule]
fn _lowlevel(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyLayout>()?;
    Ok(())
}
