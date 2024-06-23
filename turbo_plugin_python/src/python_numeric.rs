use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use pyo3::class::basic::CompareOp;
use pyo3::exceptions::{PyValueError, PyZeroDivisionError};
use pyo3::prelude::*;

fn wrap<'py>(obj: &'py PyAny) -> PyResult<u8> {
    let val = obj.call_method1("__and__", (0xFF_u8,))?;
    let val: u32 = val.extract()?;
    Ok(val as u8)
}

#[pyclass]
#[derive(Clone, Debug, Default)]
pub struct PythonByte(u8);

#[pymethods]
impl PythonByte {
    #[new]
    fn new(#[pyo3(from_py_with = "wrap")] value: u8) -> Self {
        Self(value)
    }

    fn __repr__(&self) -> PyResult<String> {
        // Get the class name dynamically in case `Number` is subclassed
        Ok(format!("Number {}", self.0))
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        hasher.finish()
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> PyResult<bool> {
        match op {
            CompareOp::Lt => Ok(self.0 < other.0),
            CompareOp::Le => Ok(self.0 <= other.0),
            CompareOp::Eq => Ok(self.0 == other.0),
            CompareOp::Ne => Ok(self.0 != other.0),
            CompareOp::Gt => Ok(self.0 > other.0),
            CompareOp::Ge => Ok(self.0 >= other.0),
        }
    }

    fn __bool__(&self) -> bool {
        self.0 != 0
    }

    fn __add__(&self, #[pyo3(from_py_with = "wrap")] other: u8) -> Self {
        Self(self.0.wrapping_add(other))
    }

    fn __sub__(&self, #[pyo3(from_py_with = "wrap")] other: u8) -> Self {
        Self(self.0.wrapping_sub(other))
    }

    fn __mul__(&self, #[pyo3(from_py_with = "wrap")] other: u8) -> Self {
        Self(self.0.wrapping_mul(other))
    }

    fn __truediv__(&self, #[pyo3(from_py_with = "wrap")] other: u8) -> PyResult<Self> {
        match self.0.checked_div(other) {
            Some(i) => Ok(Self(i)),
            None => Err(PyZeroDivisionError::new_err("division by zero")),
        }
    }

    fn __floordiv__(&self, #[pyo3(from_py_with = "wrap")] other: u8) -> PyResult<Self> {
        match self.0.checked_div(other) {
            Some(i) => Ok(Self(i)),
            None => Err(PyZeroDivisionError::new_err("division by zero")),
        }
    }

    fn __rshift__(&self, #[pyo3(from_py_with = "wrap")] other: u8) -> PyResult<Self> {
        match other.try_into() {
            Ok(rhs) => Ok(Self(self.0.wrapping_shr(rhs))),
            Err(_) => Err(PyValueError::new_err("negative shift count")),
        }
    }

    fn __lshift__(&self, other: &Self) -> PyResult<Self> {
        match other.0.try_into() {
            Ok(rhs) => Ok(Self(self.0.wrapping_shl(rhs))),
            Err(_) => Err(PyValueError::new_err("negative shift count")),
        }
    }

    fn __xor__(&self, other: &Self) -> Self {
        Self(self.0 ^ other.0)
    }

    fn __or__(&self, other: &Self) -> Self {
        Self(self.0 | other.0)
    }

    fn __and__(&self, other: &Self) -> Self {
        Self(self.0 & other.0)
    }

    fn __int__(&self) -> i32 {
        self.0 as i32
    }

    fn __float__(&self) -> f64 {
        self.0 as f64
    }
}
