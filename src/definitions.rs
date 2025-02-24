//! Rust friendly representations of XDF data types as I see them.
//!
//! Some XDF features are not implemented, and some assumptions are
//! made as to what must be present in a file. These should work
//! with the existing XDF for the Nefmoto Stage 1 community project
//! for 1.8T AMB engines, specifically the 8E0909518AK-0003 ECU
//! software.
//! Also seems to work with the 2.7t community file.

// TODO: Still need to implement min/max values, will probably do
// this based on stored value bit precision.
// (eval raw::MAX and raw ::MIN)

use core::f64;
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
    pub categories: HashMap<u32, String>,
}

impl DefinitionInfo {
    pub fn from_xdf(xdf: XDFHeader) -> Self {
        let mut categories = HashMap::new();

        for Category { index, name } in xdf.category {
            if let Some(index) = index {
                categories.insert(index, name.unwrap_or(format!("{:x}", index)));
            }
        }

        Self {
            categories,
            name: xdf.deftitle.unwrap_or_default(),
            description: xdf.description.unwrap_or_default(),
        }
    }
}

/// Single editable value
#[derive(Debug, Clone)]
pub struct Scalar {
    pub name: String,
    pub description: String,
    /// Binary offset from beginning of file
    pub address: u64,
    /// Size of stored value in bytes (max 4 with current implementation)
    pub size: usize,
    /// Equation to convert between integer representation and human readable value
    pub expression: String,
    pub categories: Vec<u32>,
}

