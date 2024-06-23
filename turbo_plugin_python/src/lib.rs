use pyo3::{
    prelude::*,
    types::{PyFunction, PyList},
};
use python_numeric::PythonByte;
use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex},
};
mod python_numeric;

#[derive(Clone, Debug, Default)]
#[pyclass]
pub struct Color {
    #[pyo3(get, set)]
    pub red: PythonByte,
    #[pyo3(get, set)]
    pub blue: PythonByte,
    #[pyo3(get)]
    pub green: PythonByte,
}

#[pymethods]
impl Color {
    #[new]
    fn new() -> Self {
        Self::default()
    }
    #[setter]
    fn red(&mut self, value: &PyAny) -> PyResult<()> {
    }
    #[setter]
    fn blue(&mut self, value: &PyAny) -> PyResult<()> {}
    #[setter]
    fn green(&mut self, value: &PyAny) -> PyResult<()> {}
}

#[derive(Debug)]
pub struct PythonEffect {
    pub update_fn: Py<PyFunction>,
    pub colors: Py<PyList>,
}

impl PythonEffect {
    pub fn new(file_path: &Path) -> PythonEffect {
        let code = fs::read_to_string(file_path).unwrap();
        let (update_fn, colors) = Python::with_gil(|py| {
            let update_fn = PyModule::from_code(py, code.as_str(), "", "")
                .unwrap()
                .getattr("update")
                .unwrap()
                .extract()
                .unwrap();
            let colors = PyList::empty(py).extract().unwrap();
            (update_fn, colors)
        });
        PythonEffect { update_fn, colors }
    }
}

#[pyfunction]
pub fn rusty(x: usize) -> usize {
    x * 2
}

#[derive(Default)]
#[pyclass]
pub struct FftResult {
    raw_data: Arc<Mutex<Vec<i32>>>,
}

#[pymethods]
impl FftResult {
    pub fn get_max_amplitude(&self) -> i32 {
        *self
            .raw_data
            .lock()
            .unwrap()
            .iter()
            .max()
            .or_else(|| Some(&0))
            .unwrap()
    }
}

impl FftResult {
    pub fn new(fft_results: Arc<Mutex<Vec<i32>>>) -> Self {
        FftResult {
            raw_data: fft_results,
        }
    }
}

#[pymodule]
#[pyo3(name = "turbo_python")]
fn python_api(_py: Python, module: &PyModule) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(rusty, module)?)?;
    module.add_class::<Color>()?;
    Ok(())
}

pub fn initialize_python_interpreter(fft_results: Arc<Mutex<Vec<i32>>>) {
    pyo3::append_to_inittab!(python_api);
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let module = py.import("turbo_python").unwrap();
        module
            .add("fft_result", FftResult::new(fft_results))
            .unwrap();
    });
}

