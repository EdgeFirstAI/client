// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

use pyo3::{
    prelude::*,
    types::{PyDateTime, PyDict},
};
use std::{collections::HashMap, fmt::Display, path::PathBuf, str::FromStr, sync::Arc};
use tokio::sync::mpsc;

/// Emit a deprecation warning for uid() methods.
///
/// This function calls Python's warnings.warn() to notify users that the
/// uid() method is deprecated and will be removed in a future version.
fn warn_uid_deprecated(py: Python<'_>, type_name: &str) -> PyResult<()> {
    let warnings = py.import("warnings")?;
    let message = format!(
        "{}.uid is deprecated and will be removed in a future version. \
         Use str({}.id) instead.",
        type_name, type_name
    );
    warnings.call_method1(
        "warn",
        (
            message,
            py.get_type::<pyo3::exceptions::PyDeprecationWarning>(),
        ),
    )?;
    Ok(())
}

/// Emit a deprecation warning for methods that take client as a parameter.
///
/// This function calls Python's warnings.warn() to notify users that passing
/// `client` to a method is deprecated. The new API embeds the client reference
/// internally.
fn warn_method_deprecated(py: Python<'_>, type_name: &str, method_name: &str) -> PyResult<()> {
    let warnings = py.import("warnings")?;
    let message = format!(
        "{}.{}(client, ...) is deprecated and will be removed in v3.0.0. \
         Use {}.{}(...) without the client parameter instead.",
        type_name, method_name, type_name, method_name
    );
    warnings.call_method1(
        "warn",
        (
            message,
            py.get_type::<pyo3::exceptions::PyDeprecationWarning>(),
        ),
    )?;
    Ok(())
}

#[cfg(feature = "polars")]
use pyo3_polars::PyDataFrame;

pub enum Error {
    Error(edgefirst_client::Error),
    PyErr(pyo3::PyErr),
    TypeError(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Error(err) => write!(f, "{:?}", err),
            Error::PyErr(err) => write!(f, "PyErr: {:?}", err),
            Error::TypeError(msg) => write!(f, "TypeError: {}", msg),
        }
    }
}

impl From<edgefirst_client::Error> for Error {
    fn from(err: edgefirst_client::Error) -> Self {
        Error::Error(err)
    }
}

impl From<pyo3::PyErr> for Error {
    fn from(err: pyo3::PyErr) -> Self {
        Error::PyErr(err)
    }
}

impl From<Error> for PyErr {
    fn from(err: Error) -> PyErr {
        pyo3::exceptions::PyRuntimeError::new_err(format!("{}", err))
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub enum Parameter {
    Integer(i64),
    Real(f64),
    Boolean(bool),
    String(String),
    Array(Vec<Parameter>),
    Object(HashMap<String, Parameter>),
}

#[pymethods]
impl Parameter {
    /// Create an Integer parameter
    #[staticmethod]
    fn integer(value: i64) -> Self {
        Parameter::Integer(value)
    }

    /// Create a Real (float) parameter
    #[staticmethod]
    fn real(value: f64) -> Self {
        Parameter::Real(value)
    }

    /// Create a Boolean parameter
    #[staticmethod]
    fn boolean(value: bool) -> Self {
        Parameter::Boolean(value)
    }

    /// Create a String parameter
    #[staticmethod]
    fn string(value: String) -> Self {
        Parameter::String(value)
    }

    /// Create an Array parameter from a Python list
    #[staticmethod]
    fn array(values: Vec<Bound<'_, PyAny>>) -> PyResult<Self> {
        let mut vec = Vec::with_capacity(values.len());
        for item in values {
            vec.push(
                item.try_into()
                    .map_err(|e: Error| pyo3::exceptions::PyTypeError::new_err(format!("{}", e)))?,
            );
        }
        Ok(Parameter::Array(vec))
    }

    /// Create an Object (dict) parameter from a Python dict
    #[staticmethod]
    fn object(values: HashMap<String, Bound<'_, PyAny>>) -> PyResult<Self> {
        let mut map = HashMap::with_capacity(values.len());
        for (k, item) in values {
            map.insert(
                k,
                item.try_into()
                    .map_err(|e: Error| pyo3::exceptions::PyTypeError::new_err(format!("{}", e)))?,
            );
        }
        Ok(Parameter::Object(map))
    }

    /// Check if this is an Integer parameter
    fn is_integer(&self) -> bool {
        matches!(self, Parameter::Integer(_))
    }

    /// Check if this is a Real parameter
    fn is_real(&self) -> bool {
        matches!(self, Parameter::Real(_))
    }

    /// Check if this is a Boolean parameter
    fn is_boolean(&self) -> bool {
        matches!(self, Parameter::Boolean(_))
    }

    /// Check if this is a String parameter
    fn is_string(&self) -> bool {
        matches!(self, Parameter::String(_))
    }

    /// Check if this is an Array parameter
    fn is_array(&self) -> bool {
        matches!(self, Parameter::Array(_))
    }

    /// Check if this is an Object parameter
    fn is_object(&self) -> bool {
        matches!(self, Parameter::Object(_))
    }

    /// Get the variant type name
    fn type_name(&self) -> &'static str {
        match self {
            Parameter::Integer(_) => "Integer",
            Parameter::Real(_) => "Real",
            Parameter::Boolean(_) => "Boolean",
            Parameter::String(_) => "String",
            Parameter::Array(_) => "Array",
            Parameter::Object(_) => "Object",
        }
    }

    /// Get the integer value (returns None if not an Integer)
    fn as_integer(&self) -> Option<i64> {
        match self {
            Parameter::Integer(v) => Some(*v),
            _ => None,
        }
    }

    /// Get the real value (returns None if not a Real)
    fn as_real(&self) -> Option<f64> {
        match self {
            Parameter::Real(v) => Some(*v),
            _ => None,
        }
    }

    /// Get the boolean value (returns None if not a Boolean)
    fn as_boolean(&self) -> Option<bool> {
        match self {
            Parameter::Boolean(v) => Some(*v),
            _ => None,
        }
    }

    /// Get the string value (returns None if not a String)
    fn as_string(&self) -> Option<String> {
        match self {
            Parameter::String(v) => Some(v.clone()),
            _ => None,
        }
    }

    /// Get the variant type name ("Integer", "Real", "Boolean", "String",
    /// "Array", "Object")
    fn variant_type(&self) -> &str {
        match self {
            Parameter::Integer(_) => "Integer",
            Parameter::Real(_) => "Real",
            Parameter::Boolean(_) => "Boolean",
            Parameter::String(_) => "String",
            Parameter::Array(_) => "Array",
            Parameter::Object(_) => "Object",
        }
    }

    /// Helper to convert a Parameter to a Python object recursively
    fn to_pyobject(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match self {
            Parameter::Integer(i) => Ok((*i).into_pyobject(py)?.into_any().unbind()),
            Parameter::Real(r) => Ok((*r).into_pyobject(py)?.into_any().unbind()),
            Parameter::Boolean(b) => Ok((*b).into_pyobject(py)?.to_owned().into_any().unbind()),
            Parameter::String(s) => Ok(s.as_str().into_pyobject(py)?.into_any().unbind()),
            Parameter::Array(_) => self
                .as_array(py)
                .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Failed to convert array")),
            Parameter::Object(_) => self
                .as_object(py)
                .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Failed to convert object")),
        }
    }

    /// Get the array as a Python list with native Python types (returns None if
    /// not an Array)
    fn as_array(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        match self {
            Parameter::Array(v) => {
                let list = pyo3::types::PyList::empty(py);
                for item in v {
                    let py_value = match item {
                        Parameter::Integer(i) => (*i).into_pyobject(py).ok()?.into_any().unbind(),
                        Parameter::Real(r) => (*r).into_pyobject(py).ok()?.into_any().unbind(),
                        Parameter::Boolean(b) => {
                            (*b).into_pyobject(py).ok()?.to_owned().into_any().unbind()
                        }
                        Parameter::String(s) => {
                            s.as_str().into_pyobject(py).ok()?.into_any().unbind()
                        }
                        Parameter::Array(_) => item.as_array(py)?,
                        Parameter::Object(_) => item.as_object(py)?,
                    };
                    list.append(py_value).ok()?;
                }
                Some(list.unbind().into_any())
            }
            _ => None,
        }
    }

    /// Get the object as a Python dict with native Python types (returns None
    /// if not an Object)
    fn as_object(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        match self {
            Parameter::Object(v) => {
                let dict = pyo3::types::PyDict::new(py);
                for (k, item) in v {
                    let py_value = match item {
                        Parameter::Integer(i) => (*i).into_pyobject(py).ok()?.into_any().unbind(),
                        Parameter::Real(r) => (*r).into_pyobject(py).ok()?.into_any().unbind(),
                        Parameter::Boolean(b) => {
                            (*b).into_pyobject(py).ok()?.to_owned().into_any().unbind()
                        }
                        Parameter::String(s) => {
                            s.as_str().into_pyobject(py).ok()?.into_any().unbind()
                        }
                        Parameter::Array(_) => item.as_array(py)?,
                        Parameter::Object(_) => item.as_object(py)?,
                    };
                    dict.set_item(k, py_value).ok()?;
                }
                Some(dict.unbind().into_any())
            }
            _ => None,
        }
    }

    /// Convert to Python int
    fn __int__(&self) -> PyResult<i64> {
        match self {
            Parameter::Integer(v) => Ok(*v),
            Parameter::Real(v) => Ok(*v as i64),
            Parameter::Boolean(v) => Ok(if *v { 1 } else { 0 }),
            _ => Err(pyo3::exceptions::PyTypeError::new_err(
                "Cannot convert to int",
            )),
        }
    }

    /// Convert to Python float
    fn __float__(&self) -> PyResult<f64> {
        match self {
            Parameter::Real(v) => Ok(*v),
            Parameter::Integer(v) => Ok(*v as f64),
            Parameter::Boolean(v) => Ok(if *v { 1.0 } else { 0.0 }),
            _ => Err(pyo3::exceptions::PyTypeError::new_err(
                "Cannot convert to float",
            )),
        }
    }

    /// Convert to Python bool
    fn __bool__(&self) -> PyResult<bool> {
        match self {
            Parameter::Boolean(v) => Ok(*v),
            Parameter::Integer(v) => Ok(*v != 0),
            Parameter::Real(v) => Ok(*v != 0.0),
            Parameter::String(v) => Ok(!v.is_empty()),
            Parameter::Array(v) => Ok(!v.is_empty()),
            Parameter::Object(v) => Ok(!v.is_empty()),
        }
    }

    /// Convert to Python str
    fn __str__(&self) -> String {
        match self {
            Parameter::String(v) => v.clone(),
            _ => self.to_string(),
        }
    }

    /// Python repr
    fn __repr__(&self) -> String {
        self.to_string()
    }

    /// Get item by key with optional default (Object only)
    #[pyo3(signature = (key, default=None))]
    fn get(&self, py: Python<'_>, key: String, default: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
        match self {
            Parameter::Object(v) => {
                if let Some(value) = v.get(&key) {
                    value.to_pyobject(py)
                } else if let Some(default_value) = default {
                    Ok(default_value)
                } else {
                    Ok(py.None())
                }
            }
            _ => Err(pyo3::exceptions::PyTypeError::new_err(
                "get() is only available for Object parameters",
            )),
        }
    }

    /// Get dictionary keys (Object only)
    fn keys(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match self {
            Parameter::Object(v) => {
                let keys: Vec<String> = v.keys().cloned().collect();
                Ok(keys.into_pyobject(py)?.into_any().unbind())
            }
            _ => Err(pyo3::exceptions::PyTypeError::new_err(
                "keys() is only available for Object parameters",
            )),
        }
    }

    /// Get dictionary values (Object only)
    fn values(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match self {
            Parameter::Object(v) => {
                let list = pyo3::types::PyList::empty(py);
                for value in v.values() {
                    list.append(value.to_pyobject(py)?)?;
                }
                Ok(list.unbind().into_any())
            }
            _ => Err(pyo3::exceptions::PyTypeError::new_err(
                "values() is only available for Object parameters",
            )),
        }
    }

    /// Get dictionary items as (key, value) tuples (Object only)
    fn items(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match self {
            Parameter::Object(v) => {
                let list = pyo3::types::PyList::empty(py);
                for (k, value) in v.iter() {
                    let tuple = pyo3::types::PyTuple::new(
                        py,
                        &[
                            k.as_str().into_pyobject(py)?.into_any().unbind(),
                            value.to_pyobject(py)?,
                        ],
                    )?;
                    list.append(tuple)?;
                }
                Ok(list.unbind().into_any())
            }
            _ => Err(pyo3::exceptions::PyTypeError::new_err(
                "items() is only available for Object parameters",
            )),
        }
    }

    /// Equality comparison with type coercion
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        match self {
            Parameter::Real(v) => {
                // Try float comparison with tolerance
                if let Ok(other_f) = other.extract::<f64>() {
                    const EPSILON: f64 = 1e-9;
                    return Ok((v - other_f).abs() <= EPSILON);
                }
                // Try int comparison
                if let Ok(other_i) = other.extract::<i64>() {
                    const EPSILON: f64 = 1e-9;
                    return Ok((v - other_i as f64).abs() <= EPSILON);
                }
                Ok(false)
            }
            Parameter::Integer(v) => {
                // Try int comparison
                if let Ok(other_i) = other.extract::<i64>() {
                    return Ok(*v == other_i);
                }
                // Try float comparison with tolerance
                if let Ok(other_f) = other.extract::<f64>() {
                    const EPSILON: f64 = 1e-9;
                    return Ok((*v as f64 - other_f).abs() <= EPSILON);
                }
                Ok(false)
            }
            Parameter::Boolean(v) => {
                if let Ok(other_b) = other.extract::<bool>() {
                    Ok(*v == other_b)
                } else {
                    Ok(false)
                }
            }
            Parameter::String(v) => {
                if let Ok(other_s) = other.extract::<String>() {
                    Ok(v == &other_s)
                } else {
                    Ok(false)
                }
            }
            // Arrays and Objects can't be compared to Python primitives
            _ => Ok(false),
        }
    }
}

// Note: __getitem__, __len__, and __contains__ magic methods cannot be
// implemented for this enum due to PyO3 limitations. Enum tuple variants
// wrapping Vec<T> and HashMap<K,V> automatically get PyO3's default sequence/
// mapping protocol implementations that cannot be overridden.
//
// Instead, use the explicit methods:
// - For Object: .get(key, default=None), .keys(), .values(), .items()
// - For Array: .as_array() to convert to native Python list
//
// This is actually a common Python pattern - many APIs use .get() as the
// primary access method (e.g., os.environ.get('KEY')).

impl From<edgefirst_client::Parameter> for Parameter {
    fn from(param: edgefirst_client::Parameter) -> Self {
        match param {
            edgefirst_client::Parameter::Integer(v) => Parameter::Integer(v),
            edgefirst_client::Parameter::Real(v) => Parameter::Real(v),
            edgefirst_client::Parameter::Boolean(v) => Parameter::Boolean(v),
            edgefirst_client::Parameter::String(v) => Parameter::String(v),
            edgefirst_client::Parameter::Array(v) => {
                Parameter::Array(v.into_iter().map(Into::into).collect())
            }
            edgefirst_client::Parameter::Object(v) => {
                Parameter::Object(v.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
        }
    }
}

impl From<Parameter> for edgefirst_client::Parameter {
    fn from(val: Parameter) -> Self {
        match val {
            Parameter::Integer(v) => edgefirst_client::Parameter::Integer(v),
            Parameter::Real(v) => edgefirst_client::Parameter::Real(v),
            Parameter::Boolean(v) => edgefirst_client::Parameter::Boolean(v),
            Parameter::String(v) => edgefirst_client::Parameter::String(v),
            Parameter::Array(v) => {
                edgefirst_client::Parameter::Array(v.into_iter().map(Into::into).collect())
            }
            Parameter::Object(v) => edgefirst_client::Parameter::Object(
                v.into_iter().map(|(k, v)| (k, v.into())).collect(),
            ),
        }
    }
}

impl<'py> TryFrom<Bound<'py, PyAny>> for Parameter {
    type Error = Error;

    fn try_from(value: Bound<'py, PyAny>) -> Result<Self, Self::Error> {
        // First check if it's already a Parameter object
        if let Ok(param) = value.extract::<Parameter>() {
            return Ok(param);
        }

        // Check bool FIRST because in Python, bool is a subclass of int
        if let Ok(v) = value.extract::<bool>() {
            return Ok(Parameter::Boolean(v));
        }

        // Check int BEFORE float because int can be extracted as float
        if let Ok(v) = value.extract::<i64>() {
            return Ok(Parameter::Integer(v));
        }

        if let Ok(v) = value.extract::<f64>() {
            return Ok(Parameter::Real(v));
        }

        if let Ok(v) = value.extract::<String>() {
            return Ok(Parameter::String(v));
        }

        if let Ok(v) = value.extract::<Vec<Bound<'py, PyAny>>>() {
            let mut vec = Vec::with_capacity(v.len());
            for item in v {
                vec.push(item.try_into()?);
            }
            return Ok(Parameter::Array(vec));
        }

        if let Ok(v) = value.extract::<HashMap<String, Bound<'py, PyAny>>>() {
            let mut map = HashMap::with_capacity(v.len());
            for (k, item) in v {
                map.insert(k, item.try_into()?);
            }
            return Ok(Parameter::Object(map));
        }

        Err(Error::TypeError(
            "Parameter must be int, float, bool, str, list, or dict".into(),
        ))
    }
}

impl Display for Parameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Parameter::Integer(v) => write!(f, "Integer({})", v),
            Parameter::Real(v) => write!(f, "Real({})", v),
            Parameter::Boolean(v) => write!(f, "Boolean({})", v),
            Parameter::String(v) => write!(f, "String({})", v),
            Parameter::Array(v) => {
                let items: Vec<String> = v.iter().map(|item| format!("{}", item)).collect();
                write!(f, "[{}]", items.join(", "))
            }
            Parameter::Object(v) => {
                let items: Vec<String> = v
                    .iter()
                    .map(|(k, item)| format!("{}: {}", k, item))
                    .collect();
                write!(f, "{{{}}}", items.join(", "))
            }
        }
    }
}

// Individual ID wrapper types for Python
#[pyclass(module = "edgefirst_client")]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ProjectID(edgefirst_client::ProjectID);

impl Display for ProjectID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ProjectID {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let project_id = s
            .parse::<edgefirst_client::ProjectID>()
            .map_err(|e| Error::TypeError(format!("Invalid project ID: {:?}", e)))?;
        Ok(ProjectID(project_id))
    }
}

impl<'py> TryFrom<Bound<'py, PyAny>> for ProjectID {
    type Error = Error;

    fn try_from(value: Bound<'py, PyAny>) -> Result<Self, Self::Error> {
        // Try to extract as our ProjectID type first
        if let Ok(project_id) = value.extract::<ProjectID>() {
            return Ok(project_id);
        }

        // Try to extract as string
        if let Ok(s) = value.extract::<String>() {
            return s.parse();
        }

        // Try to extract as integer
        if let Ok(id_val) = value.extract::<u64>() {
            let project_id = edgefirst_client::ProjectID::from(id_val);
            return Ok(ProjectID(project_id));
        }

        Err(Error::TypeError(
            "ProjectID must be str, int, or ProjectID".into(),
        ))
    }
}

#[pymethods]
impl ProjectID {
    #[getter]
    pub fn value(&self) -> u64 {
        self.0.value()
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("ProjectID('{}')", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        self.0.value()
    }
}

#[pyclass(module = "edgefirst_client")]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct DatasetID(edgefirst_client::DatasetID);

impl Display for DatasetID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for DatasetID {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let dataset_id = s
            .parse::<edgefirst_client::DatasetID>()
            .map_err(|e| Error::TypeError(format!("Invalid dataset ID: {:?}", e)))?;
        Ok(DatasetID(dataset_id))
    }
}

impl<'py> TryFrom<Bound<'py, PyAny>> for DatasetID {
    type Error = Error;

    fn try_from(value: Bound<'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(dataset_id) = value.extract::<DatasetID>() {
            return Ok(dataset_id);
        }
        if let Ok(s) = value.extract::<String>() {
            return s.parse();
        }
        if let Ok(id_val) = value.extract::<u64>() {
            let dataset_id = edgefirst_client::DatasetID::from(id_val);
            return Ok(DatasetID(dataset_id));
        }
        Err(Error::TypeError(
            "DatasetID must be str, int, or DatasetID".into(),
        ))
    }
}

