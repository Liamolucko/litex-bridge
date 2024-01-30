use std::fmt::{self, Debug, Formatter};

use wishbone_bridge::{Bridge, BridgeError};

use crate::{CsrKind, SocInfo};

#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("CSR memory region not found in SocInfo")]
    NoCsrRegion,
    #[error("required CSR `{csr}` not found in SocInfo")]
    MissingCsr { csr: String },
    #[error("expected CSR `{csr}` to be of size {expected}, found {found}")]
    CsrWrongSize {
        csr: String,
        expected: u32,
        found: u32,
    },
    #[error("expected CSR `{csr}` to be {expected}, but it was {found}")]
    CsrWrongKind {
        csr: String,
        expected: CsrKind,
        found: CsrKind,
    },
}

/// A handle to a read-only CSR in the SoC.
pub struct CsrRo<'a, const N: usize = 1> {
    bridge: &'a Bridge,
    offset: u32,
}

impl<'a, const N: usize> CsrRo<'a, N> {
    pub fn read(&self) -> Result<[u32; N], BridgeError> {
        let mut result = [0; N];
        for i in 0..N.try_into().unwrap() {
            result[i] = self
                .bridge
                .peek(self.offset + u32::try_from(4 * i).unwrap())?;
        }
        Ok(result)
    }
}

impl<const N: usize> Debug for CsrRo<'_, N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.read() {
            Ok(x) => x.fmt(f),
            Err(_) => f.pad("(error)"),
        }
    }
}

/// A handle to a read-write CSR in the SoC.
pub struct CsrRw<'a, const N: usize = 1> {
    bridge: &'a Bridge,
    offset: u32,
}

impl<'a, const N: usize> CsrRw<'a, N> {
    pub fn read(&self) -> Result<[u32; N], BridgeError> {
        let mut result = [0; N];
        for i in 0..N.try_into().unwrap() {
            result[i] = self
                .bridge
                .peek(self.offset + u32::try_from(4 * i).unwrap())?;
        }
        Ok(result)
    }

    pub fn write(&self, value: [u32; N]) -> Result<(), BridgeError> {
        for i in 0..N.try_into().unwrap() {
            self.bridge
                .poke(self.offset + u32::try_from(4 * i).unwrap(), value[i])?;
        }
        Ok(())
    }
}

impl<const N: usize> Debug for CsrRw<'_, N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.read() {
            Ok(x) => x.fmt(f),
            Err(_) => f.pad("(error)"),
        }
    }
}

/// A trait for types that are collections of CSRs.
///
/// `CsrRo`, `CsrRw` and types generated by `csr_struct!` implement this.
pub trait CsrGroup<'a> {
    /// The list of CSR addresses this type needs to know before it can be
    /// constructed from a [`Bridge`].
    type Addrs;
    /// Figures out the addresses of all the CSRs in this group from an
    /// `soc_info` and the name of the module that contains them.
    ///
    /// You also have to specify whether the bridge you're going to be using
    /// only exposes the SoC's CSRs. Right now, that's only the case for the
    /// PCIe bridge: all the other bridges expose the entire Wishbone bus.
    fn addrs(soc_info: &SocInfo, csr_only: bool, module: &str) -> Result<Self::Addrs, Error>;
    /// Creates a handle to this group of CSRs from the [`Bridge`] via which
    /// they should be accessed.
    fn backed_by(bridge: &'a Bridge, addrs: Self::Addrs) -> Self;
}

impl<'a, const N: usize> CsrGroup<'a> for CsrRo<'a, N> {
    type Addrs = u32;

    fn addrs(soc_info: &SocInfo, csr_only: bool, module: &str) -> Result<Self::Addrs, Error> {
        // We're down to the individual CSR now, which means there's nothing left to add
        // on to `module`: it's the final name of the CSR.
        let info = soc_info
            .csr_registers
            .get(module)
            .ok_or_else(|| Error::MissingCsr {
                csr: module.to_owned(),
            })?;

        let expected_size = N.try_into().unwrap();
        if info.size != expected_size {
            return Err(Error::CsrWrongSize {
                csr: module.to_owned(),
                expected: expected_size,
                found: info.size,
            });
        }
        if info.kind != CsrKind::ReadOnly {
            return Err(Error::CsrWrongKind {
                csr: module.to_owned(),
                expected: CsrKind::ReadOnly,
                found: info.kind,
            });
        }

        let mut addr = info.addr;
        if csr_only {
            addr -= soc_info.csr_base()?;
        }
        Ok(addr)
    }

    fn backed_by(bridge: &'a Bridge, addrs: Self::Addrs) -> Self {
        Self {
            bridge,
            offset: addrs,
        }
    }
}

