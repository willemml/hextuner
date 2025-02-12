//! Rust friendly representations of XDF data types as I see them.
//!
//! Some XDF features are not implemented, and some assumptions are
//! made as to what must be present in a file. These should work
//! with the existing XDF for the Nefmoto Stage 1 community project
//! for 1.8T AMB engines, specifically the 8E0909518AK-0003 ECU
//! software.

use std::{
    collections::HashMap,
    io::{Read, Seek, Write},
};

use xdftuneparser::data_types::*;

use crate::eval::{eval, eval_reverse};

fn bytes_to_u32(bytes: &[u8]) -> u32 {
    let mut final_bytes = [0; 4];
    if bytes.len() > 4 {
        panic!("too big");
    } else {
        for i in 0..bytes.len() {
            final_bytes[4 - bytes.len() + i] = bytes[bytes.len() - i - 1];
        }
    }
    u32::from_be_bytes(final_bytes)
}

/// Binary definition metadata
#[derive(Debug, Clone)]
pub struct DefinitionInfo {
    pub name: String,
    pub description: String,
}

impl DefinitionInfo {
    pub fn from_xdf(xdf: XDFHeader) -> Self {
        Self {
            name: xdf.deftitle.unwrap_or_default(),
            description: xdf.description.unwrap_or_default(),
        }
    }
}

/// Single editable value
#[derive(Debug, Clone)]
pub struct Constant {
    pub name: String,
    pub description: String,
    /// Binary offset from beginning of file
    pub address: u64,
    /// Size of stored value in bytes (max 4 with current implementation)
    pub size: usize,
    /// Equation to convert between integer representation and human readable value
    pub expression: String,
}

impl Constant {
    pub fn from_xdf(xdf: XDFConstant) -> Self {
        let edata = xdf.embedded_data.unwrap();
        let math = xdf.math.unwrap();
        let address = edata.mmedaddress.unwrap() as u64;
        let size = (edata.mmedelementsizebits.unwrap() / 8) as usize;
        let name = xdf.title.unwrap_or_default();
        let description = xdf.description.unwrap_or_default();
        let expression = math.expression.unwrap_or_default();

        Self {
            name,
            description,
            address,
            size,
            expression,
        }
    }

    pub fn read<R: Read + Seek>(&self, bin: &mut R) -> Result<f64, std::io::Error> {
        bin.seek(std::io::SeekFrom::Start(self.address))?;
        let mut buf = vec![0u8; self.size];
        bin.read_exact(&mut buf)?;
        Ok(eval(&self.expression, bytes_to_u32(&buf)))
    }

    pub fn write<W: Write + Seek>(&self, bin: &mut W, val: f64) -> Result<(), std::io::Error> {
        bin.seek(std::io::SeekFrom::Start(self.address))?;
        let bytes = (eval_reverse(&self.expression, val).round() as u32).to_be_bytes();
        let mut buf = vec![];
        for i in 0..self.size {
            buf.push(bytes[bytes.len() - i - 1]);
        }
        bin.write_all(&mut buf)
    }
}

/// Axis data, can be stored values or user defined constants
#[derive(Debug, Clone)]
pub enum AxisData {
    /// User defined axis, not stored in binary
    User(Vec<f64>),
    /// Axis data defined in binary
    Binary {
        address: u64,
        /// Size in bytes of one element (max 8 with current implementation)
        element_size: usize,
        /// Total number of elements, should equal product of rows and columns
        count: usize,
        /// Number of rows, should be 1 for X axis
        rows: usize,
        /// Number of columns, should be 1 for Y axis
        columns: usize,
        /// Equation to convert betwen integer representation and human readable value
        expression: String,
    },
}

/// Axis of a table
#[derive(Debug, Clone)]
pub struct Axis {
    pub units: String,
    pub data: AxisData,
}