#[pymethods]
impl DatasetID {
    #[getter]
    pub fn value(&self) -> u64 {
        self.0.value()
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("DatasetID('{}')", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        self.0.value()
    }
}

#[pyclass(module = "edgefirst_client")]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ExperimentID(edgefirst_client::ExperimentID);

impl Display for ExperimentID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ExperimentID {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let experiment_id = s
            .parse::<edgefirst_client::ExperimentID>()
            .map_err(|e| Error::TypeError(format!("Invalid experiment ID: {:?}", e)))?;
        Ok(ExperimentID(experiment_id))
    }
}

impl<'py> TryFrom<Bound<'py, PyAny>> for ExperimentID {
    type Error = Error;

    fn try_from(value: Bound<'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(experiment_id) = value.extract::<ExperimentID>() {
            return Ok(experiment_id);
        }
        if let Ok(s) = value.extract::<String>() {
            return s.parse();
        }
        if let Ok(id_val) = value.extract::<u64>() {
            let experiment_id = edgefirst_client::ExperimentID::from(id_val);
            return Ok(ExperimentID(experiment_id));
        }
        Err(Error::TypeError(
            "ExperimentID must be str, int, or ExperimentID".into(),
        ))
    }
}

#[pymethods]
impl ExperimentID {
    #[getter]
    pub fn value(&self) -> u64 {
        self.0.value()
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("ExperimentID('{}')", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        self.0.value()
    }
}

#[pyclass(module = "edgefirst_client")]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct OrganizationID(edgefirst_client::OrganizationID);

impl Display for OrganizationID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for OrganizationID {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let org_id = edgefirst_client::OrganizationID::try_from(s)
            .map_err(|e| Error::TypeError(format!("Invalid organization ID: {:?}", e)))?;
        Ok(OrganizationID(org_id))
    }
}

impl<'py> TryFrom<Bound<'py, PyAny>> for OrganizationID {
    type Error = Error;

    fn try_from(value: Bound<'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(org_id) = value.extract::<OrganizationID>() {
            return Ok(org_id);
        }
        if let Ok(s) = value.extract::<String>() {
            return s.parse();
        }
        if let Ok(id_val) = value.extract::<u64>() {
            let org_id = edgefirst_client::OrganizationID::from(id_val);
            return Ok(OrganizationID(org_id));
        }
        Err(Error::TypeError(
            "OrganizationID must be str, int, or OrganizationID".into(),
        ))
    }
}

#[pymethods]
impl OrganizationID {
    #[getter]
    pub fn value(&self) -> u64 {
        self.0.value()
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("OrganizationID('{}')", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        self.0.value()
    }
}

#[pyclass(module = "edgefirst_client")]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct SampleID(edgefirst_client::SampleID);

impl Display for SampleID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SampleID {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let sample_id = edgefirst_client::SampleID::try_from(s)
            .map_err(|e| Error::TypeError(format!("Invalid sample ID: {:?}", e)))?;
        Ok(SampleID(sample_id))
    }
}

impl<'py> TryFrom<Bound<'py, PyAny>> for SampleID {
    type Error = Error;

    fn try_from(value: Bound<'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(sample_id) = value.extract::<SampleID>() {
            return Ok(sample_id);
        }
        if let Ok(s) = value.extract::<String>() {
            return s.parse();
        }
        if let Ok(id_val) = value.extract::<u64>() {
            let sample_id = edgefirst_client::SampleID::from(id_val);
            return Ok(SampleID(sample_id));
        }
        Err(Error::TypeError(
            "SampleID must be str, int, or SampleID".into(),
        ))
    }
}

#[pymethods]
impl SampleID {
    #[getter]
    pub fn value(&self) -> u64 {
        self.0.value()
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("SampleID('{}')", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        self.0.value()
    }
}

#[pyclass(module = "edgefirst_client")]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct AnnotationSetID(edgefirst_client::AnnotationSetID);

impl Display for AnnotationSetID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for AnnotationSetID {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let annotation_set_id = s
            .parse::<edgefirst_client::AnnotationSetID>()
            .map_err(|e| Error::TypeError(format!("Invalid annotation set ID: {:?}", e)))?;
        Ok(AnnotationSetID(annotation_set_id))
    }
}

impl<'py> TryFrom<Bound<'py, PyAny>> for AnnotationSetID {
    type Error = Error;

    fn try_from(value: Bound<'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(annotation_set_id) = value.extract::<AnnotationSetID>() {
            return Ok(annotation_set_id);
        }
        if let Ok(s) = value.extract::<String>() {
            return s.parse();
        }
        if let Ok(id_val) = value.extract::<u64>() {
            let annotation_set_id = edgefirst_client::AnnotationSetID::from(id_val);
            return Ok(AnnotationSetID(annotation_set_id));
        }
        Err(Error::TypeError(
            "AnnotationSetID must be str, int, or AnnotationSetID".into(),
        ))
    }
}

#[pymethods]
impl AnnotationSetID {
    #[getter]
    pub fn value(&self) -> u64 {
        self.0.value()
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("AnnotationSetID('{}')", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        self.0.value()
    }
}

#[pyclass(module = "edgefirst_client")]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct TaskID(edgefirst_client::TaskID);

impl Display for TaskID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for TaskID {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let task_id = s
            .parse::<edgefirst_client::TaskID>()
            .map_err(|e| Error::TypeError(format!("Invalid task ID: {:?}", e)))?;
        Ok(TaskID(task_id))
    }
}

impl<'py> TryFrom<Bound<'py, PyAny>> for TaskID {
    type Error = Error;

    fn try_from(value: Bound<'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(task_id) = value.extract::<TaskID>() {
            return Ok(task_id);
        }
        if let Ok(s) = value.extract::<String>() {
            return s.parse();
        }
        if let Ok(id_val) = value.extract::<u64>() {
            let task_id = edgefirst_client::TaskID::from(id_val);
            return Ok(TaskID(task_id));
        }
        Err(Error::TypeError(
            "TaskID must be str, int, or TaskID".into(),
        ))
    }
}

#[pymethods]
impl TaskID {
    #[getter]
    pub fn value(&self) -> u64 {
        self.0.value()
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("TaskID('{}')", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        self.0.value()
    }
}

#[pyclass(module = "edgefirst_client")]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct TrainingSessionID(edgefirst_client::TrainingSessionID);

impl Display for TrainingSessionID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for TrainingSessionID {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let training_session_id = s
            .parse::<edgefirst_client::TrainingSessionID>()
            .map_err(|e| Error::TypeError(format!("Invalid training session ID: {:?}", e)))?;
        Ok(TrainingSessionID(training_session_id))
    }
}

impl<'py> TryFrom<Bound<'py, PyAny>> for TrainingSessionID {
    type Error = Error;

    fn try_from(value: Bound<'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(training_session_id) = value.extract::<TrainingSessionID>() {
            return Ok(training_session_id);
        }
        if let Ok(s) = value.extract::<String>() {
            return s.parse();
        }
        if let Ok(id_val) = value.extract::<u64>() {
            let training_session_id = edgefirst_client::TrainingSessionID::from(id_val);
            return Ok(TrainingSessionID(training_session_id));
        }
        Err(Error::TypeError(
            "TrainingSessionID must be str, int, or TrainingSessionID".into(),
        ))
    }
}

#[pymethods]
impl TrainingSessionID {
    #[getter]
    pub fn value(&self) -> u64 {
        self.0.value()
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("TrainingSessionID('{}')", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        self.0.value()
    }
}

#[pyclass(module = "edgefirst_client")]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ValidationSessionID(edgefirst_client::ValidationSessionID);

impl Display for ValidationSessionID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ValidationSessionID {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let validation_session_id = edgefirst_client::ValidationSessionID::try_from(s)
            .map_err(|e| Error::TypeError(format!("Invalid validation session ID: {:?}", e)))?;
        Ok(ValidationSessionID(validation_session_id))
    }
}

impl<'py> TryFrom<Bound<'py, PyAny>> for ValidationSessionID {
    type Error = Error;

    fn try_from(value: Bound<'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(validation_session_id) = value.extract::<ValidationSessionID>() {
            return Ok(validation_session_id);
        }
        if let Ok(s) = value.extract::<String>() {
            return s.parse();
        }
        if let Ok(id_val) = value.extract::<u64>() {
            let validation_session_id = edgefirst_client::ValidationSessionID::from(id_val);
            return Ok(ValidationSessionID(validation_session_id));
        }
        Err(Error::TypeError(
            "ValidationSessionID must be str, int, or ValidationSessionID".into(),
        ))
    }
}

#[pymethods]
impl ValidationSessionID {
    #[getter]
    pub fn value(&self) -> u64 {
        self.0.value()
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("ValidationSessionID('{}')", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        self.0.value()
    }
}

#[pyclass(module = "edgefirst_client")]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct SnapshotID(edgefirst_client::SnapshotID);

impl Display for SnapshotID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SnapshotID {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let snapshot_id = edgefirst_client::SnapshotID::try_from(s)
            .map_err(|e| Error::TypeError(format!("Invalid snapshot ID: {:?}", e)))?;
        Ok(SnapshotID(snapshot_id))
    }
}

impl<'py> TryFrom<Bound<'py, PyAny>> for SnapshotID {
    type Error = Error;

    fn try_from(value: Bound<'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(snapshot_id) = value.extract::<SnapshotID>() {
            return Ok(snapshot_id);
        }
        if let Ok(s) = value.extract::<String>() {
            return s.parse();
        }
        if let Ok(id_val) = value.extract::<u64>() {
            let snapshot_id = edgefirst_client::SnapshotID::from(id_val);
            return Ok(SnapshotID(snapshot_id));
        }
        Err(Error::TypeError(
            "SnapshotID must be str, int, or SnapshotID".into(),
        ))
    }
}

#[pymethods]
impl SnapshotID {
    #[getter]
    pub fn value(&self) -> u64 {
        self.0.value()
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("SnapshotID('{}')", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        self.0.value()
    }
}

#[pyclass(module = "edgefirst_client")]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ImageId(edgefirst_client::ImageId);

impl Display for ImageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ImageId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let image_id = edgefirst_client::ImageId::try_from(s)
            .map_err(|e| Error::TypeError(format!("Invalid image ID: {:?}", e)))?;
        Ok(ImageId(image_id))
    }
}

impl<'py> TryFrom<Bound<'py, PyAny>> for ImageId {
    type Error = Error;

    fn try_from(value: Bound<'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(image_id) = value.extract::<ImageId>() {
            return Ok(image_id);
        }
        if let Ok(s) = value.extract::<String>() {
            return s.parse();
        }
        if let Ok(id_val) = value.extract::<u64>() {
            let image_id = edgefirst_client::ImageId::from(id_val);
            return Ok(ImageId(image_id));
        }
        Err(Error::TypeError(
            "ImageId must be str, int, or ImageId".into(),
        ))
    }
}

#[pymethods]
impl ImageId {
    #[getter]
    pub fn value(&self) -> u64 {
        self.0.value()
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("ImageId('{}')", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        self.0.value()
    }
}

#[pyclass(module = "edgefirst_client")]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct SequenceId(edgefirst_client::SequenceId);

impl Display for SequenceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SequenceId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let sequence_id = edgefirst_client::SequenceId::try_from(s)
            .map_err(|e| Error::TypeError(format!("Invalid sequence ID: {:?}", e)))?;
        Ok(SequenceId(sequence_id))
    }
}

impl<'py> TryFrom<Bound<'py, PyAny>> for SequenceId {
    type Error = Error;

    fn try_from(value: Bound<'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(sequence_id) = value.extract::<SequenceId>() {
            return Ok(sequence_id);
        }
        if let Ok(s) = value.extract::<String>() {
            return s.parse();
        }
        if let Ok(id_val) = value.extract::<u64>() {
            let sequence_id = edgefirst_client::SequenceId::from(id_val);
            return Ok(SequenceId(sequence_id));
        }
        Err(Error::TypeError(
            "SequenceId must be str, int, or SequenceId".into(),
        ))
    }
}

#[pymethods]
impl SequenceId {
    #[getter]
    pub fn value(&self) -> u64 {
        self.0.value()
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("SequenceId('{}')", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        self.0.value()
    }
}

#[pyclass(module = "edgefirst_client")]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct AppId(edgefirst_client::AppId);

impl Display for AppId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for AppId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let app_id = edgefirst_client::AppId::try_from(s)
            .map_err(|e| Error::TypeError(format!("Invalid app ID: {:?}", e)))?;
        Ok(AppId(app_id))
    }
}

impl<'py> TryFrom<Bound<'py, PyAny>> for AppId {
    type Error = Error;

    fn try_from(value: Bound<'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(app_id) = value.extract::<AppId>() {
            return Ok(app_id);
        }
        if let Ok(s) = value.extract::<String>() {
            return s.parse();
        }
        if let Ok(id_val) = value.extract::<u64>() {
            let app_id = edgefirst_client::AppId::from(id_val);
            return Ok(AppId(app_id));
        }
        Err(Error::TypeError("AppId must be str, int, or AppId".into()))
    }
}

#[pymethods]
impl AppId {
    #[getter]
    pub fn value(&self) -> u64 {
        self.0.value()
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("AppId('{}')", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        self.0.value()
    }
}

#[pyclass(module = "edgefirst_client")]
#[derive(Debug, Clone, Copy)]
pub enum FileType {
    Image,
    LidarPcd,
    LidarDepth,
    LidarReflect,
    RadarPcd,
    RadarCube,
}

#[pyclass(module = "edgefirst_client")]
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum AnnotationType {
    Box2d,
    Box3d,
    Mask,
}

#[pyclass(module = "edgefirst_client")]
pub struct Box2d(edgefirst_client::Box2d);

#[pymethods]
impl Box2d {
    #[new]
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Box2d(edgefirst_client::Box2d::new(x, y, width, height))
    }

    #[getter]
    pub fn width(&self) -> f32 {
        self.0.width()
    }

    #[getter]
    pub fn height(&self) -> f32 {
        self.0.height()
    }

    #[getter]
    pub fn left(&self) -> f32 {
        self.0.left()
    }

    #[getter]
    pub fn top(&self) -> f32 {
        self.0.top()
    }

    #[getter]
    pub fn cx(&self) -> f32 {
        self.0.cx()
    }