impl Scalar {
    pub fn from_xdf(xdf: XDFConstant) -> Self {
        let edata = xdf.embedded_data.unwrap();
        let math = xdf.math.unwrap();
        let address = edata.mmedaddress.unwrap() as u64;
        let size = (edata.mmedelementsizebits.unwrap() / 8) as usize;
        let name = xdf.title.unwrap_or_default();
        let description = xdf.description.unwrap_or_default();
        let expression = math.expression.unwrap_or_default();
        let categories = xdf.catmem.into_iter().filter_map(|c| c.category).collect();

        Self {
            categories,
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
    pub fn len(&self) -> usize {
        match &self.data {
            AxisData::User(v) => v.len(),
            AxisData::Binary { count, .. } => *count,
        }
    }
    pub fn writeable(&self) -> bool {
        match self.data {
            AxisData::User(_) => false,
            AxisData::Binary { .. } => true,
        }
    }
    pub fn range(&self) -> Option<(f64, f64)> {
        if let AxisData::Binary {
            element_size,
            expression,
            ..
        } = &self.data
        {
            let mut bytes = [0u8; 4];
            for i in 0..*element_size {
                bytes[i] = 0xFF;
            }

            let num = u32::from_be_bytes(bytes);

            Some((eval(&expression, 0), eval(&expression, num)))
        } else {
            None
        }
    }
    pub fn precision(&self) -> Option<usize> {
        if let AxisData::Binary { expression, .. } = &self.data {
            let avg = (0..20)
                .map(|n| eval(&expression, n))
                .map_windows(|[a, b]| (a - b).abs())
                .reduce(|a, e| a + e)
                .unwrap()
                / 20.0;

            Some(avg.recip().log10().round() as usize + 1)
        } else {
            None
        }
    }
    pub fn from_xdf(xdf: XDFAxis, linked: Option<&HashMap<u32, (EmbeddedData, Math)>>) -> Self {
        // If there are no labels this must be an internally defined axis
        let data = if xdf.labels.is_empty() {
            let mut edata = xdf.embeddeddata.unwrap();
            let math;

            // Logic to get data storage information from linked object if it is missing
            if let Some(Some(link_id)) = xdf.embedinfo.map(|e| e.linkobjid) {
                let linked = linked.unwrap().get(&link_id).cloned().unwrap();
                edata = linked.0;
                math = linked.1;
            } else if edata.mmedaddress.is_some()
                && (edata.mmedcolcount.is_some()
                    || edata.mmedrowcount.is_some()
                    || xdf.count.is_some())
            {
                math = xdf.math.unwrap();
            } else {
                panic!("Found no valid embed data for data axis.");
            };

            assert_eq!(math.vars.len(), 1);

            let address = edata.mmedaddress.unwrap() as u64;

            let count = if let Some(c) = xdf.count {
                c
            } else if let (Some(c), Some(r)) = (edata.mmedcolcount, edata.mmedrowcount) {
                r * c
            } else if let Some(c) = edata.mmedcolcount {
                c
            } else {
                edata.mmedrowcount.unwrap()
            } as usize;

            // Element size must be defined or we might was well display random numbers.
            let element_size = edata.mmedelementsizebits.unwrap() as usize / 8;

            // Because we only allow one variable normalize it to 'X'
            let expression = math.expression.unwrap().replace(math.vars[0].as_str(), "X");

            AxisData::Binary {
                address,
                element_size,
                count,
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
    pub fn read_strings<R: Read + Seek>(&self, bin: &mut R) -> Result<Vec<String>, std::io::Error> {
        let floats = self.read(bin)?;

        Ok(if let Some(p) = self.precision() {
            floats.iter().map(|v| format!("{:.p$}", v)).collect()
        } else {
            floats.iter().map(f64::to_string).collect()
        })
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
            } => {
                assert_eq!(count, &vals.len());
                bin.seek(std::io::SeekFrom::Start(*address))?;
                let mut buf = vec![];
                for val in vals {
                    let bytes = (eval_reverse(&expression, val).round() as u32).to_be_bytes();
                    for i in 0..*element_size {
                        buf.push(bytes[bytes.len() - i - 1]);
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
    pub categories: Vec<u32>,
}

impl Table {
    pub fn from_xdf(
        mut xdf: XDFTable,
        linked: Option<&HashMap<u32, (EmbeddedData, Math)>>,
    ) -> Self {
        let name = xdf.title.unwrap_or_default();
        let description = xdf.description.unwrap_or_default();
        let categories = xdf
            .catmem
            .into_iter()
            .filter_map(|c| c.category.map(|v| v - 1))
            .collect();

        // Test file always has 3 axis per table, should be updated later.
        assert_eq!(xdf.axis.len(), 3);

        // For now we assume that all IDs are one of x, y, and z.
        xdf.axis.sort_by_key(|a| a.id.clone());

        // Because they are now sorted, we can just pop them and the following should work.
        let z = Axis::from_xdf(xdf.axis.pop().unwrap(), linked);
        let y = Axis::from_xdf(xdf.axis.pop().unwrap(), linked);
        let x = Axis::from_xdf(xdf.axis.pop().unwrap(), linked);

        Self {
            categories,
            name,
            description,
            x,
            y,
            z,
        }
    }
    pub fn build_array(&self, bin: &mut std::fs::File) -> std::io::Result<Vec<Vec<String>>> {
        // add one to length for row/column headers
        let xl = self.x.len();
        let yl = self.y.len();

        // read rows headers into buffer with one cell of padding for column headers
        let mut buf = vec![0.0];
        buf.append(&mut self.x.read(bin)?);

        let mut buf: Vec<String> = buf.into_iter().map(|f| f.to_string()).collect();

        buf[0] = "".into();

        let row_head = self.y.read(bin)?;

        let mut data = self.z.read(bin)?;
        // reverse data so we can use pop to put it in the table in order correctly
        data.reverse();

        let mut table = Vec::new();
        table.push(buf.split_off(0));

        for y in 0..yl {
            // add the row "header"
            buf.push(row_head[y].to_string());

            for _ in 0..xl {
                if let Some(d) = data.pop() {
                    buf.push(d.to_string());
                } else {
                    break;
                }
            }

            table.push(buf.split_off(0));
        }

        Ok(table)
    }
}

/// Definitions for a binary, metadata
#[derive(Debug, Clone)]
pub struct BinaryDefinition {
    pub info: DefinitionInfo,
    pub scalars: Vec<Scalar>,
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
                        table_zs.insert(
                            uid,
                            (axis.embeddeddata.unwrap(), axis.math.clone().unwrap()),
                        );
                    }
                }
            }
        }
        Self {
            info: DefinitionInfo::from_xdf(xdf.header.unwrap()),
            scalars: xdf.constants.into_iter().map(Scalar::from_xdf).collect(),
            tables: xdf
                .tables
                .into_iter()
                .map(|t| Table::from_xdf(t, Some(&table_zs)))
                .collect(),
        }
    }
}
