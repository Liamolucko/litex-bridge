use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

/// Information about a LiteX SoC, mainly the the addresses of all its CSRs.
///
/// To create an `SocInfo` for your SoC, you need to use the `--soc-json
/// <filename>` (or `--csr-json <filename>`) flag when generating it to generate
/// a JSON file describing the SoC, then parse it into this struct.
///
/// You can also create the JSON file by passing `csr_json=<filename>` to
/// `Builder` in Python.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocInfo {
    /// A map from the name of every LiteX module that has CSRs to the address
    /// where those CSRs start.
    ///
    /// Those addresses are addresses on the SoC's main bus, which are mapped
    /// directly to addresses in its PCIe BAR 0.
    pub csr_bases: HashMap<String, u32>,
    /// A map from the name of every CSR in the SoC to information about it.
    ///
    /// The name of a CSR is formatted as `<module>_<submodule1>_..._<csr>`,
    /// where:
    /// - `module` is the name of the module the CSR belongs in.
    /// - `submodule<n>` is the name of the `n`th submodule between `module` and
    ///   the module where the CSR's defined.
    /// - `csr_name` is the name of the CSR.
    ///
    /// It seems like the distinction between a module that shows up in
    /// `csr_bases` and a submodule that just gets added to the name is that
    /// modules are created with `self.add_module("name", module)`, and
    /// submodules are created with `self.name = module`; but I'm not an expert
    /// on Migen/LiteX so I'm not entirely sure.
    pub csr_registers: HashMap<String, CsrInfo>,
    /// A map from the names of constants about the SoC to their values.
    ///
    /// `None` is used for boolean constants: not being in the map means
    /// `false`, and `None` means `true`, similar to `#define`s in C.
    pub constants: HashMap<String, Option<SocConstant>>,
    /// A map from the names of regions of the SoC's memory to info about them.
    pub memories: HashMap<String, MemoryRegion>,
}

/// Information about an individual CSR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CsrInfo {
    /// The address of the register.
    ///
    /// Same as [`SocInfo::csr_bases`], these are addresses on the SoC's
    /// main bus.
    pub addr: u32,
    /// The number of `u32`s this CSR takes up.
    ///
    /// TODO: this is actually in units of `config_csr_data_width` (one of the
    /// values in [`SocInfo::constants`]), support that being 8 bits (the only
    /// other valid value).
    pub size: u32,
    /// Whether the CSR is read-only or read-write.
    #[serde(rename = "type")]
    pub kind: CsrKind,
}

/// Whether a CSR is read-only or read-write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CsrKind {
    /// The CSR can only be read from.
    #[serde(rename = "ro")]
    ReadOnly,
    /// The CSR can be both read and written.
    #[serde(rename = "rw")]
    ReadWrite,
}

impl Display for CsrKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            CsrKind::ReadOnly => f.pad("read-only"),
            CsrKind::ReadWrite => f.pad("read-write"),
        }
    }
}

/// The value of a constant about an SoC, which can either be a string or an
/// integer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SocConstant {
    String(String),
    // I picked an `i32` for the type here because these constants are also sometimes returned by
    // getter functions in C, and LiteX uses a C `int` as the return value for those functions
    // when the constant is an integer.
    Integer(i32),
}

/// Information about a region of the SoC's memory.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryRegion {
    /// The address where the memory region starts.
    ///
    /// Same as [`SocInfo::csr_bases`], these are addresses on the SoC's
    /// main bus.
    pub base: u32,
    /// The size of the memory region, in bytes.
    pub size: u32,
    /// What kind of region this is.
    ///
    /// This is either `"cached"`, `"io"`, or either of the two with `+linker`
    /// added onto the end.
    #[serde(rename = "type")]
    pub kind: String,
}

impl SocInfo {
    /// Helper function to retrieve the base address of the SoC's CSR memory
    /// region.
    ///
    /// Needed since the PCIe bridge only exposes the SoC's CSRs, but the
    /// addresses of the CSRs are given as addresses in the SoC's overall memory
    /// space and so need to have the CSR base address subtracted from them.
    pub fn csr_base(&self) -> Result<u32, crate::Error> {
        match self.memories.get("csr") {
            Some(region) => Ok(region.base),
            None => Err(crate::csr::Error::NoCsrRegion),
        }
    }
}