    #[getter]
    pub fn cy(&self) -> f32 {
        self.0.cy()
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct Box3d(edgefirst_client::Box3d);

#[pymethods]
impl Box3d {
    #[new]
    pub fn new(cx: f32, cy: f32, cz: f32, width: f32, height: f32, length: f32) -> Self {
        Box3d(edgefirst_client::Box3d::new(
            cx, cy, cz, width, height, length,
        ))
    }

    #[getter]
    pub fn width(&self) -> f32 {
        self.0.width()
    }

    #[getter]
    pub fn height(&self) -> f32 {
        self.0.height()
    }

    #[getter]
    pub fn length(&self) -> f32 {
        self.0.length()
    }

    #[getter]
    pub fn cx(&self) -> f32 {
        self.0.cx()
    }

    #[getter]
    pub fn cy(&self) -> f32 {
        self.0.cy()
    }

    #[getter]
    pub fn cz(&self) -> f32 {
        self.0.cz()
    }

    #[getter]
    pub fn left(&self) -> f32 {
        self.0.left()
    }

    #[getter]
    pub fn top(&self) -> f32 {
        self.0.top()
    }

    #[getter]
    pub fn front(&self) -> f32 {
        self.0.front()
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct Mask(edgefirst_client::Mask);

#[pymethods]
impl Mask {
    #[new]
    pub fn new(polygon: Vec<Vec<(f32, f32)>>) -> Self {
        Mask(edgefirst_client::Mask::new(polygon))
    }

    #[getter]
    pub fn polygon(&self) -> &Vec<Vec<(f32, f32)>> {
        &self.0.polygon
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct Organization(edgefirst_client::Organization);

#[pymethods]
impl Organization {
    #[getter]
    pub fn id(&self) -> OrganizationID {
        OrganizationID(self.0.id())
    }

    #[getter]
    pub fn name(&self) -> &str {
        self.0.name()
    }

    #[getter]
    pub fn credits(&self) -> i64 {
        self.0.credits()
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct Project {
    inner: edgefirst_client::Project,
    client: Option<Arc<edgefirst_client::Client>>,
}

impl Project {
    fn with_client(
        inner: edgefirst_client::Project,
        client: Arc<edgefirst_client::Client>,
    ) -> Self {
        Self {
            inner,
            client: Some(client),
        }
    }
}

impl Display for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[pymethods]
impl Project {
    #[getter]
    pub fn id(&self) -> ProjectID {
        ProjectID(self.inner.id())
    }

    #[getter]
    pub fn uid(&self, py: Python<'_>) -> PyResult<String> {
        warn_uid_deprecated(py, "Project")?;
        Ok(self.inner.id().to_string())
    }

    #[getter]
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    #[getter]
    pub fn description(&self) -> &str {
        self.inner.description()
    }

    /// Get datasets for this project.
    ///
    /// New API (v2.6.0+): `project.datasets()` - uses embedded client reference
    /// Deprecated API: `project.datasets(client)` - passing client explicitly
    #[pyo3(signature = (client_or_name=None, name=None))]
    #[tokio_wrap::sync]
    pub fn datasets(
        &self,
        py: Python<'_>,
        client_or_name: Option<&Bound<'_, PyAny>>,
        name: Option<&str>,
    ) -> Result<Vec<Dataset>, Error> {
        // Handle deprecated API: datasets(client, name)
        if let Some(arg) = client_or_name {
            if let Ok(client) = arg.extract::<PyRef<Client>>() {
                warn_method_deprecated(py, "Project", "datasets")?;
                let client_arc = Arc::new(client.0.clone());
                let datasets = client.0.datasets(self.inner.id(), name).await?;
                return Ok(datasets
                    .into_iter()
                    .map(|d| Dataset::with_client(d, Arc::clone(&client_arc)))
                    .collect());
            }
            // First arg is name string
            if let Ok(name_str) = arg.extract::<String>() {
                let client_ref = self.client.as_ref().ok_or_else(|| {
                    Error::TypeError(
                        "Project has no client reference. Use client.datasets(project.id) instead."
                            .to_string(),
                    )
                })?;
                let datasets = client_ref
                    .datasets(self.inner.id(), Some(&name_str))
                    .await?;
                return Ok(datasets
                    .into_iter()
                    .map(|d| Dataset::with_client(d, Arc::clone(client_ref)))
                    .collect());
            }
        }

        // New API: datasets() or datasets(name=...)
        let client_ref = self.client.as_ref().ok_or_else(|| {
            Error::TypeError(
                "Project has no client reference. Use client.datasets(project.id) instead."
                    .to_string(),
            )
        })?;
        let datasets = client_ref.datasets(self.inner.id(), name).await?;
        Ok(datasets
            .into_iter()
            .map(|d| Dataset::with_client(d, Arc::clone(client_ref)))
            .collect())
    }

    /// Get experiments for this project.
    ///
    /// New API (v2.6.0+): `project.experiments()` - uses embedded client
    /// reference Deprecated API: `project.experiments(client)` - passing
    /// client explicitly
    #[pyo3(signature = (client_or_name=None, name=None))]
    #[tokio_wrap::sync]
    pub fn experiments(
        &self,
        py: Python<'_>,
        client_or_name: Option<&Bound<'_, PyAny>>,
        name: Option<&str>,
    ) -> Result<Vec<Experiment>, Error> {
        if let Some(arg) = client_or_name {
            if let Ok(client) = arg.extract::<PyRef<Client>>() {
                warn_method_deprecated(py, "Project", "experiments")?;
                let client_arc = Arc::new(client.0.clone());
                let experiments = client.0.experiments(self.inner.id(), name).await?;
                return Ok(experiments
                    .into_iter()
                    .map(|e| Experiment::with_client(e, Arc::clone(&client_arc)))
                    .collect());
            }
            if let Ok(name_str) = arg.extract::<String>() {
                let client_ref = self.client.as_ref().ok_or_else(|| {
                    Error::TypeError(
                        "Project has no client reference. Use client.experiments(project.id) instead."
                            .to_string(),
                    )
                })?;
                let client_arc = Arc::new((**client_ref).clone());
                let experiments = client_ref
                    .experiments(self.inner.id(), Some(&name_str))
                    .await?;
                return Ok(experiments
                    .into_iter()
                    .map(|e| Experiment::with_client(e, Arc::clone(&client_arc)))
                    .collect());
            }
        }

        let client_ref = self.client.as_ref().ok_or_else(|| {
            Error::TypeError(
                "Project has no client reference. Use client.experiments(project.id) instead."
                    .to_string(),
            )
        })?;
        let client_arc = Arc::new((**client_ref).clone());
        let experiments = client_ref.experiments(self.inner.id(), name).await?;
        Ok(experiments
            .into_iter()
            .map(|e| Experiment::with_client(e, Arc::clone(&client_arc)))
            .collect())
    }

    /// Get validation sessions for this project.
    ///
    /// New API (v2.6.0+): `project.validation_sessions()` - uses embedded
    /// client reference Deprecated API:
    /// `project.validation_sessions(client)` - passing client explicitly
    #[pyo3(signature = (client=None))]
    #[tokio_wrap::sync]
    pub fn validation_sessions(
        &self,
        py: Python<'_>,
        client: Option<&Client>,
    ) -> Result<Vec<ValidationSession>, Error> {
        if let Some(c) = client {
            warn_method_deprecated(py, "Project", "validation_sessions")?;
            let client_arc = Arc::new(c.0.clone());
            let sessions = c.0.validation_sessions(self.inner.id()).await?;
            return Ok(sessions
                .into_iter()
                .map(|s| ValidationSession::with_client(s, Arc::clone(&client_arc)))
                .collect());
        }

        let client_ref = self.client.as_ref().ok_or_else(|| {
            Error::TypeError(
                "Project has no client reference. Use client.validation_sessions(project.id) instead."
                    .to_string(),
            )
        })?;
        let sessions = client_ref.validation_sessions(self.inner.id()).await?;
        Ok(sessions
            .into_iter()
            .map(|s| ValidationSession::with_client(s, Arc::clone(client_ref)))
            .collect())
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct Dataset {
    inner: edgefirst_client::Dataset,
    client: Option<Arc<edgefirst_client::Client>>,
}

impl Dataset {
    /// Create a Dataset with a client reference (for new ergonomic API)
    fn with_client(
        inner: edgefirst_client::Dataset,
        client: Arc<edgefirst_client::Client>,
    ) -> Self {
        Self {
            inner,
            client: Some(client),
        }
    }

    /// Create a Dataset without a client reference (legacy)
    #[allow(dead_code)]
    fn without_client(inner: edgefirst_client::Dataset) -> Self {
        Self {
            inner,
            client: None,
        }
    }
}

impl Display for Dataset {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[pymethods]
impl Dataset {
    #[getter]
    pub fn id(&self) -> DatasetID {
        DatasetID(self.inner.id())
    }

    #[getter]
    pub fn uid(&self, py: Python<'_>) -> PyResult<String> {
        warn_uid_deprecated(py, "Dataset")?;
        Ok(self.inner.id().to_string())
    }

    #[getter]
    pub fn project_id(&self) -> ProjectID {
        ProjectID(self.inner.project_id())
    }

    #[getter]
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    #[getter]
    pub fn description(&self) -> &str {
        self.inner.description()
    }

    #[getter]
    pub fn created(&self, py: Python<'_>) -> PyResult<Py<PyDateTime>> {
        Ok(self.inner.created().into_pyobject(py)?.into())
    }

    /// Get labels for this dataset.
    ///
    /// New API (v2.6.0+): `dataset.labels()` - uses embedded client reference
    /// Deprecated API: `dataset.labels(client)` - passing client explicitly
    ///
    /// If the Dataset was created without a client reference (legacy code),
    /// use `client.labels(dataset.id)` instead.
    #[pyo3(signature = (client=None))]
    #[tokio_wrap::sync]
    pub fn labels(&self, py: Python<'_>, client: Option<&Client>) -> Result<Vec<Label>, Error> {
        // If client is passed, emit deprecation warning and use it
        if let Some(c) = client {
            warn_method_deprecated(py, "Dataset", "labels")?;
            let labels = c.0.labels(self.inner.id()).await?;
            return Ok(labels.into_iter().map(Label).collect());
        }

        // Use stored client reference (new API)
        let client_ref = self.client.as_ref().ok_or_else(|| {
            Error::TypeError(
                "Dataset has no client reference. Use client.labels(dataset.id) instead."
                    .to_string(),
            )
        })?;
        let labels = client_ref.labels(self.inner.id()).await?;
        Ok(labels.into_iter().map(Label).collect())
    }

    /// Add a label to this dataset.
    ///
    /// New API (v2.6.0+): `dataset.add_label("name")` - uses embedded client
    /// reference Deprecated API: `dataset.add_label(client, "name")` -
    /// passing client explicitly
    #[pyo3(signature = (name_or_client, name=None))]
    #[tokio_wrap::sync]
    pub fn add_label(
        &self,
        py: Python<'_>,
        name_or_client: &Bound<'_, PyAny>,
        name: Option<String>,
    ) -> Result<(), Error> {
        // Try to extract as Client first (deprecated API)
        if let Ok(client) = name_or_client.extract::<PyRef<Client>>() {
            warn_method_deprecated(py, "Dataset", "add_label")?;
            let label_name = name.ok_or_else(|| {
                Error::TypeError("add_label(client, name) requires name parameter".to_string())
            })?;
            client.0.add_label(self.inner.id(), &label_name).await?;
            return Ok(());
        }

        // Try to extract as string (new API)
        if let Ok(label_name) = name_or_client.extract::<String>() {
            let client_ref = self.client.as_ref().ok_or_else(|| {
                Error::TypeError(
                    "Dataset has no client reference. Use client.add_label(dataset.id, name) instead."
                        .to_string(),
                )
            })?;
            client_ref.add_label(self.inner.id(), &label_name).await?;
            return Ok(());
        }

        Err(Error::TypeError(
            "add_label() first argument must be a string (label name) or Client (deprecated)"
                .to_string(),
        ))
    }

    /// Remove a label from this dataset by name.
    ///
    /// New API (v2.6.0+): `dataset.remove_label("name")` - uses embedded client
    /// reference Deprecated API: `dataset.remove_label(client, "name")` -
    /// passing client explicitly
    #[pyo3(signature = (name_or_client, name=None))]
    #[tokio_wrap::sync]
    pub fn remove_label(
        &self,
        py: Python<'_>,
        name_or_client: &Bound<'_, PyAny>,
        name: Option<String>,
    ) -> Result<(), Error> {
        // Try to extract as Client first (deprecated API)
        if let Ok(client) = name_or_client.extract::<PyRef<Client>>() {
            warn_method_deprecated(py, "Dataset", "remove_label")?;
            let label_name = name.ok_or_else(|| {
                Error::TypeError("remove_label(client, name) requires name parameter".to_string())
            })?;
            let labels = client.0.labels(self.inner.id()).await?;
            let label = labels
                .iter()
                .find(|l| l.name() == label_name)
                .ok_or_else(|| {
                    Error::Error(edgefirst_client::Error::MissingLabel(label_name.clone()))
                })?;
            client.0.remove_label(label.id()).await?;
            return Ok(());
        }

        // Try to extract as string (new API)
        if let Ok(label_name) = name_or_client.extract::<String>() {
            let client_ref = self.client.as_ref().ok_or_else(|| {
                Error::TypeError(
                    "Dataset has no client reference. Use client.remove_label(label.id) instead."
                        .to_string(),
                )
            })?;
            let labels = client_ref.labels(self.inner.id()).await?;
            let label = labels
                .iter()
                .find(|l| l.name() == label_name)
                .ok_or_else(|| {
                    Error::Error(edgefirst_client::Error::MissingLabel(label_name.clone()))
                })?;
            client_ref.remove_label(label.id()).await?;
            return Ok(());
        }

        Err(Error::TypeError(
            "remove_label() first argument must be a string (label name) or Client (deprecated)"
                .to_string(),
        ))
    }

    /// Download this dataset to a local directory.
    ///
    /// New API (v2.6.0+): `dataset.download(output, ...)` - uses embedded
    /// client reference
    ///
    /// Note: For downloading multiple samples, this is the recommended approach
    /// as it uses batch downloading for far higher performance compared to
    /// downloading samples individually.
    ///
    /// Args:
    ///     output: Local directory path to save downloaded files
    ///     groups: Filter by sample groups (e.g., ["train", "val"])
    ///     types: File types to download (default: [FileType.Image])
    ///     flatten: If True, download all files to a flat directory structure
    ///     progress: Optional callback function(current, total) for progress
    ///
    /// If the Dataset was created without a client reference (legacy code),
    /// use `client.download_dataset(dataset.id, ...)` instead.
    #[pyo3(signature = (output, groups = vec![], types = vec![FileType::Image], flatten = false, progress = None))]
    pub fn download(
        &self,
        output: PathBuf,
        groups: Vec<String>,
        types: Vec<FileType>,
        flatten: bool,
        progress: Option<Py<PyAny>>,
    ) -> Result<(), Error> {
        let client_ref = self.client.as_ref().ok_or_else(|| {
            Error::TypeError(
                "Dataset has no client reference. Use client.download_dataset(dataset.id, ...) instead."
                    .to_string(),
            )
        })?;

        let types_converted: Vec<edgefirst_client::FileType> = types
            .into_iter()
            .map(|x| match x {
                FileType::Image => edgefirst_client::FileType::Image,
                FileType::LidarPcd => edgefirst_client::FileType::LidarPcd,
                FileType::LidarDepth => edgefirst_client::FileType::LidarDepth,
                FileType::LidarReflect => edgefirst_client::FileType::LidarReflect,
                FileType::RadarPcd => edgefirst_client::FileType::RadarPcd,
                FileType::RadarCube => edgefirst_client::FileType::RadarCube,
            })
            .collect();

        match progress {
            Some(progress) => {
                let (tx, mut rx) = mpsc::channel(1);
                let client = client_ref.clone();
                let dataset_id = self.inner.id();
                let groups_clone = groups.clone();
                let types_clone = types_converted.clone();
                let output_clone = output.clone();

                let task = std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        client
                            .download_dataset(
                                dataset_id,
                                &groups_clone,
                                &types_clone,
                                output_clone,
                                flatten,
                                Some(tx),
                            )
                            .await
                    })
                });

                while let Some(status) = rx.blocking_recv() {
                    Python::attach(|py| {
                        progress
                            .call1(py, (status.current, status.total))
                            .expect("Progress callback should be callable");
                    });
                }

                Ok(task.join().unwrap()?)
            }
            None => {
                let client = client_ref.clone();
                let dataset_id = self.inner.id();
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    client
                        .download_dataset(
                            dataset_id,
                            &groups,
                            &types_converted,
                            output,
                            flatten,
                            None,
                        )
                        .await
                })?;
                Ok(())
            }
        }
    }

    /// Get samples for this dataset.
    ///
    /// New API (v2.6.0+): `dataset.samples(...)` - uses embedded client
    /// reference
    ///
    /// Args:
    ///     annotation_set_id: Optional annotation set to include annotations
    /// from     annotation_types: Filter by annotation types
    ///     groups: Filter by sample groups (e.g., ["train", "val"])
    ///     types: File types to include (default: [FileType.Image])
    ///     progress: Optional callback function(current, total) for progress
    ///
    /// Returns:
    ///     List of Sample objects
    ///
    /// If the Dataset was created without a client reference (legacy code),
    /// use `client.samples(dataset.id, ...)` instead.
    #[pyo3(signature = (annotation_set_id = None, annotation_types = vec![], groups = vec![], types = vec![FileType::Image], progress = None))]
    pub fn samples<'py>(
        &self,
        annotation_set_id: Option<Bound<'py, PyAny>>,
        annotation_types: Vec<AnnotationType>,
        groups: Vec<String>,
        types: Vec<FileType>,
        progress: Option<Py<PyAny>>,
    ) -> Result<Vec<Sample>, Error> {
        let client_ref = self.client.as_ref().ok_or_else(|| {
            Error::TypeError(
                "Dataset has no client reference. Use client.samples(dataset.id, ...) instead."
                    .to_string(),
            )
        })?;

        let annotation_set_id: Option<AnnotationSetID> = match annotation_set_id {
            Some(id) => Some(id.try_into()?),
            None => None,
        };

        let annotation_types_converted: Vec<edgefirst_client::AnnotationType> = annotation_types
            .into_iter()
            .map(|x| match x {
                AnnotationType::Box2d => edgefirst_client::AnnotationType::Box2d,
                AnnotationType::Box3d => edgefirst_client::AnnotationType::Box3d,
                AnnotationType::Mask => edgefirst_client::AnnotationType::Mask,
            })
            .collect();

        let types_converted: Vec<edgefirst_client::FileType> = types
            .into_iter()
            .map(|x| match x {
                FileType::Image => edgefirst_client::FileType::Image,
                FileType::LidarPcd => edgefirst_client::FileType::LidarPcd,
                FileType::LidarDepth => edgefirst_client::FileType::LidarDepth,
                FileType::LidarReflect => edgefirst_client::FileType::LidarReflect,
                FileType::RadarPcd => edgefirst_client::FileType::RadarPcd,
                FileType::RadarCube => edgefirst_client::FileType::RadarCube,
            })
            .collect();

        let client_arc = Arc::clone(client_ref);
        let samples = match progress {
            Some(progress) => {
                let (tx, mut rx) = mpsc::channel(1);
                let client = client_ref.clone();
                let dataset_id = self.inner.id();
                let groups_clone = groups.clone();
                let annotation_types_clone = annotation_types_converted.clone();
                let types_clone = types_converted.clone();

                let task = std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        client
                            .samples(
                                dataset_id,
                                annotation_set_id.map(|x| x.0),
                                &annotation_types_clone,
                                &groups_clone,
                                &types_clone,
                                Some(tx),
                            )
                            .await
                    })
                });

                while let Some(status) = rx.blocking_recv() {
                    Python::attach(|py| {
                        progress
                            .call1(py, (status.current, status.total))
                            .expect("Progress callback should be callable");
                    });
                }

                task.join().unwrap()?
            }
            None => {
                let client = client_ref.clone();
                let dataset_id = self.inner.id();
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    client
                        .samples(
                            dataset_id,
                            annotation_set_id.map(|x| x.0),
                            &annotation_types_converted,
                            &groups,
                            &types_converted,
                            None,
                        )
                        .await
                })?
            }
        };

        Ok(samples
            .into_iter()
            .map(|s| Sample::with_client(s, Arc::clone(&client_arc)))
            .collect())
    }

    /// Get annotation sets for this dataset.
    ///
    /// New API (v2.6.0+): `dataset.annotation_sets()` - uses embedded client
    /// reference
    ///
    /// Returns:
    ///     List of AnnotationSet objects associated with this dataset
    ///
    /// If the Dataset was created without a client reference (legacy code),
    /// use `client.annotation_sets(dataset.id)` instead.
    #[tokio_wrap::sync]
    pub fn annotation_sets(&self) -> Result<Vec<AnnotationSet>, Error> {
        let client_ref = self.client.as_ref().ok_or_else(|| {
            Error::TypeError(
                "Dataset has no client reference. Use client.annotation_sets(dataset.id) instead."
                    .to_string(),
            )
        })?;

        let client_arc = Arc::clone(client_ref);
        let annotation_sets = client_ref.annotation_sets(self.inner.id()).await?;
        Ok(annotation_sets
            .into_iter()
            .map(|a| AnnotationSet::with_client(a, Arc::clone(&client_arc)))
            .collect())
    }

    /// Get the count of samples in this dataset.
    ///
    /// New API (v2.6.0+): `dataset.samples_count()` - uses embedded client
    /// reference
    ///
    /// Args:
    ///     annotation_set_id: Optional annotation set to filter by
    ///     annotation_types: Filter by annotation types
    ///     groups: Filter by sample groups (e.g., ["train", "val"])
    ///     types: File types to count (default: [FileType.Image])
    ///
    /// Returns:
    ///     SamplesCountResult with train/val/test counts
    ///
    /// If the Dataset was created without a client reference (legacy code),
    /// use `client.samples_count(dataset.id, ...)` instead.
    #[pyo3(signature = (annotation_set_id = None, annotation_types = vec![], groups = vec![], types = vec![FileType::Image]))]
    #[tokio_wrap::sync]
    pub fn samples_count<'py>(
        &self,
        annotation_set_id: Option<Bound<'py, PyAny>>,
        annotation_types: Vec<AnnotationType>,
        groups: Vec<String>,
        types: Vec<FileType>,
    ) -> Result<SamplesCountResult, Error> {
        let client_ref = self.client.as_ref().ok_or_else(|| {
            Error::TypeError(
                "Dataset has no client reference. Use client.samples_count(dataset.id, ...) instead."
                    .to_string(),
            )
        })?;

        let annotation_set_id: Option<AnnotationSetID> = match annotation_set_id {
            Some(id) => Some(id.try_into()?),
            None => None,
        };

        let annotation_types_converted: Vec<edgefirst_client::AnnotationType> = annotation_types
            .into_iter()
            .map(|x| match x {
                AnnotationType::Box2d => edgefirst_client::AnnotationType::Box2d,
                AnnotationType::Box3d => edgefirst_client::AnnotationType::Box3d,
                AnnotationType::Mask => edgefirst_client::AnnotationType::Mask,
            })
            .collect();

        let types_converted: Vec<edgefirst_client::FileType> = types
            .into_iter()
            .map(|x| match x {
                FileType::Image => edgefirst_client::FileType::Image,
                FileType::LidarPcd => edgefirst_client::FileType::LidarPcd,
                FileType::LidarDepth => edgefirst_client::FileType::LidarDepth,
                FileType::LidarReflect => edgefirst_client::FileType::LidarReflect,
                FileType::RadarPcd => edgefirst_client::FileType::RadarPcd,
                FileType::RadarCube => edgefirst_client::FileType::RadarCube,
            })
            .collect();

        Ok(SamplesCountResult(
            client_ref
                .samples_count(
                    self.inner.id(),
                    annotation_set_id.map(|x| x.0),
                    &annotation_types_converted,
                    &groups,
                    &types_converted,
                )
                .await?,
        ))
    }
}

#[pyclass(module = "edgefirst_client")]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Label(edgefirst_client::Label);

#[pymethods]
impl Label {
    #[getter]
    pub fn id(&self) -> u64 {
        self.0.id()
    }

    #[getter]
    pub fn name(&self) -> &str {
        self.0.name()
    }

    #[getter]
    pub fn index(&self) -> u64 {
        self.0.index()
    }

    #[getter]
    pub fn dataset_id(&self) -> DatasetID {
        DatasetID(self.0.dataset_id())
    }

    #[allow(deprecated)]
    #[tokio_wrap::sync]
    pub fn remove(&self, client: &Client) -> Result<(), Error> {
        Ok(self.0.remove(&client.0).await?)
    }

    #[allow(deprecated)]
    #[tokio_wrap::sync]
    pub fn set_name(&mut self, client: &Client, name: &str) -> Result<(), Error> {
        Ok(self.0.set_name(&client.0, name).await?)
    }

    #[allow(deprecated)]
    #[tokio_wrap::sync]
    pub fn set_index(&mut self, client: &Client, index: u64) -> Result<(), Error> {
        Ok(self.0.set_index(&client.0, index).await?)
    }

    pub fn __repr__(&self) -> String {
        format!(
            "Label(id={}, index={}, name='{}')",
            self.id(),
            self.index(),
            self.name()
        )
    }

    pub fn __str__(&self) -> String {
        format!("{}", self)
    }
}

impl Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct AnnotationSet {
    inner: edgefirst_client::AnnotationSet,
    client: Option<Arc<edgefirst_client::Client>>,
}

impl AnnotationSet {
    fn with_client(
        inner: edgefirst_client::AnnotationSet,
        client: Arc<edgefirst_client::Client>,
    ) -> Self {
        Self {
            inner,
            client: Some(client),
        }
    }
}

impl Display for AnnotationSet {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[pymethods]
impl AnnotationSet {
    #[getter]
    pub fn id(&self) -> AnnotationSetID {
        AnnotationSetID(self.inner.id())
    }

    #[getter]
    pub fn uid(&self, py: Python<'_>) -> PyResult<String> {
        warn_uid_deprecated(py, "AnnotationSet")?;
        Ok(self.inner.id().to_string())
    }

    #[getter]
    pub fn dataset_id(&self) -> DatasetID {
        DatasetID(self.inner.dataset_id())
    }

    #[getter]
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    #[getter]
    pub fn description(&self) -> &str {
        self.inner.description()
    }

    #[getter]
    pub fn created(&self, py: Python<'_>) -> PyResult<Py<PyDateTime>> {
        Ok(self.inner.created().into_pyobject(py)?.into())
    }