impl Axis {
    pub fn from_xdf(xdf: XDFAxis, linked: Option<&HashMap<u32, EmbeddedData>>) -> Self {
        // If there are no labels this must be an internally defined axis
        let data = if xdf.labels.is_empty() {
            let edata = xdf.embeddeddata.unwrap();

            // Logic to get data storage information from linked object if it is missing
            let edata = if edata.mmedaddress.is_some()
                && (edata.mmedcolcount.is_some() || edata.mmedrowcount.is_some())
            {
                edata
            } else {
                let link_id = xdf
                    .embedinfo
                    .as_ref()
                    .map(|ei| ei.linkobjid.unwrap())
                    .unwrap();
                linked.unwrap().get(&link_id).cloned().unwrap()
            };
            let address = edata.mmedaddress.unwrap() as u64;

            // There must be at least one of row or column count defined,
            // otherwise there is no way of knowing how to organize the data.
            assert!(edata.mmedrowcount.is_some() || edata.mmedcolcount.is_some());
            let rows = edata.mmedrowcount.unwrap_or(1) as usize;
            let columns = edata.mmedcolcount.unwrap_or(1) as usize;

            let count = rows * columns;

            // Make sure count is as expected, otherwise row or column counts are wrong.
            if let Some(icount) = xdf.count {
                assert_eq!(icount as usize, count);
            }

            // Element size must be defined or we might was well display random numbers.
            let element_size = edata.mmedelementsizebits.unwrap() as usize;

            let math = xdf.math.unwrap();
            assert_eq!(math.vars.len(), 1);

            // Because we only allow one variable normalize it to 'X'
            let expression = math.expression.unwrap().replace(math.vars[0].as_str(), "X");

            AxisData::Binary {
                address,
                element_size,
                count,
                rows,
                columns,
                expression,
            }
        } else {
            AxisData::User(
                xdf.labels
                    .iter()
                    .map(|l| {
                        l.value
                            .as_ref()
                            .map_or(0.0, |s| s.parse().unwrap_or_default())
                    })
                    .collect(),
            )
        };

        Self {
            units: xdf.unit.unwrap_or_default(),
            data,
        }
    }
    pub fn read<R: Read + Seek>(&self, bin: &mut R) -> Result<Vec<f64>, std::io::Error> {
        match &self.data {
            AxisData::User(items) => Ok(items.clone()),
            AxisData::Binary {
                address,
                element_size,
                count,
                expression,
                ..
            } => {
                bin.seek(std::io::SeekFrom::Start(*address))?;
                let mut buf = vec![0u8; *element_size];

                let mut result = Vec::with_capacity(*count);

                for _ in 0..*count {
                    bin.read_exact(&mut buf)?;
                    result.push(eval(&expression, bytes_to_u32(&buf)));
                }

                Ok(result)
            }
        }
    }
    pub fn write<W: Write + Seek>(
        &self,
        bin: &mut W,
        vals: Vec<f64>,
    ) -> Result<(), std::io::Error> {
        match &self.data {
            AxisData::User(_) => panic!("Cannot write user defined constant values to binary"),
            AxisData::Binary {
                address,
                element_size,
                count,
                expression,
                ..
            } => {
                assert_eq!(count, &vals.len());
                bin.seek(std::io::SeekFrom::Start(*address))?;
                let mut buf = vec![];
                for val in vals {
                    let bytes = (eval_reverse(&expression, val).round() as u32).to_be_bytes();
                    for i in 0..*element_size {
                        buf.push(bytes[bytes.len() - i]);
                    }
                }
                bin.write_all(&mut buf)
            }
        }
    }
}

/// Multivalue map data definitions
#[derive(Debug, Clone)]
pub struct Table {
    pub name: String,
    pub description: String,
    /// Column labels
    pub x: Axis,
    /// Row labels
    pub y: Axis,
    /// Primary map axis
    pub z: Axis,
}

impl Table {
    pub fn from_xdf(mut xdf: XDFTable, linked: Option<&HashMap<u32, EmbeddedData>>) -> Self {
        let name = xdf.title.unwrap_or_default();
        let description = xdf.description.unwrap_or_default();

        // Test file always has 3 axis per table, should be updated later.
        assert_eq!(xdf.axis.len(), 3);

        // For now we assume that all IDs are one of x, y, and z.
        xdf.axis.sort_by_key(|a| a.id.clone());

        // Because they are now sorted, we can just pop them and the following should work.
        let z = Axis::from_xdf(xdf.axis.pop().unwrap(), linked);
        let y = Axis::from_xdf(xdf.axis.pop().unwrap(), linked);
        let x = Axis::from_xdf(xdf.axis.pop().unwrap(), linked);

        Self {
            name,
            description,
            x,
            y,
            z,
        }
    }
}

/// Definitions for a binary, metadata
#[derive(Debug, Clone)]
pub struct BinaryDefinition {
    pub info: DefinitionInfo,
    pub constants: Vec<Constant>,
    pub tables: Vec<Table>,
}

impl BinaryDefinition {
    pub fn from_xdf(xdf: XDFFormat) -> Self {
        // This allows me to support linked objects, where the axis is defined in a different table.
        let mut table_zs = HashMap::new();
        for table in xdf.tables.iter() {
            if let Some(uid) = table.uid.clone() {
                for axis in table.axis.iter() {
                    if axis.id.as_ref().unwrap().to_lowercase() == "z" {
                        if let Some(edata) = axis.embeddeddata.clone() {
                            table_zs.insert(uid, edata);
                        }
                    }
                }
            }
        }
        Self {
            info: DefinitionInfo::from_xdf(xdf.header.unwrap()),
            constants: xdf.constants.into_iter().map(Constant::from_xdf).collect(),
            tables: xdf
                .tables
                .into_iter()
                .map(|t| Table::from_xdf(t, Some(&table_zs)))
                .collect(),
        }
    }
}
