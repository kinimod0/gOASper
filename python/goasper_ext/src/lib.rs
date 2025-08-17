use goasper_core::{GoasperError, Layout};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

fn to_pyerr(e: GoasperError) -> PyErr {
    pyo3::exceptions::PyIOError::new_err(e.to_string())
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
        self.inner.load_gds(path).map_err(to_pyerr)?;
        Ok(())
    }

    fn save_oas(&self, path: &str) -> PyResult<()> {
        self.inner.save_oas(path).map_err(to_pyerr)?;
        Ok(())
    }

    fn cell_names(&self) -> Vec<String> {
        self.inner.cell_names()
    }

    fn summary<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let out = PyDict::new(py);
        out.set_item("libname", self.inner.libname())?;

        let cells = PyList::empty(py);
        for c in self.inner.cell_summaries() {
            let d = PyDict::new(py);
            d.set_item("name", &c.name)?;
            if let Some(bb) = c.bbox {
                d.set_item("bbox", (bb.xmin, bb.ymin, bb.xmax, bb.ymax))?;
            } else {
                d.set_item("bbox", py.None())?;
            }
            d.set_item("total_polys", c.total_polys)?;
            let lp = PyDict::new(py);
            for ((lay, dt), cnt) in &c.layer_poly_counts {
                lp.set_item(format!("{},{}", lay, dt), *cnt)?;
            }
            d.set_item("layer_poly_counts", lp)?;
            cells.append(d)?;
        }
        out.set_item("cells", cells)?;
        Ok(out)
    }

    /// Return polygons grouped per cell as:
    /// [{"name": str, "polys": [{"layer":int,"datatype":int,"xy":[(x,y),..]}]}]
    fn polygons<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let cells_out = PyList::empty(py);
        for c in self.inner.polygons() {
            let d = PyDict::new(py);
            d.set_item("name", &c.name)?;
            let polys = PyList::empty(py);
            for p in &c.polys {
                let pd = PyDict::new(py);
                pd.set_item("layer", p.layer)?;
                pd.set_item("datatype", p.datatype)?;
                // Convert points to a Python list of (x,y) tuples
                let pts = PyList::empty(py);
                for (x, y) in &p.xy {
                    pts.append((*x, *y))?;
                }
                pd.set_item("xy", pts)?;
                polys.append(pd)?;
            }
            d.set_item("polys", polys)?;
            cells_out.append(d)?;
        }
        Ok(cells_out)
    }
}

#[pymodule]
fn _lowlevel(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyLayout>()?;
    Ok(())
}