    /// Get annotations for this annotation set.
    ///
    /// Args:
    ///     groups: List of dataset groups (train, val, test)
    ///     annotation_types: List of annotation types to filter
    ///     progress: Optional progress callback
    ///
    /// Returns:
    ///     List[Annotation]: Annotations in this set
    #[pyo3(signature = (groups = vec![], annotation_types = vec![], progress = None))]
    #[tokio_wrap::sync]
    pub fn annotations(
        &self,
        groups: Vec<String>,
        annotation_types: Vec<AnnotationType>,
        progress: Option<Py<PyAny>>,
    ) -> Result<Vec<Annotation>, Error> {
        let client_ref = self.client.as_ref().ok_or_else(|| {
            Error::TypeError(
                "AnnotationSet has no client reference. Use client.annotations(annotation_set.id, ...) instead."
                    .to_string(),
            )
        })?;

        let annotation_types_converted: Vec<edgefirst_client::AnnotationType> = annotation_types
            .into_iter()
            .map(|x| match x {
                AnnotationType::Box2d => edgefirst_client::AnnotationType::Box2d,
                AnnotationType::Box3d => edgefirst_client::AnnotationType::Box3d,
                AnnotationType::Mask => edgefirst_client::AnnotationType::Mask,
            })
            .collect();

        match progress {
            Some(progress) => {
                let (tx, mut rx) = mpsc::channel(1);
                let client = client_ref.clone();
                let annotation_set_id = self.inner.id();
                let groups_clone = groups.clone();
                let annotation_types_clone = annotation_types_converted.clone();

                let task = std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        client
                            .annotations(
                                annotation_set_id,
                                &groups_clone,
                                &annotation_types_clone,
                                Some(tx),
                            )
                            .await
                    })
                });

                while let Some(status) = rx.blocking_recv() {
                    Python::attach(|py| {
                        progress
                            .call1(py, (status.current, status.total))
                            .expect("Progress callback should be callable");
                    });
                }

                Ok(task.join().unwrap()?.into_iter().map(Annotation).collect())
            }
            None => {
                let annotations = client_ref
                    .annotations(self.inner.id(), &groups, &annotation_types_converted, None)
                    .await?;
                Ok(annotations.into_iter().map(Annotation).collect())
            }
        }
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct Experiment {
    inner: edgefirst_client::Experiment,
    client: Option<Arc<edgefirst_client::Client>>,
}

impl Experiment {
    fn with_client(
        inner: edgefirst_client::Experiment,
        client: Arc<edgefirst_client::Client>,
    ) -> Self {
        Self {
            inner,
            client: Some(client),
        }
    }
}

impl Display for Experiment {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[pymethods]
impl Experiment {
    #[getter]
    pub fn id(&self) -> ExperimentID {
        ExperimentID(self.inner.id())
    }

    #[getter]
    pub fn uid(&self, py: Python<'_>) -> PyResult<String> {
        warn_uid_deprecated(py, "Experiment")?;
        Ok(self.inner.id().to_string())
    }

    #[getter]
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    #[getter]
    pub fn description(&self) -> &str {
        self.inner.description()
    }

    /// Get training sessions for this experiment.
    #[pyo3(signature = (name = None))]
    #[tokio_wrap::sync]
    pub fn training_sessions(&self, name: Option<&str>) -> Result<Vec<TrainingSession>, Error> {
        let client_ref = self.client.as_ref().ok_or_else(|| {
            Error::TypeError(
                "Experiment has no client reference. Use client.training_sessions(experiment.id) instead."
                    .to_string(),
            )
        })?;
        let client_arc = Arc::new((**client_ref).clone());
        let sessions = client_ref.training_sessions(self.inner.id(), name).await?;
        Ok(sessions
            .into_iter()
            .map(|s| TrainingSession::with_client(s, Arc::clone(&client_arc)))
            .collect())
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct TrainingSession {
    inner: edgefirst_client::TrainingSession,
    client: Option<Arc<edgefirst_client::Client>>,
}

impl TrainingSession {
    /// Create a TrainingSession with a client reference (for new ergonomic API)
    fn with_client(
        inner: edgefirst_client::TrainingSession,
        client: Arc<edgefirst_client::Client>,
    ) -> Self {
        Self {
            inner,
            client: Some(client),
        }
    }

    /// Create a TrainingSession without a client reference (legacy)
    #[allow(dead_code)]
    fn without_client(inner: edgefirst_client::TrainingSession) -> Self {
        Self {
            inner,
            client: None,
        }
    }
}

impl Display for TrainingSession {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[pymethods]
impl TrainingSession {
    #[getter]
    pub fn id(&self) -> TrainingSessionID {
        TrainingSessionID(self.inner.id())
    }

    #[getter]
    pub fn uid(&self, py: Python<'_>) -> PyResult<String> {
        warn_uid_deprecated(py, "TrainingSession")?;
        Ok(self.inner.id().to_string())
    }

    #[getter]
    pub fn experiment_id(&self) -> ExperimentID {
        ExperimentID(self.inner.experiment_id())
    }

    #[getter]
    pub fn model(&self) -> &str {
        self.inner.model()
    }

    #[getter]
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    #[getter]
    pub fn description(&self) -> &str {
        self.inner.description()
    }

    #[getter]
    pub fn model_params<'py>(&self, py: Python<'py>) -> Result<Py<PyDict>, Error> {
        let params = PyDict::new(py);

        for (key, value) in self.inner.model_params() {
            let value = Parameter::from(value.clone());
            params.set_item(key, value.into_pyobject(py)?)?;
        }

        Ok(params.into())
    }

    #[getter]
    pub fn dataset_params(&self) -> DatasetParams {
        DatasetParams(self.inner.dataset_params().clone())
    }

    #[getter]
    pub fn task(&self) -> Task {
        Task(self.inner.task())
    }

    /// Get metrics for this training session.
    ///
    /// New API (v2.6.0+): `session.metrics()` - uses embedded client reference
    /// Deprecated API: `session.metrics(client)` - passing client explicitly
    #[pyo3(signature = (client=None))]
    #[tokio_wrap::sync]
    pub fn metrics<'py>(
        &self,
        py: Python<'py>,
        client: Option<&Client>,
    ) -> Result<Py<PyDict>, Error> {
        let client_ref = if let Some(c) = client {
            warn_method_deprecated(py, "TrainingSession", "metrics")?;
            &c.0
        } else {
            self.client.as_ref().ok_or_else(|| {
                Error::TypeError(
                    "TrainingSession has no client reference. Use session.metrics(client) instead."
                        .to_string(),
                )
            })?
        };

        let metrics = PyDict::new(py);
        for (key, value) in self.inner.metrics(client_ref).await? {
            let value = Parameter::from(value.clone());
            metrics.set_item(key, value.into_pyobject(py)?)?;
        }

        Ok(metrics.into())
    }

    /// Set metrics for this training session.
    ///
    /// New API (v2.6.0+): `session.set_metrics(metrics)` - uses embedded client
    /// reference Deprecated API: `session.set_metrics(client, metrics)` -
    /// passing client explicitly
    #[pyo3(signature = (metrics_or_client, metrics=None))]
    #[tokio_wrap::sync]
    pub fn set_metrics<'py>(
        &self,
        py: Python<'py>,
        metrics_or_client: &Bound<'py, PyAny>,
        metrics: Option<HashMap<String, Bound<'py, PyAny>>>,
    ) -> Result<(), Error> {
        // Try to extract as Client first (deprecated API)
        if let Ok(client) = metrics_or_client.extract::<PyRef<Client>>() {
            warn_method_deprecated(py, "TrainingSession", "set_metrics")?;
            let metrics = metrics.ok_or_else(|| {
                Error::TypeError(
                    "set_metrics() requires 'metrics' argument when using deprecated API"
                        .to_string(),
                )
            })?;
            let mut map = HashMap::<String, edgefirst_client::Parameter>::new();
            for (key, value) in metrics {
                let value: Parameter = value.try_into()?;
                map.insert(key, value.into());
            }
            return Ok(self.inner.set_metrics(&client.0, map).await?);
        }

        // Try to extract as dict (new API)
        if let Ok(metrics_dict) = metrics_or_client.extract::<HashMap<String, Bound<'py, PyAny>>>()
        {
            let client_ref = self.client.as_ref().ok_or_else(|| {
                Error::TypeError(
                    "TrainingSession has no client reference. Use session.set_metrics(client, metrics) instead."
                        .to_string(),
                )
            })?;
            let mut map = HashMap::<String, edgefirst_client::Parameter>::new();
            for (key, value) in metrics_dict {
                let value: Parameter = value.try_into()?;
                map.insert(key, value.into());
            }
            return Ok(self.inner.set_metrics(client_ref.as_ref(), map).await?);
        }

        Err(Error::TypeError(
            "set_metrics() first argument must be a dict (metrics) or Client (deprecated)"
                .to_string(),
        ))
    }

    /// Get artifacts for this training session.
    ///
    /// New API (v2.6.0+): `session.artifacts()` - uses embedded client
    /// reference Deprecated API: `session.artifacts(client)` - passing
    /// client explicitly
    #[pyo3(signature = (client=None))]
    #[tokio_wrap::sync]
    pub fn artifacts(
        &self,
        py: Python<'_>,
        client: Option<&Client>,
    ) -> Result<Vec<Artifact>, Error> {
        let client_ref = if let Some(c) = client {
            warn_method_deprecated(py, "TrainingSession", "artifacts")?;
            &c.0
        } else {
            self.client.as_ref().ok_or_else(|| {
                Error::TypeError(
                    "TrainingSession has no client reference. Use session.artifacts(client) instead."
                        .to_string(),
                )
            })?
        };

        let artifacts = client_ref
            .artifacts(self.inner.id())
            .await?
            .into_iter()
            .map(Artifact)
            .collect();
        Ok(artifacts)
    }

    /// Download an artifact file from the training session.
    ///
    /// New API (v2.6.0+): `session.download_artifact(filename)` - uses embedded
    /// client reference Deprecated API: `session.download_artifact(client,
    /// filename)` - passing client explicitly
    #[pyo3(signature = (filename_or_client, filename=None))]
    #[tokio_wrap::sync]
    pub fn download_artifact(
        &self,
        py: Python<'_>,
        filename_or_client: &Bound<'_, PyAny>,
        filename: Option<String>,
    ) -> Result<Vec<u8>, Error> {
        // Try to extract as Client first (deprecated API)
        if let Ok(client) = filename_or_client.extract::<PyRef<Client>>() {
            warn_method_deprecated(py, "TrainingSession", "download_artifact")?;
            let filename = filename.ok_or_else(|| {
                Error::TypeError(
                    "download_artifact() requires 'filename' argument when using deprecated API"
                        .to_string(),
                )
            })?;
            return Ok(self.inner.download_artifact(&client.0, &filename).await?);
        }

        // Try to extract as filename string (new API)
        if let Ok(fname) = filename_or_client.extract::<String>() {
            let client_ref = self.client.as_ref().ok_or_else(|| {
                Error::TypeError(
                    "TrainingSession has no client reference. Use session.download_artifact(client, filename) instead."
                        .to_string(),
                )
            })?;
            return Ok(self
                .inner
                .download_artifact(client_ref.as_ref(), &fname)
                .await?);
        }

        Err(Error::TypeError(
            "download_artifact() first argument must be a string (filename) or Client (deprecated)"
                .to_string(),
        ))
    }

    /// Upload an artifact file to the training session.
    ///
    /// New API (v2.6.0+): `session.upload_artifact(filename, path)` - uses
    /// embedded client reference Deprecated API:
    /// `session.upload_artifact(client, filename, path)` - passing client
    /// explicitly
    #[pyo3(signature = (filename_or_client, filename_or_path=None, path=None))]
    #[tokio_wrap::sync]
    pub fn upload_artifact(
        &self,
        py: Python<'_>,
        filename_or_client: &Bound<'_, PyAny>,
        filename_or_path: Option<&Bound<'_, PyAny>>,
        path: Option<PathBuf>,
    ) -> Result<(), Error> {
        // Try to extract as Client first (deprecated API)
        if let Ok(client) = filename_or_client.extract::<PyRef<Client>>() {
            warn_method_deprecated(py, "TrainingSession", "upload_artifact")?;
            let filename = filename_or_path
                .ok_or_else(|| {
                    Error::TypeError(
                        "upload_artifact() requires 'filename' argument when using deprecated API"
                            .to_string(),
                    )
                })?
                .extract::<String>()?;
            let path = path.unwrap_or_else(|| PathBuf::from(&filename));
            return Ok(self
                .inner
                .upload_artifact(&client.0, &filename, path)
                .await?);
        }

        // Try to extract as filename string (new API)
        if let Ok(fname) = filename_or_client.extract::<String>() {
            let client_ref = self.client.as_ref().ok_or_else(|| {
                Error::TypeError(
                    "TrainingSession has no client reference. Use session.upload_artifact(client, filename, path) instead."
                        .to_string(),
                )
            })?;
            // In new API: filename_or_path is the path, and path is unused
            let file_path = filename_or_path
                .map(|p| p.extract::<PathBuf>())
                .transpose()?
                .unwrap_or_else(|| PathBuf::from(&fname));
            return Ok(self
                .inner
                .upload_artifact(client_ref.as_ref(), &fname, file_path)
                .await?);
        }

        Err(Error::TypeError(
            "upload_artifact() first argument must be a string (filename) or Client (deprecated)"
                .to_string(),
        ))
    }

    /// Download a checkpoint file from the training session.
    ///
    /// New API (v2.6.0+): `session.download_checkpoint(filename)` - uses
    /// embedded client reference Deprecated API:
    /// `session.download_checkpoint(client, filename)` - passing client
    /// explicitly
    #[pyo3(signature = (filename_or_client, filename=None))]
    #[tokio_wrap::sync]
    pub fn download_checkpoint(
        &self,
        py: Python<'_>,
        filename_or_client: &Bound<'_, PyAny>,
        filename: Option<String>,
    ) -> Result<Vec<u8>, Error> {
        // Try to extract as Client first (deprecated API)
        if let Ok(client) = filename_or_client.extract::<PyRef<Client>>() {
            warn_method_deprecated(py, "TrainingSession", "download_checkpoint")?;
            let filename = filename.ok_or_else(|| {
                Error::TypeError(
                    "download_checkpoint() requires 'filename' argument when using deprecated API"
                        .to_string(),
                )
            })?;
            return Ok(self.inner.download_checkpoint(&client.0, &filename).await?);
        }

        // Try to extract as filename string (new API)
        if let Ok(fname) = filename_or_client.extract::<String>() {
            let client_ref = self.client.as_ref().ok_or_else(|| {
                Error::TypeError(
                    "TrainingSession has no client reference. Use session.download_checkpoint(client, filename) instead."
                        .to_string(),
                )
            })?;
            return Ok(self
                .inner
                .download_checkpoint(client_ref.as_ref(), &fname)
                .await?);
        }

        Err(Error::TypeError(
            "download_checkpoint() first argument must be a string (filename) or Client (deprecated)"
                .to_string(),
        ))
    }

    /// Upload a checkpoint file to the training session.
    ///
    /// New API (v2.6.0+): `session.upload_checkpoint(filename, path)` - uses
    /// embedded client reference Deprecated API:
    /// `session.upload_checkpoint(client, filename, path)` - passing client
    /// explicitly
    #[pyo3(signature = (filename_or_client, filename_or_path=None, path=None))]
    #[tokio_wrap::sync]
    pub fn upload_checkpoint(
        &self,
        py: Python<'_>,
        filename_or_client: &Bound<'_, PyAny>,
        filename_or_path: Option<&Bound<'_, PyAny>>,
        path: Option<PathBuf>,
    ) -> Result<(), Error> {
        // Try to extract as Client first (deprecated API)
        if let Ok(client) = filename_or_client.extract::<PyRef<Client>>() {
            warn_method_deprecated(py, "TrainingSession", "upload_checkpoint")?;
            let filename = filename_or_path
                .ok_or_else(|| {
                    Error::TypeError(
                        "upload_checkpoint() requires 'filename' argument when using deprecated API"
                            .to_string(),
                    )
                })?
                .extract::<String>()?;
            let path = path.unwrap_or_else(|| PathBuf::from(&filename));
            return Ok(self
                .inner
                .upload_checkpoint(&client.0, &filename, path)
                .await?);
        }

        // Try to extract as filename string (new API)
        if let Ok(fname) = filename_or_client.extract::<String>() {
            let client_ref = self.client.as_ref().ok_or_else(|| {
                Error::TypeError(
                    "TrainingSession has no client reference. Use session.upload_checkpoint(client, filename, path) instead."
                        .to_string(),
                )
            })?;
            // In new API: filename_or_path is the path, and path is unused
            let file_path = filename_or_path
                .map(|p| p.extract::<PathBuf>())
                .transpose()?
                .unwrap_or_else(|| PathBuf::from(&fname));
            return Ok(self
                .inner
                .upload_checkpoint(client_ref.as_ref(), &fname, file_path)
                .await?);
        }

        Err(Error::TypeError(
            "upload_checkpoint() first argument must be a string (filename) or Client (deprecated)"
                .to_string(),
        ))
    }

    /// Upload files to the training session.
    ///
    /// New API (v2.6.0+): `session.upload(files)` - uses embedded client
    /// reference Deprecated API: `session.upload(client, files)` - passing
    /// client explicitly
    ///
    /// Args:
    ///     files_or_client: Either List[Tuple[str, Path]] (new API) or Client
    /// (deprecated)     files: List[Tuple[str, Path]] when using deprecated
    /// API
    #[pyo3(signature = (files_or_client, files=None))]
    #[tokio_wrap::sync]
    pub fn upload(
        &self,
        py: Python<'_>,
        files_or_client: &Bound<'_, PyAny>,
        files: Option<Vec<(String, PathBuf)>>,
    ) -> Result<(), Error> {
        // Try to extract as Client first (deprecated API)
        if let Ok(client) = files_or_client.extract::<PyRef<Client>>() {
            warn_method_deprecated(py, "TrainingSession", "upload")?;
            let files = files.ok_or_else(|| {
                Error::TypeError(
                    "upload() requires 'files' argument when using deprecated API".to_string(),
                )
            })?;
            return Ok(self.inner.upload(&client.0, &files).await?);
        }

        // Try to extract as files list (new API)
        if let Ok(files_list) = files_or_client.extract::<Vec<(String, PathBuf)>>() {
            let client_ref = self.client.as_ref().ok_or_else(|| {
                Error::TypeError(
                    "TrainingSession has no client reference. Use session.upload(client, files) instead."
                        .to_string(),
                )
            })?;
            return Ok(self.inner.upload(client_ref.as_ref(), &files_list).await?);
        }

        Err(Error::TypeError(
            "upload() first argument must be a list of (str, Path) tuples or Client (deprecated)"
                .to_string(),
        ))
    }

    /// Download a file from the training session.
    ///
    /// New API (v2.6.0+): `session.download(filename)` - uses embedded client
    /// reference Deprecated API: `session.download(client, filename)` -
    /// passing client explicitly
    ///
    /// Args:
    ///     filename_or_client: Either str filename (new API) or Client
    /// (deprecated)     filename: str filename when using deprecated API
    #[pyo3(signature = (filename_or_client, filename=None))]
    #[tokio_wrap::sync]
    pub fn download(
        &self,
        py: Python<'_>,
        filename_or_client: &Bound<'_, PyAny>,
        filename: Option<String>,
    ) -> Result<String, Error> {
        // Try to extract as Client first (deprecated API)
        if let Ok(client) = filename_or_client.extract::<PyRef<Client>>() {
            warn_method_deprecated(py, "TrainingSession", "download")?;
            let filename = filename.ok_or_else(|| {
                Error::TypeError(
                    "download() requires 'filename' argument when using deprecated API".to_string(),
                )
            })?;
            return Ok(self.inner.download(&client.0, &filename).await?);
        }

        // Try to extract as filename string (new API)
        if let Ok(fname) = filename_or_client.extract::<String>() {
            let client_ref = self.client.as_ref().ok_or_else(|| {
                Error::TypeError(
                    "TrainingSession has no client reference. Use session.download(client, filename) instead."
                        .to_string(),
                )
            })?;
            return Ok(self.inner.download(client_ref.as_ref(), &fname).await?);
        }

        Err(Error::TypeError(
            "download() first argument must be a string (filename) or Client (deprecated)"
                .to_string(),
        ))
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct ValidationSession {
    inner: edgefirst_client::ValidationSession,
    client: Option<Arc<edgefirst_client::Client>>,
}

impl ValidationSession {
    /// Create a ValidationSession with a client reference (for new ergonomic
    /// API)
    fn with_client(
        inner: edgefirst_client::ValidationSession,
        client: Arc<edgefirst_client::Client>,
    ) -> Self {
        Self {
            inner,
            client: Some(client),
        }
    }

    /// Create a ValidationSession without a client reference (legacy)
    #[allow(dead_code)]
    fn without_client(inner: edgefirst_client::ValidationSession) -> Self {
        Self {
            inner,
            client: None,
        }
    }
}

#[pymethods]
impl ValidationSession {
    #[getter]
    pub fn id(&self) -> ValidationSessionID {
        ValidationSessionID(self.inner.id())
    }

    #[getter]
    pub fn uid(&self, py: Python<'_>) -> PyResult<String> {
        warn_uid_deprecated(py, "ValidationSession")?;
        Ok(self.inner.id().to_string())
    }

    #[getter]
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    #[getter]
    pub fn description(&self) -> &str {
        self.inner.description()
    }

    #[getter]
    pub fn dataset_id(&self) -> DatasetID {
        DatasetID(self.inner.dataset_id())
    }

    #[getter]
    pub fn experiment_id(&self) -> ExperimentID {
        ExperimentID(self.inner.experiment_id())
    }

    #[getter]
    pub fn training_session_id(&self) -> TrainingSessionID {
        TrainingSessionID(self.inner.training_session_id())
    }

    #[getter]
    pub fn annotation_set_id(&self) -> AnnotationSetID {
        AnnotationSetID(self.inner.annotation_set_id())
    }

    #[getter]
    pub fn params<'py>(&self, py: Python<'py>) -> Result<Py<PyDict>, Error> {
        let params = PyDict::new(py);

        for (key, value) in self.inner.params() {
            let value = Parameter::from(value.clone());
            params.set_item(key, value.into_pyobject(py)?)?;
        }

        Ok(params.into())
    }

    #[getter]
    pub fn task(&self) -> Task {
        Task(self.inner.task().clone())
    }

    /// Get metrics for this validation session.
    ///
    /// New API (v2.6.0+): `session.metrics()` - uses embedded client reference
    /// Deprecated API: `session.metrics(client)` - passing client explicitly
    #[pyo3(signature = (client=None))]
    #[tokio_wrap::sync]
    pub fn metrics<'py>(
        &self,
        py: Python<'py>,
        client: Option<&Client>,
    ) -> Result<Py<PyDict>, Error> {
        let client_ref = if let Some(c) = client {
            warn_method_deprecated(py, "ValidationSession", "metrics")?;
            &c.0
        } else {
            self.client.as_ref().ok_or_else(|| {
                Error::TypeError(
                    "ValidationSession has no client reference. Use session.metrics(client) instead."
                        .to_string(),
                )
            })?
        };

        let metrics = PyDict::new(py);
        for (key, value) in self.inner.metrics(client_ref).await? {
            let value = Parameter::from(value.clone());
            metrics.set_item(key, value.into_pyobject(py)?)?;
        }

        Ok(metrics.into())
    }