impl<'a, const N: usize> CsrGroup<'a> for CsrRw<'a, N> {
    type Addrs = u32;

    fn addrs(soc_info: &SocInfo, csr_only: bool, module: &str) -> Result<Self::Addrs, Error> {
        // We're down to the individual CSR now, which means there's nothing left to add
        // on to `module`: it's the final name of the CSR.
        let info = soc_info
            .csr_registers
            .get(module)
            .ok_or_else(|| Error::MissingCsr {
                csr: module.to_owned(),
            })?;

        let expected_size = N.try_into().unwrap();
        if info.size != expected_size {
            return Err(Error::CsrWrongSize {
                csr: module.to_owned(),
                expected: expected_size,
                found: info.size,
            });
        }
        if info.kind != CsrKind::ReadWrite {
            return Err(Error::CsrWrongKind {
                csr: module.to_owned(),
                expected: CsrKind::ReadWrite,
                found: info.kind,
            });
        }

        let mut addr = info.addr;
        if csr_only {
            addr -= soc_info.csr_base()?;
        }
        Ok(addr)
    }

    fn backed_by(bridge: &'a Bridge, addrs: Self::Addrs) -> Self {
        Self {
            bridge,
            offset: addrs,
        }
    }
}

impl<'a, T: CsrGroup<'a>> CsrGroup<'a> for Option<T> {
    type Addrs = Option<T::Addrs>;

    fn addrs(soc_info: &SocInfo, csr_json: bool, module: &str) -> Result<Option<T::Addrs>, Error> {
        match T::addrs(soc_info, csr_json, module) {
            Ok(addrs) => Ok(Some(addrs)),
            Err(Error::MissingCsr { .. }) => Ok(None),
            Err(other) => Err(other),
        }
    }

    fn backed_by(bridge: &'a Bridge, addrs: Option<T::Addrs>) -> Self {
        addrs.map(|addrs| T::backed_by(bridge, addrs))
    }
}

/// A macro similar to `pci_driver::pci_struct` for defining groups of LiteX
/// CSRs.
///
/// The main difference is that instead of hardcoding the offsets of all the
/// fields, it looks them up from an [`SocInfo`].
#[macro_export]
macro_rules! csr_struct {
    (
        $(
            $(#[$attr:meta])*
            $vis:vis struct $name:ident<$lifetime:lifetime> {
                $(
                    $(#[$field_attr:meta])*
                    $field_name:ident: $field_ty:ty
                ),* $(,)?
            }
        )*
    ) => {
        $(
            $(#[$attr])*
            #[derive($crate::std::clone::Clone, $crate::std::marker::Copy)]
            $vis struct $name<$lifetime, T = &$lifetime $crate::wishbone_bridge::Bridge> {
                bridge: T,
                $($field_name: <$field_ty as $crate::CsrGroup<$lifetime>>::Addrs,)*
            }

            impl<$lifetime> $crate::CsrGroup<$lifetime> for $name<$lifetime> {
                type Addrs = $name<$lifetime, ()>;
                fn addrs(soc_info: &$crate::SocInfo, csr_only: $crate::std::primitive::bool, module: &$crate::std::primitive::str) -> $crate::std::result::Result<Self::Addrs, $crate::Error> {
                    Ok($name {
                        bridge: (),
                        $(
                            $field_name: <$field_ty as $crate::CsrGroup>::addrs(
                                soc_info,
                                csr_only,
                                &$crate::std::format!("{}_{}", module, $crate::std::stringify!($field_name))
                            )?,
                        )*
                    })
                }
                fn backed_by(bridge: &'a $crate::wishbone_bridge::Bridge, addrs: Self::Addrs) -> Self {
                    Self {
                        bridge,
                        $($field_name: addrs.$field_name,)*
                    }
                }
            }

            impl<$lifetime> $name<$lifetime> {
                $(
                    $(#[$field_attr])*
                    $vis fn $field_name(&self) -> $field_ty {
                        <$field_ty as $crate::CsrGroup<$lifetime>>::backed_by(&self.bridge, self.$field_name)
                    }
                )*
            }

            impl<$lifetime> $crate::std::fmt::Debug for $name<$lifetime> {
                fn fmt(&self, f: &mut $crate::std::fmt::Formatter<'_>) -> $crate::std::fmt::Result {
                    f.debug_struct(stringify!($name))
                    $(
                        .field(stringify!($field_name), &self.$field_name())
                    )*
                    .finish()
                }
            }
        )*
    };
}