    /// Set metrics for this validation session.
    ///
    /// New API (v2.6.0+): `session.set_metrics(metrics)` - uses embedded client
    /// reference Deprecated API: `session.set_metrics(client, metrics)` -
    /// passing client explicitly
    #[pyo3(signature = (metrics_or_client, metrics=None))]
    #[tokio_wrap::sync]
    pub fn set_metrics<'py>(
        &self,
        py: Python<'py>,
        metrics_or_client: &Bound<'py, PyAny>,
        metrics: Option<HashMap<String, Bound<'py, PyAny>>>,
    ) -> Result<(), Error> {
        // Try to extract as Client first (deprecated API)
        if let Ok(client) = metrics_or_client.extract::<PyRef<Client>>() {
            warn_method_deprecated(py, "ValidationSession", "set_metrics")?;
            let metrics = metrics.ok_or_else(|| {
                Error::TypeError(
                    "set_metrics() requires 'metrics' argument when using deprecated API"
                        .to_string(),
                )
            })?;
            let mut map = HashMap::<String, edgefirst_client::Parameter>::new();
            for (key, value) in metrics {
                let value: Parameter = value.try_into()?;
                map.insert(key, value.into());
            }
            return Ok(self.inner.set_metrics(&client.0, map).await?);
        }

        // Try to extract as dict (new API)
        if let Ok(metrics_dict) = metrics_or_client.extract::<HashMap<String, Bound<'py, PyAny>>>()
        {
            let client_ref = self.client.as_ref().ok_or_else(|| {
                Error::TypeError(
                    "ValidationSession has no client reference. Use session.set_metrics(client, metrics) instead."
                        .to_string(),
                )
            })?;
            let mut map = HashMap::<String, edgefirst_client::Parameter>::new();
            for (key, value) in metrics_dict {
                let value: Parameter = value.try_into()?;
                map.insert(key, value.into());
            }
            return Ok(self.inner.set_metrics(client_ref.as_ref(), map).await?);
        }

        Err(Error::TypeError(
            "set_metrics() first argument must be a dict (metrics) or Client (deprecated)"
                .to_string(),
        ))
    }

    /// Get artifacts for this validation session.
    ///
    /// New API (v2.6.0+): `session.artifacts()` - uses embedded client
    /// reference Deprecated API: `session.artifacts(client)` - passing
    /// client explicitly
    ///
    /// Note: Returns artifacts from the associated training session.
    #[pyo3(signature = (client=None))]
    #[tokio_wrap::sync]
    pub fn artifacts(
        &self,
        py: Python<'_>,
        client: Option<&Client>,
    ) -> Result<Vec<Artifact>, Error> {
        let client_ref = if let Some(c) = client {
            warn_method_deprecated(py, "ValidationSession", "artifacts")?;
            &c.0
        } else {
            self.client.as_ref().ok_or_else(|| {
                Error::TypeError(
                    "ValidationSession has no client reference. Use session.artifacts(client) instead."
                        .to_string(),
                )
            })?
        };

        // ValidationSession uses its associated training_session_id for artifacts
        let artifacts = client_ref
            .artifacts(self.inner.training_session_id())
            .await?
            .into_iter()
            .map(Artifact)
            .collect();
        Ok(artifacts)
    }

    /// Upload files to the validation session.
    ///
    /// New API (v2.6.0+): `session.upload(files)` - uses embedded client
    /// reference Deprecated API: `session.upload(client, files)` - passing
    /// client explicitly
    #[pyo3(signature = (files_or_client, files=None))]
    #[tokio_wrap::sync]
    pub fn upload(
        &self,
        py: Python<'_>,
        files_or_client: &Bound<'_, PyAny>,
        files: Option<Vec<(String, PathBuf)>>,
    ) -> Result<(), Error> {
        // Try to extract as Client first (deprecated API)
        if let Ok(client) = files_or_client.extract::<PyRef<Client>>() {
            warn_method_deprecated(py, "ValidationSession", "upload")?;
            let files = files.ok_or_else(|| {
                Error::TypeError(
                    "upload() requires 'files' argument when using deprecated API".to_string(),
                )
            })?;
            return Ok(self.inner.upload(&client.0, &files).await?);
        }

        // Try to extract as files list (new API)
        if let Ok(files_list) = files_or_client.extract::<Vec<(String, PathBuf)>>() {
            let client_ref = self.client.as_ref().ok_or_else(|| {
                Error::TypeError(
                    "ValidationSession has no client reference. Use session.upload(client, files) instead."
                        .to_string(),
                )
            })?;
            return Ok(self.inner.upload(client_ref.as_ref(), &files_list).await?);
        }

        Err(Error::TypeError(
            "upload() first argument must be a list of (str, Path) tuples or Client (deprecated)"
                .to_string(),
        ))
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct Snapshot {
    inner: edgefirst_client::Snapshot,
    client: Option<Arc<edgefirst_client::Client>>,
}

impl Snapshot {
    /// Create a Snapshot with a client reference (for new ergonomic API)
    fn with_client(
        inner: edgefirst_client::Snapshot,
        client: Arc<edgefirst_client::Client>,
    ) -> Self {
        Self {
            inner,
            client: Some(client),
        }
    }

    /// Create a Snapshot without a client reference (legacy)
    #[allow(dead_code)]
    fn without_client(inner: edgefirst_client::Snapshot) -> Self {
        Self {
            inner,
            client: None,
        }
    }
}

#[pymethods]
impl Snapshot {
    #[getter]
    pub fn id(&self) -> SnapshotID {
        SnapshotID(self.inner.id())
    }

    #[getter]
    pub fn uid(&self, py: Python<'_>) -> PyResult<String> {
        warn_uid_deprecated(py, "Snapshot")?;
        Ok(self.inner.id().to_string())
    }

    #[getter]
    pub fn description(&self) -> &str {
        self.inner.description()
    }

    #[getter]
    pub fn status(&self) -> &str {
        self.inner.status()
    }

    #[getter]
    pub fn path(&self) -> &str {
        self.inner.path()
    }

    #[getter]
    pub fn created(&self) -> String {
        self.inner.created().to_string()
    }

    /// Download this snapshot to a local directory.
    ///
    /// New API (v2.6.0+): `snapshot.download(output_path)` - uses embedded
    /// client reference Deprecated API: `snapshot.download(client,
    /// output_path)` - passing client explicitly
    ///
    /// Args:
    ///     output_or_client: Either the output path (str) or Client
    /// (deprecated)     output: Output path when using deprecated API
    ///
    /// If the Snapshot was created without a client reference (legacy code),
    /// use `client.download_snapshot(snapshot.id, output)` instead.
    #[pyo3(signature = (output_or_client, output=None))]
    #[tokio_wrap::sync]
    pub fn download(
        &self,
        py: Python<'_>,
        output_or_client: &Bound<'_, PyAny>,
        output: Option<String>,
    ) -> Result<(), Error> {
        // Try to extract as Client first (deprecated API)
        if let Ok(client) = output_or_client.extract::<PyRef<Client>>() {
            warn_method_deprecated(py, "Snapshot", "download")?;
            let output_path = output.ok_or_else(|| {
                Error::TypeError("download(client, output) requires output parameter".to_string())
            })?;
            client
                .0
                .download_snapshot(self.inner.id(), std::path::PathBuf::from(output_path), None)
                .await?;
            return Ok(());
        }

        // Try to extract as string (new API)
        if let Ok(output_path) = output_or_client.extract::<String>() {
            let client_ref = self.client.as_ref().ok_or_else(|| {
                Error::TypeError(
                    "Snapshot has no client reference. Use client.download_snapshot(snapshot.id, output) instead."
                        .to_string(),
                )
            })?;
            client_ref
                .download_snapshot(self.inner.id(), std::path::PathBuf::from(output_path), None)
                .await?;
            return Ok(());
        }

        Err(Error::TypeError(
            "download() first argument must be a string (output path) or Client (deprecated)"
                .to_string(),
        ))
    }

    pub fn __repr__(&self) -> String {
        format!(
            "Snapshot(id={}, description='{}', status='{}', path='{}')",
            self.inner.id(),
            self.inner.description(),
            self.inner.status(),
            self.inner.path()
        )
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct SnapshotRestoreResult(edgefirst_client::SnapshotRestoreResult);

#[pymethods]
impl SnapshotRestoreResult {
    #[getter]
    pub fn id(&self) -> SnapshotID {
        SnapshotID(self.0.id)
    }

    #[getter]
    pub fn description(&self) -> &str {
        &self.0.description
    }

    #[getter]
    pub fn dataset_name(&self) -> &str {
        &self.0.dataset_name
    }

    #[getter]
    pub fn dataset_id(&self) -> DatasetID {
        DatasetID(self.0.dataset_id)
    }

    #[getter]
    pub fn annotation_set_id(&self) -> AnnotationSetID {
        AnnotationSetID(self.0.annotation_set_id)
    }

    #[getter]
    pub fn task_id(&self) -> Option<TaskID> {
        self.0.task_id.map(TaskID)
    }

    #[getter]
    pub fn date(&self) -> String {
        self.0.date.to_string()
    }

    pub fn __repr__(&self) -> String {
        let task_id_str = match &self.0.task_id {
            Some(id) => id.to_string(),
            None => "None".to_string(),
        };
        format!(
            "SnapshotRestoreResult(dataset_id={}, dataset_name='{}', annotation_set_id={}, task_id={})",
            self.0.dataset_id, self.0.dataset_name, self.0.annotation_set_id, task_id_str
        )
    }
}

/// Result of creating a snapshot from a dataset.
///
/// Contains the snapshot ID and optional task ID for monitoring progress.
#[pyclass(module = "edgefirst_client")]
pub struct SnapshotFromDatasetResult(edgefirst_client::SnapshotFromDatasetResult);

#[pymethods]
impl SnapshotFromDatasetResult {
    /// The ID of the created snapshot.
    #[getter]
    pub fn id(&self) -> SnapshotID {
        SnapshotID(self.0.id)
    }

    /// The task ID for monitoring snapshot creation progress, if available.
    #[getter]
    pub fn task_id(&self) -> Option<TaskID> {
        self.0.task_id.map(TaskID)
    }

    pub fn __repr__(&self) -> String {
        let task_id_str = match &self.0.task_id {
            Some(id) => id.to_string(),
            None => "None".to_string(),
        };
        format!(
            "SnapshotFromDatasetResult(id={}, task_id={})",
            self.0.id, task_id_str
        )
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct DatasetParams(edgefirst_client::DatasetParams);

#[pymethods]
impl DatasetParams {
    #[getter]
    pub fn dataset_id(&self) -> DatasetID {
        DatasetID(self.0.dataset_id())
    }

    #[getter]
    pub fn annotation_set_id(&self) -> AnnotationSetID {
        AnnotationSetID(self.0.annotation_set_id())
    }

    #[getter]
    pub fn train_group(&self) -> &str {
        self.0.train_group()
    }

    #[getter]
    pub fn val_group(&self) -> &str {
        self.0.val_group()
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct Task(edgefirst_client::Task);

#[pymethods]
impl Task {
    #[getter]
    pub fn id(&self) -> TaskID {
        TaskID(self.0.id())
    }

    #[getter]
    pub fn uid(&self, py: Python<'_>) -> PyResult<String> {
        warn_uid_deprecated(py, "Task")?;
        Ok(self.0.id().to_string())
    }

    #[getter]
    pub fn name(&self) -> &str {
        self.0.name()
    }

    #[getter]
    pub fn workflow(&self) -> &str {
        self.0.workflow()
    }

    #[getter]
    pub fn status(&self) -> &str {
        self.0.status()
    }

    #[getter]
    pub fn manager(&self) -> Option<&str> {
        self.0.manager()
    }

    #[getter]
    pub fn instance(&self) -> &str {
        self.0.instance()
    }

    #[getter]
    pub fn created(&self, py: Python<'_>) -> PyResult<Py<PyDateTime>> {
        Ok(self.0.created().into_pyobject(py)?.into())
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct TaskInfo(edgefirst_client::TaskInfo);

#[pymethods]
impl TaskInfo {
    #[getter]
    pub fn id(&self) -> TaskID {
        TaskID(self.0.id())
    }

    #[getter]
    pub fn uid(&self, py: Python<'_>) -> PyResult<String> {
        warn_uid_deprecated(py, "TaskInfo")?;
        Ok(self.0.id().to_string())
    }

    #[getter]
    pub fn project_id(&self) -> Option<ProjectID> {
        self.0.project_id().map(ProjectID)
    }

    #[getter]
    pub fn status(&self) -> &Option<String> {
        self.0.status()
    }

    #[getter]
    pub fn description(&self) -> &str {
        self.0.description()
    }

    #[getter]
    pub fn stages(&self) -> HashMap<String, Stage> {
        self.0
            .stages()
            .iter()
            .map(|(k, v)| (k.to_string(), Stage(v.clone())))
            .collect()
    }

    #[getter]
    pub fn created(&self, py: Python<'_>) -> PyResult<Py<PyDateTime>> {
        Ok(self.0.created().into_pyobject(py)?.into())
    }

    #[getter]
    pub fn completed(&self, py: Python<'_>) -> PyResult<Py<PyDateTime>> {
        Ok(self.0.completed().into_pyobject(py)?.into())
    }

    #[tokio_wrap::sync]
    pub fn set_status(&mut self, client: &Client, status: &str) -> Result<(), Error> {
        Ok(self.0.set_status(&client.0, status).await?)
    }

    #[tokio_wrap::sync]
    pub fn update_stage(
        &mut self,
        client: &Client,
        stage: &str,
        status: &str,
        message: &str,
        percentage: u8,
    ) -> Result<(), Error> {
        self.0
            .update_stage(&client.0, stage, status, message, percentage)
            .await?;
        Ok(())
    }

    #[tokio_wrap::sync]
    pub fn set_stages(
        &mut self,
        client: &Client,
        stages: Vec<(String, String)>,
    ) -> Result<(), Error> {
        let stages: Vec<(&str, &str)> = stages
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        self.0.set_stages(&client.0, &stages).await?;
        Ok(())
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct Stage(edgefirst_client::Stage);

#[pymethods]
impl Stage {
    #[getter]
    pub fn task_id(&self) -> Option<TaskID> {
        self.0.task_id().map(TaskID)
    }

    #[getter]
    pub fn stage(&self) -> &str {
        self.0.stage()
    }

    #[getter]
    pub fn status(&self) -> &Option<String> {
        self.0.status()
    }

    #[getter]
    pub fn description(&self) -> &Option<String> {
        self.0.description()
    }

    #[getter]
    pub fn message(&self) -> &Option<String> {
        self.0.message()
    }

    #[getter]
    pub fn percentage(&self) -> u8 {
        self.0.percentage()
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct Artifact(edgefirst_client::Artifact);

#[pymethods]
impl Artifact {
    #[getter]
    pub fn name(&self) -> &str {
        self.0.name()
    }

    #[getter]
    pub fn model_type(&self) -> &str {
        self.0.model_type()
    }
}

// =============================================================================
// Token Storage Classes
// =============================================================================

/// File-based token storage for desktop platforms.
///
/// Stores the authentication token in a file on the local filesystem.
/// By default, uses the platform-specific config directory.
#[pyclass(module = "edgefirst_client")]
#[derive(Clone)]
pub struct FileTokenStorage(Arc<edgefirst_client::FileTokenStorage>);

#[pymethods]
impl FileTokenStorage {
    /// Create a new FileTokenStorage using the default platform config
    /// directory.
    #[new]
    #[pyo3(signature = ())]
    pub fn new() -> Result<Self, Error> {
        let storage = edgefirst_client::FileTokenStorage::new()
            .map_err(|e| Error::Error(edgefirst_client::Error::StorageError(e.to_string())))?;
        Ok(FileTokenStorage(Arc::new(storage)))
    }

    /// Create a new FileTokenStorage with a custom file path.
    #[staticmethod]
    pub fn with_path(path: PathBuf) -> Self {
        FileTokenStorage(Arc::new(edgefirst_client::FileTokenStorage::with_path(
            path,
        )))
    }

    /// Returns the path where the token is stored.
    #[getter]
    pub fn path(&self) -> PathBuf {
        self.0.path().clone()
    }

    /// Store a token.
    pub fn store(&self, token: &str) -> Result<(), Error> {
        use edgefirst_client::TokenStorage;
        self.0
            .store(token)
            .map_err(|e| Error::Error(edgefirst_client::Error::StorageError(e.to_string())))?;
        Ok(())
    }

    /// Load the stored token.
    pub fn load(&self) -> Result<Option<String>, Error> {
        use edgefirst_client::TokenStorage;
        self.0
            .load()
            .map_err(|e| Error::Error(edgefirst_client::Error::StorageError(e.to_string())))
    }

    /// Clear the stored token.
    pub fn clear(&self) -> Result<(), Error> {
        use edgefirst_client::TokenStorage;
        self.0
            .clear()
            .map_err(|e| Error::Error(edgefirst_client::Error::StorageError(e.to_string())))?;
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!("FileTokenStorage(path={:?})", self.0.path())
    }
}

/// In-memory token storage (no persistence).
///
/// Stores the authentication token in memory only. The token is lost when
/// the application exits.
#[pyclass(module = "edgefirst_client")]
#[derive(Clone)]
pub struct MemoryTokenStorage(Arc<edgefirst_client::MemoryTokenStorage>);

#[pymethods]
impl MemoryTokenStorage {
    /// Create a new MemoryTokenStorage.
    #[new]
    pub fn new() -> Self {
        MemoryTokenStorage(Arc::new(edgefirst_client::MemoryTokenStorage::new()))
    }

    /// Store a token.
    pub fn store(&self, token: &str) -> Result<(), Error> {
        use edgefirst_client::TokenStorage;
        self.0
            .store(token)
            .map_err(|e| Error::Error(edgefirst_client::Error::StorageError(e.to_string())))?;
        Ok(())
    }

    /// Load the stored token.
    pub fn load(&self) -> Result<Option<String>, Error> {
        use edgefirst_client::TokenStorage;
        self.0
            .load()
            .map_err(|e| Error::Error(edgefirst_client::Error::StorageError(e.to_string())))
    }

    /// Clear the stored token.
    pub fn clear(&self) -> Result<(), Error> {
        use edgefirst_client::TokenStorage;
        self.0
            .clear()
            .map_err(|e| Error::Error(edgefirst_client::Error::StorageError(e.to_string())))?;
        Ok(())
    }

    fn __repr__(&self) -> String {
        "MemoryTokenStorage()".to_string()
    }
}

/// Bridge for Python custom token storage implementations.
///
/// Allows Python objects implementing store/load/clear methods to be used
/// as token storage backends.
struct PyTokenStorageBridge {
    py_storage: Py<PyAny>,
}

impl std::fmt::Debug for PyTokenStorageBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PyTokenStorageBridge").finish()
    }
}

impl PyTokenStorageBridge {
    fn new(py_storage: Py<PyAny>) -> Self {
        Self { py_storage }
    }
}

impl edgefirst_client::TokenStorage for PyTokenStorageBridge {
    fn store(&self, token: &str) -> Result<(), edgefirst_client::StorageError> {
        Python::attach(|py| {
            self.py_storage
                .call_method1(py, "store", (token,))
                .map_err(|e| {
                    edgefirst_client::StorageError::WriteError(format!("Python error: {}", e))
                })?;
            Ok(())
        })
    }

    fn load(&self) -> Result<Option<String>, edgefirst_client::StorageError> {
        Python::attach(|py| {
            let result = self.py_storage.call_method0(py, "load").map_err(|e| {
                edgefirst_client::StorageError::ReadError(format!("Python error: {}", e))
            })?;

            if result.is_none(py) {
                return Ok(None);
            }

            let token: String = result.extract(py).map_err(|e| {
                edgefirst_client::StorageError::ReadError(format!("Failed to extract token: {}", e))
            })?;

            if token.is_empty() {
                Ok(None)
            } else {
                Ok(Some(token))
            }
        })
    }

    fn clear(&self) -> Result<(), edgefirst_client::StorageError> {
        Python::attach(|py| {
            self.py_storage.call_method0(py, "clear").map_err(|e| {
                edgefirst_client::StorageError::ClearError(format!("Python error: {}", e))
            })?;
            Ok(())
        })
    }
}

// Make PyTokenStorageBridge Send + Sync (required for TokenStorage trait)
// This is safe because Python's GIL ensures thread-safe access to Python
// objects
unsafe impl Send for PyTokenStorageBridge {}
unsafe impl Sync for PyTokenStorageBridge {}

// =============================================================================
// Client
// =============================================================================

#[pyclass(module = "edgefirst_client")]
pub struct Client(edgefirst_client::Client);

/// Emit a deprecation warning for constructor parameters.
fn warn_constructor_deprecated(py: Python<'_>, param_name: &str, new_method: &str) -> PyResult<()> {
    let warnings = py.import("warnings")?;
    let message = format!(
        "Client({}) is deprecated and will be removed in v3.0.0. \
         Use Client().{}() instead.",
        param_name, new_method
    );
    warnings.call_method1(
        "warn",
        (
            message,
            py.get_type::<pyo3::exceptions::PyDeprecationWarning>(),
        ),
    )?;
    Ok(())
}

#[pymethods]
impl Client {
    /// Create a new EdgeFirst Client.
    ///
    /// By default, creates a client with file-based token storage that
    /// automatically loads any existing token.
    ///
    /// # Deprecated Parameters
    ///
    /// The following constructor parameters are deprecated:
    /// - `token` - Use `.with_token()` instead
    /// - `username`/`password` - Use `.with_login()` instead
    /// - `server` - Use `.with_server()` instead
    /// - `use_token_file` - Use `.with_storage()` or `.with_memory_storage()`
    ///   instead
    ///
    /// # Examples
    ///
    /// ```python
    /// # New recommended API
    /// client = Client().with_server("test").with_login("user", "pass")
    ///
    /// # With custom storage
    /// client = Client().with_storage(FileTokenStorage.with_path("/custom/path"))
    ///
    /// # Without token persistence
    /// client = Client().with_memory_storage().with_login("user", "pass")
    /// ```
    #[tokio_wrap::sync]
    #[new]
    #[pyo3(signature = (token=None, username=None, password=None, server=None, use_token_file=true))]
    pub fn new(
        py: Python<'_>,
        token: Option<String>,
        username: Option<String>,
        password: Option<String>,
        server: Option<String>,
        use_token_file: bool,
    ) -> Result<Self, Error> {
        // Emit deprecation warnings for deprecated parameters
        if token.is_some() {
            let _ = warn_constructor_deprecated(py, "token=...", "with_token");
        }
        if username.is_some() || password.is_some() {
            let _ = warn_constructor_deprecated(py, "username=..., password=...", "with_login");
        }
        if server.is_some() {
            let _ = warn_constructor_deprecated(py, "server=...", "with_server");
        }
        if !use_token_file {
            let _ = warn_constructor_deprecated(py, "use_token_file=False", "with_memory_storage");
        }

        let client = edgefirst_client::Client::new()?;

        // For backwards compatibility with use_token_file=False, use memory storage
        let client = if !use_token_file {
            client.with_memory_storage()
        } else {
            client
        };

        let client = match token {
            Some(token) => client.with_token(&token)?,
            None => client,
        };

        let client = match server {
            Some(server) => client.with_server(&server)?,
            None => client,
        };

        let client = match (username, password) {
            (Some(username), Some(password)) => client.with_login(&username, &password).await?,
            _ => client,
        };

        Ok(Client(client))
    }

    /// Configure custom token storage.
    ///
    /// Args:
    ///     storage: A storage object (FileTokenStorage, MemoryTokenStorage,
    ///              or any object with store/load/clear methods)
    ///
    /// Returns:
    ///     Client: A new client with the specified storage
    ///
    /// Example:
    ///     >>> storage = FileTokenStorage.with_path("/custom/path")
    ///     >>> client = Client().with_storage(storage)
    pub fn with_storage(&self, _py: Python<'_>, storage: Bound<'_, PyAny>) -> Result<Self, Error> {
        // Check if it's a FileTokenStorage
        if let Ok(file_storage) = storage.extract::<FileTokenStorage>() {
            let new_client = self.0.clone().with_storage(file_storage.0.clone());
            return Ok(Client(new_client));
        }

        // Check if it's a MemoryTokenStorage
        if let Ok(memory_storage) = storage.extract::<MemoryTokenStorage>() {
            let new_client = self.0.clone().with_storage(memory_storage.0.clone());
            return Ok(Client(new_client));
        }

        // Assume it's a Python object with store/load/clear methods
        // Validate that the object has required methods before proceeding
        for method in ["store", "load", "clear"] {
            if !storage.hasattr(method)? {
                return Err(Error::Error(edgefirst_client::Error::InvalidToken)); // or a more appropriate error
            }
        }
        let bridge = PyTokenStorageBridge::new(storage.unbind());
        let new_client = self.0.clone().with_storage(Arc::new(bridge));
        Ok(Client(new_client))
    }

    /// Configure in-memory token storage (no persistence).
    ///
    /// Tokens are stored in memory only and lost when the application exits.
    ///
    /// Returns:
    ///     Client: A new client with memory storage
    ///
    /// Example:
    ///     >>> client = Client().with_memory_storage()
    ///     >>> client = client.with_login("user", "pass")
    pub fn with_memory_storage(&self) -> Self {
        Client(self.0.clone().with_memory_storage())
    }

    /// Disable token storage entirely.
    ///
    /// Tokens are not persisted. Use this when you want to manage tokens
    /// entirely manually.
    ///
    /// Returns:
    ///     Client: A new client without storage
    pub fn with_no_storage(&self) -> Self {
        Client(self.0.clone().with_no_storage())
    }

    /// Configure the server instance.
    ///
    /// The server parameter is an instance name that maps to a URL:
    ///
    /// - ``""`` or ``"saas"`` → ``https://edgefirst.studio`` (default)
    /// - ``"test"`` → ``https://test.edgefirst.studio``
    /// - ``"stage"`` → ``https://stage.edgefirst.studio``
    /// - ``"dev"`` → ``https://dev.edgefirst.studio``
    /// - ``"{name}"`` → ``https://{name}.edgefirst.studio``
    ///
    /// Server Selection Priority:
    ///     1. **Token's server** (highest) - JWT tokens encode their server.
    ///     2. **with_server()** - Used when logging in or no token exists.
    ///     3. **Default "saas"** - If no token and no server specified.
    ///
    /// Important:
    ///     If a token is already set, calling this method will **drop it**
    ///     as tokens are server-specific. Use ``parse_token_server()`` to
    ///     check a token's server before calling this method.
    ///
    /// Args:
    ///     server: Server instance name
    ///
    /// Returns:
    ///     Client: A new client connected to the specified server
    ///
    /// Example:
    ///     >>> client = Client().with_server("test")
    pub fn with_server(&self, server: &str) -> Result<Self, Error> {
        Ok(Client(self.0.with_server(server)?))
    }

    /// Authenticate with a token.
    ///
    /// Args:
    ///     token: JWT authentication token
    ///
    /// Returns:
    ///     Client: A new authenticated client
    ///
    /// Example:
    ///     >>> client = Client().with_token("eyJ...")
    pub fn with_token(&self, token: &str) -> Result<Self, Error> {
        Ok(Client(self.0.with_token(token)?))
    }

    /// Authenticate with username and password.
    ///
    /// Args:
    ///     username: User email or username
    ///     password: User password
    ///
    /// Returns:
    ///     Client: A new authenticated client
    ///
    /// Example:
    ///     >>> client = Client().with_server("test")
    ///     >>> client = client.with_login("user@example.com", "password")
    #[tokio_wrap::sync]
    pub fn with_login(&self, username: &str, password: &str) -> Result<Self, Error> {
        Ok(Client(self.0.with_login(username, password).await?))
    }

    #[tokio_wrap::sync]
    pub fn version(&self) -> Result<String, Error> {
        Ok(self.0.version().await?)
    }

    #[tokio_wrap::sync]
    pub fn logout(&self) -> Result<(), Error> {
        Ok(self.0.logout().await?)
    }

    #[tokio_wrap::sync]
    pub fn token(&self) -> String {
        self.0.token().await
    }

    #[tokio_wrap::sync]
    pub fn verify_token(&self) -> Result<(), Error> {
        Ok(self.0.verify_token().await?)
    }

    #[tokio_wrap::sync]
    pub fn renew_token(&self) -> Result<(), Error> {
        Ok(self.0.renew_token().await?)
    }

    #[tokio_wrap::sync]
    #[getter]
    pub fn token_expiration(&self, py: Python<'_>) -> Result<Py<PyDateTime>, Error> {
        let dt = self.0.token_expiration().await?;
        Ok(dt.into_pyobject(py)?.into())
    }

    #[tokio_wrap::sync]
    #[getter]
    pub fn username(&self) -> Result<String, Error> {
        Ok(self.0.username().await?)
    }

    #[getter]
    pub fn url(&self) -> &str {
        self.0.url()
    }

    /// Returns the server name for the current client (e.g., "saas", "test").
    #[getter]
    pub fn server(&self) -> &str {
        self.0.server()
    }

    #[tokio_wrap::sync]
    pub fn organization(&self) -> Result<Organization, Error> {
        Ok(Organization(self.0.organization().await?))
    }

    #[pyo3(signature = (name = None))]
    #[tokio_wrap::sync]
    pub fn projects(&self, name: Option<&str>) -> Result<Vec<Project>, Error> {
        let client_arc = Arc::new(self.0.clone());
        Ok(self
            .0
            .projects(name)
            .await?
            .into_iter()
            .map(|p| Project::with_client(p, Arc::clone(&client_arc)))
            .collect())
    }

    #[tokio_wrap::sync]
    pub fn project<'py>(&self, project_id: Bound<'py, PyAny>) -> Result<Project, Error> {
        let project_id: ProjectID = project_id.try_into()?;
        let inner = self.0.project(project_id.0).await?;
        Ok(Project::with_client(inner, Arc::new(self.0.clone())))
    }

    #[tokio_wrap::sync]
    pub fn dataset<'py>(&self, dataset_id: Bound<'py, PyAny>) -> Result<Dataset, Error> {
        let dataset_id: DatasetID = dataset_id.try_into()?;
        let inner = self.0.dataset(dataset_id.0).await?;
        Ok(Dataset::with_client(inner, Arc::new(self.0.clone())))
    }

    #[pyo3(signature = (project_id, name = None))]
    #[tokio_wrap::sync]
    pub fn datasets<'py>(
        &self,
        project_id: Bound<'py, PyAny>,
        name: Option<&str>,
    ) -> Result<Vec<Dataset>, Error> {
        let project_id: ProjectID = project_id.try_into()?;
        let client_arc = Arc::new(self.0.clone());
        Ok(self
            .0
            .datasets(project_id.0, name)
            .await?
            .into_iter()
            .map(|d| Dataset::with_client(d, Arc::clone(&client_arc)))
            .collect())
    }

    #[tokio_wrap::sync]
    pub fn labels<'py>(&self, dataset_id: Bound<'py, PyAny>) -> Result<Vec<Label>, Error> {
        let dataset_id: DatasetID = dataset_id.try_into()?;
        let labels = self
            .0
            .labels(dataset_id.0)
            .await?
            .into_iter()
            .map(Label)
            .collect::<Vec<_>>();
        Ok(labels)
    }

    #[tokio_wrap::sync]
    pub fn add_label<'py>(&self, dataset_id: Bound<'py, PyAny>, name: &str) -> Result<(), Error> {
        let dataset_id: DatasetID = dataset_id.try_into()?;
        Ok(self.0.add_label(dataset_id.0, name).await?)
    }

    #[tokio_wrap::sync]
    pub fn remove_label(&self, label_id: u64) -> Result<(), Error> {
        Ok(self.0.remove_label(label_id).await?)
    }

    #[tokio_wrap::sync]
    pub fn update_label(&self, label: &Label) -> Result<(), Error> {
        Ok(self.0.update_label(&label.0).await?)
    }

    #[tokio_wrap::sync]
    #[pyo3(signature = (project_id, name, description=None))]
    pub fn create_dataset<'py>(
        &self,
        project_id: Bound<'py, PyAny>,
        name: &str,
        description: Option<&str>,
    ) -> Result<String, Error> {
        let project_id: ProjectID = project_id.try_into()?;
        let dataset_id = self
            .0
            .create_dataset(project_id.to_string().as_str(), name, description)
            .await?;
        Ok(dataset_id.to_string())
    }

    #[tokio_wrap::sync]
    pub fn delete_dataset<'py>(&self, dataset_id: Bound<'py, PyAny>) -> Result<(), Error> {
        let dataset_id: DatasetID = dataset_id.try_into()?;
        Ok(self.0.delete_dataset(dataset_id.0).await?)
    }

    #[tokio_wrap::sync]
    #[pyo3(signature = (dataset_id, name, description=None))]
    pub fn create_annotation_set<'py>(
        &self,
        dataset_id: Bound<'py, PyAny>,
        name: &str,
        description: Option<&str>,
    ) -> Result<String, Error> {
        let dataset_id: DatasetID = dataset_id.try_into()?;
        let annotation_set_id = self
            .0
            .create_annotation_set(dataset_id.0, name, description)
            .await?;
        Ok(annotation_set_id.to_string())
    }

    #[tokio_wrap::sync]
    pub fn delete_annotation_set<'py>(
        &self,
        annotation_set_id: Bound<'py, PyAny>,
    ) -> Result<(), Error> {
        let annotation_set_id: AnnotationSetID = annotation_set_id.try_into()?;
        Ok(self.0.delete_annotation_set(annotation_set_id.0).await?)
    }

    #[tokio_wrap::sync]
    pub fn annotation_sets<'py>(
        &self,
        dataset_id: Bound<'py, PyAny>,
    ) -> Result<Vec<AnnotationSet>, Error> {
        let dataset_id: DatasetID = dataset_id.try_into()?;
        let client_arc = Arc::new(self.0.clone());
        Ok(self
            .0
            .annotation_sets(dataset_id.0)
            .await?
            .into_iter()
            .map(|s| AnnotationSet::with_client(s, Arc::clone(&client_arc)))
            .collect())
    }

    #[tokio_wrap::sync]
    pub fn annotation_set<'py>(
        &self,
        annotation_set_id: Bound<'py, PyAny>,
    ) -> Result<AnnotationSet, Error> {
        let annotation_set_id: AnnotationSetID = annotation_set_id.try_into()?;
        let inner = self.0.annotation_set(annotation_set_id.0).await?;
        Ok(AnnotationSet::with_client(inner, Arc::new(self.0.clone())))
    }

    #[pyo3(signature = (annotation_set_id, groups = vec![], annotation_types = vec![], progress = None))]
    pub fn annotations<'py>(
        &self,
        annotation_set_id: Bound<'py, PyAny>,
        groups: Vec<String>,
        annotation_types: Vec<AnnotationType>,
        progress: Option<Py<PyAny>>,
    ) -> Result<Vec<Annotation>, Error> {
        let annotation_set_id: AnnotationSetID = annotation_set_id.try_into()?;
        let annotation_types = annotation_types
            .into_iter()
            .map(|x| match x {
                AnnotationType::Box2d => edgefirst_client::AnnotationType::Box2d,
                AnnotationType::Box3d => edgefirst_client::AnnotationType::Box3d,
                AnnotationType::Mask => edgefirst_client::AnnotationType::Mask,
            })
            .collect::<Vec<_>>();

        let annotations = match progress {
            Some(progress) => {
                let (tx, mut rx) = mpsc::channel(1);

                let client = Client(self.0.clone());
                let task = std::thread::spawn(move || {
                    client.annotations_sync(annotation_set_id, &groups, &annotation_types, Some(tx))
                });

                while let Some(status) = rx.blocking_recv() {
                    Python::attach(|py| {
                        progress
                            .call1(py, (status.current, status.total))
                            .expect("Progress callback should be callable and accept a tuple of (current, total) progress.");
                    });
                }

                task.join().unwrap()
            }
            None => self.annotations_sync(annotation_set_id, &groups, &annotation_types, None),
        }?;

        Ok(annotations.into_iter().map(Annotation).collect::<Vec<_>>())
    }

    #[pyo3(signature = (annotation_set_id, groups = vec![], annotation_types = vec![], progress = None))]
    pub fn annotations_dataframe<'py>(
        &self,
        py: Python<'py>,
        annotation_set_id: Bound<'py, PyAny>,
        groups: Vec<String>,
        annotation_types: Vec<AnnotationType>,
        progress: Option<Py<PyAny>>,
    ) -> Result<PyDataFrame, Error> {
        // Emit deprecation warning
        let warnings = py.import("warnings")?;
        warnings.call_method1(
            "warn",
            (
                "Client.annotations_dataframe is deprecated and will be removed in a future version. \
                 Use Client.samples_dataframe instead for complete 2025.10 schema support.",
                py.get_type::<pyo3::exceptions::PyDeprecationWarning>(),
            ),
        )?;

        let annotation_set_id: AnnotationSetID = annotation_set_id.try_into()?;
        let annotation_types = annotation_types
            .into_iter()
            .map(|x| match x {
                AnnotationType::Box2d => edgefirst_client::AnnotationType::Box2d,
                AnnotationType::Box3d => edgefirst_client::AnnotationType::Box3d,
                AnnotationType::Mask => edgefirst_client::AnnotationType::Mask,
            })
            .collect::<Vec<_>>();

        let df = match progress {
            Some(progress) => {
                let (tx, mut rx) = mpsc::channel(1);

                let client = Client(self.0.clone());
                let task = std::thread::spawn(move || {
                    client.annotations_dataframe_sync(
                        annotation_set_id,
                        &groups,
                        &annotation_types,
                        Some(tx),
                    )
                });

                while let Some(status) = rx.blocking_recv() {
                    Python::attach(|py| {
                        progress
                            .call1(py, (status.current, status.total))
                            .expect("Progress callback should be callable and accept a tuple of (current, total) progress.");
                    });
                }

                task.join().unwrap()
            }
            None => {
                self.annotations_dataframe_sync(annotation_set_id, &groups, &annotation_types, None)
            }
        }?;

        Ok(df)
    }

    /// Get samples as a DataFrame with complete 2025.10 schema.
    ///
    /// Args:
    ///     dataset_id: Dataset identifier
    ///     annotation_set_id: Optional annotation set filter
    ///     groups: List of dataset groups (train, val, test)
    ///     annotation_types: List of annotation types (bbox, box3d, mask)
    ///     progress: Optional progress callback
    ///
    /// Returns:
    ///     Polars DataFrame with 13 columns (2025.10 schema)
    ///
    /// Example:
    ///     >>> df = client.samples_dataframe(
    ///     ...     dataset_id,
    ///     ...     annotation_set_id,
    ///     ...     ["train"],
    ///     ...     [],
    ///     ...     None
    ///     ... )
    #[pyo3(signature = (dataset_id, annotation_set_id = None, groups = vec![], annotation_types = vec![], progress = None))]
    pub fn samples_dataframe<'py>(
        &self,
        dataset_id: Bound<'py, PyAny>,
        annotation_set_id: Option<Bound<'py, PyAny>>,
        groups: Vec<String>,
        annotation_types: Vec<AnnotationType>,
        progress: Option<Py<PyAny>>,
    ) -> Result<PyDataFrame, Error> {
        let dataset_id: DatasetID = dataset_id.try_into()?;
        let annotation_set_id = match annotation_set_id {
            Some(id) => Some(id.try_into()?),
            None => None,
        };
        let annotation_types = annotation_types
            .into_iter()
            .map(|x| match x {
                AnnotationType::Box2d => edgefirst_client::AnnotationType::Box2d,
                AnnotationType::Box3d => edgefirst_client::AnnotationType::Box3d,
                AnnotationType::Mask => edgefirst_client::AnnotationType::Mask,
            })
            .collect::<Vec<_>>();

        let df = match progress {
            Some(progress) => {
                let (tx, mut rx) = mpsc::channel(1);

                let client = Client(self.0.clone());
                let task = std::thread::spawn(move || {
                    client.samples_dataframe_sync(
                        dataset_id,
                        annotation_set_id,
                        &groups,
                        &annotation_types,
                        Some(tx),
                    )
                });

                while let Some(status) = rx.blocking_recv() {
                    Python::attach(|py| {
                        progress
                            .call1(py, (status.current, status.total))
                            .expect("Progress callback should be callable and accept a tuple of (current, total) progress.");
                    });
                }

                task.join().unwrap()
            }
            None => self.samples_dataframe_sync(
                dataset_id,
                annotation_set_id,
                &groups,
                &annotation_types,
                None,
            ),
        }?;

        Ok(df)
    }

    #[pyo3(signature = (dataset_id, annotation_set_id = None, annotation_types = vec![], groups = vec![], types = vec![FileType::Image]))]
    #[tokio_wrap::sync]
    pub fn samples_count<'py>(
        &self,
        dataset_id: Bound<'py, PyAny>,
        annotation_set_id: Option<Bound<'py, PyAny>>,
        annotation_types: Vec<AnnotationType>,
        groups: Vec<String>,
        types: Vec<FileType>,
    ) -> Result<SamplesCountResult, Error> {
        let dataset_id: DatasetID = dataset_id.try_into()?;
        let annotation_set_id = match annotation_set_id {
            Some(id) => Some(id.try_into()?),
            None => None,
        };
        let annotation_types = annotation_types
            .into_iter()
            .map(|x| match x {
                AnnotationType::Box2d => edgefirst_client::AnnotationType::Box2d,
                AnnotationType::Box3d => edgefirst_client::AnnotationType::Box3d,
                AnnotationType::Mask => edgefirst_client::AnnotationType::Mask,
            })
            .collect::<Vec<_>>();

        let types = types
            .into_iter()
            .map(|x| match x {
                FileType::Image => edgefirst_client::FileType::Image,
                FileType::LidarPcd => edgefirst_client::FileType::LidarPcd,
                FileType::LidarDepth => edgefirst_client::FileType::LidarDepth,
                FileType::LidarReflect => edgefirst_client::FileType::LidarReflect,
                FileType::RadarPcd => edgefirst_client::FileType::RadarPcd,
                FileType::RadarCube => edgefirst_client::FileType::RadarCube,
            })
            .collect::<Vec<_>>();

        Ok(SamplesCountResult(
            self.0
                .samples_count(
                    dataset_id.0,
                    annotation_set_id.map(|x: AnnotationSetID| x.0),
                    &annotation_types,
                    &groups,
                    &types,
                )
                .await?,
        ))
    }

    #[pyo3(signature = (dataset_id, annotation_set_id = None, annotation_types = vec![], groups = vec![], types = vec![FileType::Image], progress = None))]
    pub fn samples<'py>(
        &self,
        dataset_id: Bound<'py, PyAny>,
        annotation_set_id: Option<Bound<'py, PyAny>>,
        annotation_types: Vec<AnnotationType>,
        groups: Vec<String>,
        types: Vec<FileType>,
        progress: Option<Py<PyAny>>,
    ) -> Result<Vec<Sample>, Error> {
        let dataset_id: DatasetID = dataset_id.try_into()?;
        let annotation_set_id = match annotation_set_id {
            Some(id) => Some(id.try_into()?),
            None => None,
        };
        let annotation_types = annotation_types
            .into_iter()
            .map(|x| match x {
                AnnotationType::Box2d => edgefirst_client::AnnotationType::Box2d,
                AnnotationType::Box3d => edgefirst_client::AnnotationType::Box3d,
                AnnotationType::Mask => edgefirst_client::AnnotationType::Mask,
            })
            .collect::<Vec<_>>();

        let types = types
            .into_iter()
            .map(|x| match x {
                FileType::Image => edgefirst_client::FileType::Image,
                FileType::LidarPcd => edgefirst_client::FileType::LidarPcd,
                FileType::LidarDepth => edgefirst_client::FileType::LidarDepth,
                FileType::LidarReflect => edgefirst_client::FileType::LidarReflect,
                FileType::RadarPcd => edgefirst_client::FileType::RadarPcd,
                FileType::RadarCube => edgefirst_client::FileType::RadarCube,
            })
            .collect::<Vec<_>>();

        let samples = match progress {
            Some(progress) => {
                let (tx, mut rx) = mpsc::channel(1);

                let client = Client(self.0.clone());
                let task = std::thread::spawn(move || {
                    client.samples_sync(
                        dataset_id,
                        annotation_set_id,
                        &annotation_types,
                        &groups,
                        &types,
                        Some(tx),
                    )
                });

                while let Some(status) = rx.blocking_recv() {
                    Python::attach(|py| {
                        progress
                            .call1(py, (status.current, status.total))
                            .expect("Progress callback should be callable and accept a tuple of (current, total) progress.");
                    });
                }

                task.join().unwrap()
            }
            None => self.samples_sync(
                dataset_id,
                annotation_set_id,
                &annotation_types,
                &groups,
                &types,
                None,
            ),
        }?;

        let client_arc = Arc::new(self.0.clone());
        Ok(samples
            .into_iter()
            .map(|s| Sample::with_client(s, Arc::clone(&client_arc)))
            .collect::<Vec<_>>())
    }

    /// Populate samples into a dataset with automatic file uploads.
    ///
    /// This method creates new samples in the specified dataset and
    /// automatically uploads their associated files (images, LiDAR, etc.)
    /// to S3 using presigned URLs.
    ///
    /// The server will auto-generate UUIDs and extract image dimensions for
    /// samples that don't have them specified.
    ///
    /// Args:
    ///     dataset_id: ID of the dataset to populate
    ///     annotation_set_id: ID of the annotation set for sample annotations
    ///     samples: List of Sample objects to create (with files and
    /// annotations)     progress: Optional callback function(current,
    /// total) for upload progress
    ///
    /// Returns:
    ///     List of SamplesPopulateResult objects with UUIDs and presigned URLs
    ///
    /// Example:
    ///     ```python
    ///     from edgefirst_client import Client, Sample, SampleFile, Annotation,
    /// Box2d
    ///
    ///     client = Client()
    ///     sample = Sample()
    ///     sample.set_image_name("test.png")
    ///     sample.add_file(SampleFile("image", "path/to/test.png"))
    ///
    ///     annotation = Annotation()
    ///     annotation.set_label("car")
    ///     annotation.set_box2d(Box2d(10.0, 20.0, 100.0, 50.0))
    ///     sample.add_annotation(annotation)
    ///
    ///     results = client.populate_samples(
    ///         dataset_id,
    ///         annotation_set_id,
    ///         [sample],
    ///         lambda curr, total: print(f"{curr}/{total}")
    ///     )
    ///     ```
    #[pyo3(signature = (dataset_id, annotation_set_id, samples, progress = None))]
    pub fn populate_samples<'py>(
        &self,
        py: Python<'py>,
        dataset_id: Bound<'py, PyAny>,
        annotation_set_id: Bound<'py, PyAny>,
        samples: Vec<Py<Sample>>,
        progress: Option<Py<PyAny>>,
    ) -> Result<Vec<SamplesPopulateResult>, Error> {
        let dataset_id: DatasetID = dataset_id.try_into()?;
        let annotation_set_id: AnnotationSetID = annotation_set_id.try_into()?;

        // Convert Python Sample objects to Rust Sample objects
        let samples: Vec<edgefirst_client::Sample> =
            samples.iter().map(|s| s.borrow(py).inner.clone()).collect();

        let results = match progress {
            Some(progress) => {
                let (tx, mut rx) = mpsc::channel(1);

                let client = Client(self.0.clone());
                let task = std::thread::spawn(move || {
                    client.populate_samples_sync(dataset_id, annotation_set_id, samples, Some(tx))
                });

                while let Some(status) = rx.blocking_recv() {
                    Python::attach(|py| {
                        progress
                            .call1(py, (status.current, status.total))
                            .expect("Progress callback should be callable and accept a tuple of (current, total) progress.");
                    });
                }

                task.join().unwrap()
            }
            None => self.populate_samples_sync(dataset_id, annotation_set_id, samples, None),
        }?;

        Ok(results
            .into_iter()
            .map(SamplesPopulateResult)
            .collect::<Vec<_>>())
    }

    #[pyo3(signature = (dataset_id, groups = vec![], types = vec![FileType::Image], output = ".".into(), flatten = false, progress = None))]
    pub fn download_dataset<'py>(
        &self,
        dataset_id: Bound<'py, PyAny>,
        groups: Vec<String>,
        types: Vec<FileType>,
        output: PathBuf,
        flatten: bool,
        progress: Option<Py<PyAny>>,
    ) -> Result<(), Error> {
        let dataset_id: DatasetID = dataset_id.try_into()?;
        let types = types
            .into_iter()
            .map(|x| match x {
                FileType::Image => edgefirst_client::FileType::Image,
                FileType::LidarPcd => edgefirst_client::FileType::LidarPcd,
                FileType::LidarDepth => edgefirst_client::FileType::LidarDepth,
                FileType::LidarReflect => edgefirst_client::FileType::LidarReflect,
                FileType::RadarPcd => edgefirst_client::FileType::RadarPcd,
                FileType::RadarCube => edgefirst_client::FileType::RadarCube,
            })
            .collect::<Vec<_>>();

        match progress {
            Some(progress) => {
                let (tx, mut rx) = mpsc::channel(1);

                let client = Client(self.0.clone());
                let task = std::thread::spawn(move || {
                    client.download_dataset_sync(
                        dataset_id,
                        &groups,
                        &types,
                        output,
                        flatten,
                        Some(tx),
                    )
                });

                while let Some(status) = rx.blocking_recv() {
                    Python::attach(|py| {
                        progress
                            .call1(py, (status.current, status.total))
                            .expect("Progress callback should be callable and accept a tuple of (current, total) progress.");
                    });
                }

                Ok(task.join().unwrap()?)
            }
            None => {
                Ok(self.download_dataset_sync(dataset_id, &groups, &types, output, flatten, None)?)
            }
        }
    }

    #[tokio_wrap::sync]
    #[pyo3(signature = (project_id, name = None))]
    pub fn experiments<'py>(
        &self,
        project_id: Bound<'py, PyAny>,
        name: Option<&str>,
    ) -> Result<Vec<Experiment>, Error> {
        let project_id: ProjectID = project_id.try_into()?;
        let client_arc = Arc::new(self.0.clone());
        Ok(self
            .0
            .experiments(project_id.0, name)
            .await?
            .into_iter()
            .map(|e| Experiment::with_client(e, Arc::clone(&client_arc)))
            .collect())
    }

    #[tokio_wrap::sync]
    pub fn experiment<'py>(&self, experiment_id: Bound<'py, PyAny>) -> Result<Experiment, Error> {
        let experiment_id: ExperimentID = experiment_id.try_into()?;
        let inner = self.0.experiment(experiment_id.0).await?;
        Ok(Experiment::with_client(inner, Arc::new(self.0.clone())))
    }

    #[tokio_wrap::sync]
    pub fn training_session<'py>(
        &self,
        training_session_id: Bound<'py, PyAny>,
    ) -> Result<TrainingSession, Error> {
        let training_session_id: TrainingSessionID = training_session_id.try_into()?;
        let inner = self.0.training_session(training_session_id.0).await?;
        Ok(TrainingSession::with_client(
            inner,
            Arc::new(self.0.clone()),
        ))
    }

    #[tokio_wrap::sync]
    #[pyo3(signature = (experiment_id, name = None))]
    pub fn training_sessions<'py>(
        &self,
        experiment_id: Bound<'py, PyAny>,
        name: Option<&str>,
    ) -> Result<Vec<TrainingSession>, Error> {
        let experiment_id: ExperimentID = experiment_id.try_into()?;
        let client_arc = Arc::new(self.0.clone());
        Ok(self
            .0
            .training_sessions(experiment_id.0, name)
            .await?
            .into_iter()
            .map(|t| TrainingSession::with_client(t, Arc::clone(&client_arc)))
            .collect())
    }

    #[tokio_wrap::sync]
    pub fn validation_sessions<'py>(
        &self,
        project_id: Bound<'py, PyAny>,
    ) -> Result<Vec<ValidationSession>, Error> {
        let project_id: ProjectID = project_id.try_into()?;
        let client_arc = Arc::new(self.0.clone());
        Ok(self
            .0
            .validation_sessions(project_id.0)
            .await?
            .into_iter()
            .map(|v| ValidationSession::with_client(v, Arc::clone(&client_arc)))
            .collect())
    }

    #[tokio_wrap::sync]
    pub fn validation_session<'py>(
        &self,
        session_id: Bound<'py, PyAny>,
    ) -> Result<ValidationSession, Error> {
        let session_id: ValidationSessionID = session_id.try_into()?;
        let inner = self.0.validation_session(session_id.0).await?;
        Ok(ValidationSession::with_client(
            inner,
            Arc::new(self.0.clone()),
        ))
    }

    #[tokio_wrap::sync]
    pub fn snapshots(&self) -> Result<Vec<Snapshot>, Error> {
        let client_arc = Arc::new(self.0.clone());
        Ok(self
            .0
            .snapshots(None)
            .await?
            .into_iter()
            .map(|s| Snapshot::with_client(s, Arc::clone(&client_arc)))
            .collect())
    }

    #[tokio_wrap::sync]
    pub fn snapshot<'py>(&self, snapshot_id: Bound<'py, PyAny>) -> Result<Snapshot, Error> {
        let snapshot_id: SnapshotID = snapshot_id.try_into()?;
        let inner = self.0.snapshot(snapshot_id.0).await?;
        Ok(Snapshot::with_client(inner, Arc::new(self.0.clone())))
    }

    #[tokio_wrap::sync]
    pub fn delete_snapshot<'py>(&self, snapshot_id: Bound<'py, PyAny>) -> Result<(), Error> {
        let snapshot_id: SnapshotID = snapshot_id.try_into()?;
        Ok(self.0.delete_snapshot(snapshot_id.0).await?)
    }

    #[tokio_wrap::sync]
    pub fn create_snapshot(&self, path: &str) -> Result<Snapshot, Error> {
        let inner = self.0.create_snapshot(path, None).await?;
        Ok(Snapshot::with_client(inner, Arc::new(self.0.clone())))
    }

    #[tokio_wrap::sync]
    pub fn download_snapshot<'py>(
        &self,
        snapshot_id: Bound<'py, PyAny>,
        output: &str,
    ) -> Result<(), Error> {
        let snapshot_id: SnapshotID = snapshot_id.try_into()?;
        self.0
            .download_snapshot(snapshot_id.0, std::path::PathBuf::from(output), None)
            .await?;
        Ok(())
    }

    #[tokio_wrap::sync]
    pub fn restore_snapshot<'py>(
        &self,
        project_id: Bound<'py, PyAny>,
        snapshot_id: Bound<'py, PyAny>,
        topics: Vec<String>,
        autolabel: Vec<String>,
        autodepth: bool,
        dataset_name: Option<String>,
        dataset_description: Option<String>,
    ) -> Result<SnapshotRestoreResult, Error> {
        let project_id: ProjectID = project_id.try_into()?;
        let snapshot_id: SnapshotID = snapshot_id.try_into()?;
        Ok(SnapshotRestoreResult(
            self.0
                .restore_snapshot(
                    project_id.0,
                    snapshot_id.0,
                    &topics,
                    &autolabel,
                    autodepth,
                    dataset_name.as_deref(),
                    dataset_description.as_deref(),
                )
                .await?,
        ))
    }

    /// Create a snapshot from an existing dataset on the server.
    ///
    /// Triggers server-side snapshot generation which exports the dataset's
    /// images and annotations into a downloadable EdgeFirst Dataset Format.
    ///
    /// Args:
    ///     dataset_id: The dataset ID to create snapshot from (DatasetID or
    ///         string like "ds-xxx").
    ///     description: Description for the created snapshot.
    ///     annotation_set_id: Optional annotation set ID. If not provided,
    ///         uses the "annotations" set or first available.
    ///
    /// Returns:
    ///     SnapshotFromDatasetResult containing the snapshot ID and task ID.
    ///
    /// Example:
    ///     >>> result = client.create_snapshot_from_dataset(
    ///     ...     "ds-12345", "My Dataset Backup"
    ///     ... )
    ///     >>> print(f"Created snapshot: {result.id}")
    ///     >>> if result.task_id:
    ///     ...     client.task(result.task_id, monitor=True)
    #[pyo3(signature = (dataset_id, description, annotation_set_id = None))]
    #[tokio_wrap::sync]
    pub fn create_snapshot_from_dataset<'py>(
        &self,
        dataset_id: Bound<'py, PyAny>,
        description: &str,
        annotation_set_id: Option<Bound<'py, PyAny>>,
    ) -> Result<SnapshotFromDatasetResult, Error> {
        let dataset_id: DatasetID = dataset_id.try_into()?;
        let annotation_set_id: Option<AnnotationSetID> =
            annotation_set_id.map(|a| a.try_into()).transpose()?;
        Ok(SnapshotFromDatasetResult(
            self.0
                .create_snapshot_from_dataset(
                    dataset_id.0,
                    description,
                    annotation_set_id.map(|a| a.0),
                )
                .await?,
        ))
    }

    #[tokio_wrap::sync]
    pub fn artifacts<'py>(
        &self,
        training_session_id: Bound<'py, PyAny>,
    ) -> Result<Vec<Artifact>, Error> {
        let training_session_id: TrainingSessionID = training_session_id.try_into()?;
        Ok(self
            .0
            .artifacts(training_session_id.0)
            .await?
            .into_iter()
            .map(Artifact)
            .collect())
    }

    #[pyo3(signature = (training_session_id, modelname, filename = None, progress = None))]
    pub fn download_artifact<'py>(
        &self,
        training_session_id: Bound<'py, PyAny>,
        modelname: &str,
        filename: Option<PathBuf>,
        progress: Option<Py<PyAny>>,
    ) -> Result<(), Error> {
        let training_session_id: TrainingSessionID = training_session_id.try_into()?;
        match progress {
            Some(progress) => {
                let (tx, mut rx) = mpsc::channel(1);

                let client = Client(self.0.clone());
                let modelname = modelname.to_string();

                let task = std::thread::spawn(move || {
                    client.download_artifact_sync(
                        training_session_id,
                        &modelname,
                        filename,
                        Some(tx),
                    )
                });

                while let Some(status) = rx.blocking_recv() {
                    Python::attach(|py| {
                        progress
                            .call1(py, (status.current, status.total))
                            .expect("Progress callback should be callable and accept a tuple of (current, total) progress.");
                    });
                }

                Ok(task.join().unwrap()?)
            }
            None => {
                Ok(self.download_artifact_sync(training_session_id, modelname, filename, None)?)
            }
        }
    }

    #[pyo3(signature = (training_session_id, checkpoint, filename = None, progress = None))]
    pub fn download_checkpoint<'py>(
        &self,
        training_session_id: Bound<'py, PyAny>,
        checkpoint: &str,
        filename: Option<PathBuf>,
        progress: Option<Py<PyAny>>,
    ) -> Result<(), Error> {
        let training_session_id: TrainingSessionID = training_session_id.try_into()?;
        match progress {
            Some(progress) => {
                let (tx, mut rx) = mpsc::channel(1);

                let client = Client(self.0.clone());
                let checkpoint = checkpoint.to_string();

                let task = std::thread::spawn(move || {
                    client.download_checkpoint_sync(
                        training_session_id,
                        &checkpoint,
                        filename,
                        Some(tx),
                    )
                });

                while let Some(status) = rx.blocking_recv() {
                    Python::attach(|py| {
                        progress
                            .call1(py, (status.current, status.total))
                            .expect("Progress callback should be callable and accept a tuple of (current, total) progress.");
                    });
                }

                Ok(task.join().unwrap()?)
            }
            None => Ok(self.download_checkpoint_sync(
                training_session_id,
                checkpoint,
                filename,
                None,
            )?),
        }
    }

    /// Get the list of known tasks for the current user.  If name is provided
    /// then only tasks containing this name will be returned.  The task list
    /// has basic information about each task, for detailed information use
    /// the `task_info` method with the ID of the desired task.
    #[tokio_wrap::sync]
    #[pyo3(signature = (name = None, workflow = None, status = None, manager = None))]
    pub fn tasks(
        &self,
        name: Option<&str>,
        workflow: Option<&str>,
        status: Option<&str>,
        manager: Option<&str>,
    ) -> Result<Vec<Task>, Error> {
        Ok(self
            .0
            .tasks(name, workflow, status, manager)
            .await?
            .into_iter()
            .map(Task)
            .collect())
    }

    /// Get the information about a specific task.
    #[tokio_wrap::sync]
    pub fn task_info(&self, task_id: TaskID) -> Result<TaskInfo, Error> {
        Ok(TaskInfo(self.0.task_info(task_id.0).await?))
    }

    /// Updates the tasks status.
    #[tokio_wrap::sync]
    pub fn task_status(&self, task_id: TaskID, status: &str) -> Result<Task, Error> {
        Ok(Task(self.0.task_status(task_id.0, status).await?))
    }

    /// Configures the task stages.  Stages are used to show various steps
    /// in the task execution process.
    #[tokio_wrap::sync]
    pub fn set_stages(&self, task_id: TaskID, stages: Vec<(String, String)>) -> Result<(), Error> {
        let stages: Vec<(&str, &str)> = stages
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        self.0.set_stages(task_id.0, &stages).await?;
        Ok(())
    }

    /// Updates the stage for the given task.  This is used to show progress
    /// information to the user.
    #[tokio_wrap::sync]
    pub fn update_stage(
        &self,
        task_id: TaskID,
        stage: &str,
        status: &str,
        message: &str,
        percentage: u8,
    ) -> Result<(), Error> {
        self.0
            .update_stage(task_id.0, stage, status, message, percentage)
            .await?;
        Ok(())
    }
}

impl Client {
    #[tokio_wrap::sync]
    fn annotations_sync<'py>(
        &self,
        annotation_set_id: AnnotationSetID,
        groups: &[String],
        annotation_types: &[edgefirst_client::AnnotationType],
        progress: Option<mpsc::Sender<edgefirst_client::Progress>>,
    ) -> Result<Vec<edgefirst_client::Annotation>, edgefirst_client::Error> {
        self.0
            .annotations(annotation_set_id.0, groups, annotation_types, progress)
            .await
    }

    #[allow(deprecated)]
    #[tokio_wrap::sync]
    fn annotations_dataframe_sync<'py>(
        &self,
        annotation_set_id: AnnotationSetID,
        groups: &[String],
        annotation_types: &[edgefirst_client::AnnotationType],
        progress: Option<mpsc::Sender<edgefirst_client::Progress>>,
    ) -> Result<PyDataFrame, edgefirst_client::Error> {
        let df = self
            .0
            .annotations_dataframe(annotation_set_id.0, groups, annotation_types, progress)
            .await?;
        Ok(PyDataFrame(df))
    }

    #[tokio_wrap::sync]
    fn samples_dataframe_sync<'py>(
        &self,
        dataset_id: DatasetID,
        annotation_set_id: Option<AnnotationSetID>,
        groups: &[String],
        annotation_types: &[edgefirst_client::AnnotationType],
        progress: Option<mpsc::Sender<edgefirst_client::Progress>>,
    ) -> Result<PyDataFrame, edgefirst_client::Error> {
        let df = self
            .0
            .samples_dataframe(
                dataset_id.0,
                annotation_set_id.map(|x| x.0),
                groups,
                annotation_types,
                progress,
            )
            .await?;
        Ok(PyDataFrame(df))
    }

    #[tokio_wrap::sync]
    fn samples_sync<'py>(
        &self,
        dataset_id: DatasetID,
        annotation_set_id: Option<AnnotationSetID>,
        annotation_types: &[edgefirst_client::AnnotationType],
        groups: &[String],
        types: &[edgefirst_client::FileType],
        progress: Option<mpsc::Sender<edgefirst_client::Progress>>,
    ) -> Result<Vec<edgefirst_client::Sample>, edgefirst_client::Error> {
        self.0
            .samples(
                dataset_id.0,
                annotation_set_id.map(|x| x.0),
                annotation_types,
                groups,
                types,
                progress,
            )
            .await
    }

    #[tokio_wrap::sync]
    fn populate_samples_sync<'py>(
        &self,
        dataset_id: DatasetID,
        annotation_set_id: AnnotationSetID,
        samples: Vec<edgefirst_client::Sample>,
        progress: Option<mpsc::Sender<edgefirst_client::Progress>>,
    ) -> Result<Vec<edgefirst_client::SamplesPopulateResult>, edgefirst_client::Error> {
        self.0
            .populate_samples(dataset_id.0, Some(annotation_set_id.0), samples, progress)
            .await
    }

    #[tokio_wrap::sync]
    fn download_dataset_sync<'py>(
        &self,
        dataset_id: DatasetID,
        groups: &[String],
        types: &[edgefirst_client::FileType],
        output: PathBuf,
        flatten: bool,
        progress: Option<mpsc::Sender<edgefirst_client::Progress>>,
    ) -> Result<(), edgefirst_client::Error> {
        self.0
            .download_dataset(dataset_id.0, groups, types, output, flatten, progress)
            .await
    }

    #[tokio_wrap::sync]
    fn download_artifact_sync<'py>(
        &self,
        training_session_id: TrainingSessionID,
        modelname: &str,
        filename: Option<PathBuf>,
        progress: Option<mpsc::Sender<edgefirst_client::Progress>>,
    ) -> Result<(), edgefirst_client::Error> {
        self.0
            .download_artifact(training_session_id.0, modelname, filename, progress)
            .await
    }

    #[tokio_wrap::sync]
    fn download_checkpoint_sync<'py>(
        &self,
        training_session_id: TrainingSessionID,
        checkpoint: &str,
        filename: Option<PathBuf>,
        progress: Option<mpsc::Sender<edgefirst_client::Progress>>,
    ) -> Result<(), edgefirst_client::Error> {
        self.0
            .download_checkpoint(training_session_id.0, checkpoint, filename, progress)
            .await
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct SampleFile(edgefirst_client::SampleFile);

#[pymethods]
impl SampleFile {
    /// Creates a new sample file with type and filename for upload.
    ///
    /// Args:
    ///     file_type: Type of the file (e.g., "image", "lidar", "depth")
    ///     filename: Path to the file to upload
    #[new]
    pub fn new(file_type: String, filename: String) -> Self {
        SampleFile(edgefirst_client::SampleFile::with_filename(
            file_type, filename,
        ))
    }

    #[getter]
    pub fn file_type(&self) -> &str {
        self.0.file_type()
    }

    #[getter]
    pub fn filename(&self) -> Option<String> {
        self.0.filename().map(str::to_string)
    }

    #[getter]
    pub fn url(&self) -> Option<String> {
        self.0.url().map(str::to_string)
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct PresignedUrl(edgefirst_client::PresignedUrl);

#[pymethods]
impl PresignedUrl {
    #[getter]
    pub fn filename(&self) -> &str {
        &self.0.filename
    }

    #[getter]
    pub fn key(&self) -> &str {
        &self.0.key
    }

    #[getter]
    pub fn url(&self) -> &str {
        &self.0.url
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct SamplesCountResult(edgefirst_client::SamplesCountResult);

#[pymethods]
impl SamplesCountResult {
    #[getter]
    pub fn total(&self) -> u64 {
        self.0.total
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct SamplesPopulateResult(edgefirst_client::SamplesPopulateResult);

#[pymethods]
impl SamplesPopulateResult {
    #[getter]
    pub fn uuid(&self) -> &str {
        &self.0.uuid
    }

    #[getter]
    pub fn urls(&self) -> Vec<PresignedUrl> {
        self.0
            .urls
            .iter()
            .map(|u| {
                PresignedUrl(edgefirst_client::PresignedUrl {
                    filename: u.filename.clone(),
                    key: u.key.clone(),
                    url: u.url.clone(),
                })
            })
            .collect()
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct Annotation(edgefirst_client::Annotation);

#[pymethods]
impl Annotation {
    /// Creates a new empty annotation.
    #[new]
    pub fn new() -> Self {
        Annotation(edgefirst_client::Annotation::new())
    }

    /// Sets the label for this annotation.
    pub fn set_label(&mut self, label: Option<String>) {
        self.0.set_label(label);
    }

    /// Sets the object identifier for this annotation.
    pub fn set_object_id(&mut self, object_id: Option<String>) {
        self.0.set_object_id(object_id);
    }

    /// Legacy alias for :meth:`set_object_id`.
    #[pyo3(name = "set_object_reference")]
    pub fn set_object_reference_alias(&mut self, object_id: Option<String>) {
        self.0.set_object_id(object_id);
    }

    /// Sets the 2D bounding box for this annotation.
    pub fn set_box2d(&mut self, box2d: Option<&Box2d>) {
        self.0.set_box2d(box2d.map(|b| b.0.clone()));
    }

    /// Sets the 3D bounding box for this annotation.
    pub fn set_box3d(&mut self, box3d: Option<&Box3d>) {
        self.0.set_box3d(box3d.map(|b| b.0.clone()));
    }

    /// Sets the mask for this annotation.
    pub fn set_mask(&mut self, mask: Option<&Mask>) {
        self.0.set_mask(mask.map(|m| m.0.clone()));
    }

    #[getter]
    pub fn sample_id(&self) -> Option<SampleID> {
        self.0.sample_id().map(SampleID)
    }

    #[getter]
    pub fn name(&self) -> Option<String> {
        self.0.name().cloned()
    }

    #[getter]
    pub fn group(&self) -> Option<String> {
        self.0.group().cloned()
    }

    #[getter]
    pub fn sequence_name(&self) -> Option<String> {
        self.0.sequence_name().cloned()
    }

    #[getter]
    pub fn object_id(&self) -> Option<String> {
        self.0.object_id().cloned()
    }

    /// Legacy accessor for ``object_id``.
    #[getter]
    #[pyo3(name = "object_reference")]
    pub fn object_reference_alias(&self) -> Option<String> {
        self.object_id()
    }

    #[getter]
    pub fn label(&self) -> Option<String> {
        self.0.label().cloned()
    }

    #[getter]
    pub fn label_index(&self) -> Option<u64> {
        self.0.label_index()
    }

    #[getter]
    pub fn box2d(&self) -> Option<Box2d> {
        self.0.box2d().map(|x| Box2d(x.clone()))
    }

    #[getter]
    pub fn box3d(&self) -> Option<Box3d> {
        self.0.box3d().map(|x| Box3d(x.clone()))
    }

    #[getter]
    pub fn mask(&self) -> Option<Mask> {
        self.0.mask().map(|x| Mask(x.clone()))
    }
}

#[pyclass(module = "edgefirst_client")]
pub struct Sample {
    inner: edgefirst_client::Sample,
    client: Option<Arc<edgefirst_client::Client>>,
}

impl Sample {
    /// Create a Sample with a client reference (for new ergonomic API)
    fn with_client(inner: edgefirst_client::Sample, client: Arc<edgefirst_client::Client>) -> Self {
        Self {
            inner,
            client: Some(client),
        }
    }

    /// Create a Sample without a client reference (legacy or new samples)
    fn without_client(inner: edgefirst_client::Sample) -> Self {
        Self {
            inner,
            client: None,
        }
    }
}

#[pymethods]
impl Sample {
    /// Creates a new empty sample.
    #[new]
    pub fn new() -> Self {
        Sample::without_client(edgefirst_client::Sample::new())
    }

    /// Sets the image filename for this sample.
    pub fn set_image_name(&mut self, image_name: Option<String>) {
        self.inner.image_name = image_name;
    }

    /// Sets the group for this sample (e.g., "train", "val", "test").
    pub fn set_group(&mut self, group: Option<String>) {
        self.inner.group = group;
    }

    /// Sets the sequence name for this sample.
    pub fn set_sequence_name(&mut self, sequence_name: Option<String>) {
        self.inner.sequence_name = sequence_name;
    }

    /// Sets the sequence UUID for this sample.
    pub fn set_sequence_uuid(&mut self, sequence_uuid: Option<String>) {
        self.inner.sequence_uuid = sequence_uuid;
    }

    /// Sets the sequence description for this sample.
    pub fn set_sequence_description(&mut self, sequence_description: Option<String>) {
        self.inner.sequence_description = sequence_description;
    }

    /// Sets the frame number for this sample.
    pub fn set_frame_number(&mut self, frame_number: Option<u32>) {
        self.inner.frame_number = frame_number;
    }

    /// Adds a file to this sample.
    pub fn add_file(&mut self, file: &SampleFile) {
        self.inner.files.push(file.0.clone());
    }

    /// Adds an annotation to this sample.
    pub fn add_annotation(&mut self, annotation: &Annotation) {
        self.inner.annotations.push(annotation.0.clone());
    }

    #[getter]
    pub fn id(&self) -> Option<SampleID> {
        self.inner.id().map(SampleID)
    }

    #[getter]
    pub fn uid(&self, py: Python<'_>) -> PyResult<Option<String>> {
        warn_uid_deprecated(py, "Sample")?;
        Ok(self.inner.id().map(|id| id.to_string()))
    }

    #[getter]
    pub fn name(&self) -> Option<String> {
        self.inner.name()
    }

    #[getter]
    pub fn group(&self) -> Option<String> {
        self.inner.group().cloned()
    }

    #[getter]
    pub fn sequence_name(&self) -> Option<String> {
        self.inner.sequence_name().cloned()
    }

    #[getter]
    pub fn sequence_uuid(&self) -> Option<String> {
        self.inner.sequence_uuid().cloned()
    }

    #[getter]
    pub fn sequence_description(&self) -> Option<String> {
        self.inner.sequence_description().cloned()
    }

    #[getter]
    pub fn frame_number(&self) -> Option<u32> {
        self.inner.frame_number()
    }

    #[getter]
    pub fn uuid(&self) -> Option<String> {
        self.inner.uuid().cloned()
    }

    #[getter]
    pub fn image_name(&self) -> Option<String> {
        self.inner.image_name().map(str::to_string)
    }

    #[getter]
    pub fn image_url(&self) -> Option<String> {
        self.inner.image_url().map(str::to_string)
    }

    #[getter]
    pub fn width(&self) -> Option<u32> {
        self.inner.width()
    }

    #[getter]
    pub fn height(&self) -> Option<u32> {
        self.inner.height()
    }

    #[getter]
    pub fn date(&self) -> Option<String> {
        self.inner.date().map(|d| d.to_rfc3339())
    }

    #[getter]
    pub fn source(&self) -> Option<String> {
        self.inner.source().cloned()
    }

    #[getter]
    pub fn files(&self) -> Vec<SampleFile> {
        self.inner
            .files()
            .iter()
            .map(|f| SampleFile(f.clone()))
            .collect()
    }

    #[getter]
    pub fn annotations(&self) -> Vec<Annotation> {
        self.inner
            .annotations()
            .iter()
            .map(|x| Annotation(x.clone()))
            .collect()
    }

    /// Download sample file data.
    ///
    /// New API (v2.6.0+): `sample.download(file_type)` - uses embedded client
    /// reference Deprecated API: `sample.download(client, file_type)` -
    /// passing client explicitly
    ///
    /// Note: For downloading multiple samples, use `dataset.download()` which
    /// is far higher performance due to batch downloading.
    ///
    /// Args:
    ///     file_type_or_client: Either FileType (new API) or Client
    /// (deprecated)     file_type: FileType when using deprecated API
    ///
    /// Returns:
    ///     Optional bytes of the downloaded file content
    ///
    /// If the Sample was created without a client reference (e.g.,
    /// Sample.new()), you must use the deprecated API with a client
    /// parameter.
    #[pyo3(signature = (file_type_or_client=None, file_type=None))]
    #[tokio_wrap::sync]
    pub fn download(
        &self,
        py: Python<'_>,
        file_type_or_client: Option<&Bound<'_, PyAny>>,
        file_type: Option<FileType>,
    ) -> Result<Option<Vec<u8>>, Error> {
        // Convert FileType enum to client type
        fn convert_file_type(ft: FileType) -> edgefirst_client::FileType {
            match ft {
                FileType::Image => edgefirst_client::FileType::Image,
                FileType::LidarPcd => edgefirst_client::FileType::LidarPcd,
                FileType::LidarDepth => edgefirst_client::FileType::LidarDepth,
                FileType::LidarReflect => edgefirst_client::FileType::LidarReflect,
                FileType::RadarPcd => edgefirst_client::FileType::RadarPcd,
                FileType::RadarCube => edgefirst_client::FileType::RadarCube,
            }
        }

        // No argument: use embedded client with default FileType::Image
        if file_type_or_client.is_none() {
            let client_ref = self.client.as_ref().ok_or_else(|| {
                Error::TypeError(
                    "Sample has no client reference. Use sample.download(client) instead."
                        .to_string(),
                )
            })?;
            let ft = convert_file_type(FileType::Image);
            return Ok(self.inner.download(client_ref.as_ref(), ft).await?);
        }

        let first_arg = file_type_or_client.unwrap();

        // Try to extract as Client first (deprecated API)
        if let Ok(client) = first_arg.extract::<PyRef<Client>>() {
            warn_method_deprecated(py, "Sample", "download")?;
            let ft = convert_file_type(file_type.unwrap_or(FileType::Image));
            return Ok(self.inner.download(&client.0, ft).await?);
        }

        // Try to extract as FileType (new API)
        if let Ok(ft_enum) = first_arg.extract::<FileType>() {
            let client_ref = self.client.as_ref().ok_or_else(|| {
                Error::TypeError(
                    "Sample has no client reference. Use sample.download(client, file_type) instead."
                        .to_string(),
                )
            })?;
            let ft = convert_file_type(ft_enum);
            return Ok(self.inner.download(client_ref.as_ref(), ft).await?);
        }

        Err(Error::TypeError(
            "download() first argument must be a FileType or Client (deprecated)".to_string(),
        ))
    }
}

/// EdgeFirst Client Library
/// This library provides a client for the EdgeFirst API, allowing users to
/// interact with the EdgeFirst Studio Server and perform various operations
/// such as logging in, managing projects, and working with datasets and
/// snapshots.
///
/// This is the Python API binding for the EdgeFirst Client Library.  A Rust
/// library and a command-line interface are also available.  The CLI tool is
/// bundled with the Python wheel and can be called using the
/// `edgefirst-client` command.
///
/// The Python API is available as the `edgefirst_client` module.  The main
/// class is `Client`, which provides methods for interacting with the EdgeFirst
/// Studio Server.  To use the `Client` object you need to create an instance,
/// with an optional authentication token or username and password.
#[pymodule(name = "edgefirst_client")]
fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // ID types
    m.add_class::<ProjectID>()?;
    m.add_class::<DatasetID>()?;
    m.add_class::<ExperimentID>()?;
    m.add_class::<OrganizationID>()?;
    m.add_class::<SampleID>()?;
    m.add_class::<AnnotationSetID>()?;
    m.add_class::<TaskID>()?;
    m.add_class::<TrainingSessionID>()?;
    m.add_class::<ValidationSessionID>()?;
    m.add_class::<SnapshotID>()?;
    m.add_class::<ImageId>()?;
    m.add_class::<SequenceId>()?;
    m.add_class::<AppId>()?;

    // Storage classes
    m.add_class::<FileTokenStorage>()?;
    m.add_class::<MemoryTokenStorage>()?;

    // Client
    m.add_class::<Client>()?;
    m.add_class::<Project>()?;
    m.add_class::<Experiment>()?;
    m.add_class::<TrainingSession>()?;
    m.add_class::<ValidationSession>()?;
    m.add_class::<Snapshot>()?;
    m.add_class::<SnapshotRestoreResult>()?;
    m.add_class::<SnapshotFromDatasetResult>()?;
    m.add_class::<AnnotationSet>()?;
    m.add_class::<AnnotationType>()?;
    m.add_class::<Dataset>()?;
    m.add_class::<Box2d>()?;
    m.add_class::<Box3d>()?;
    m.add_class::<Mask>()?;
    m.add_class::<Sample>()?;
    m.add_class::<SampleFile>()?;
    m.add_class::<FileType>()?;
    m.add_class::<Annotation>()?;
    m.add_class::<PresignedUrl>()?;
    m.add_class::<SamplesCountResult>()?;
    m.add_class::<SamplesPopulateResult>()?;
    m.add_class::<DatasetParams>()?;
    m.add_class::<Parameter>()?;
    m.add_class::<Task>()?;
    m.add_class::<TaskInfo>()?;
    m.add_class::<Stage>()?;

    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(is_polars_enabled, m)?)?;

    Ok(())
}

#[pyfunction]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

#[pyfunction]
pub fn is_polars_enabled() -> bool {
    #[cfg(feature = "polars")]
    {
        true
    }
    #[cfg(not(feature = "polars"))]
    {
        false
    }
}
