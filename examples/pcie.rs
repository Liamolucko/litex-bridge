//! Prints the identifier of an SoC connected via. PCIe.

use std::ffi::CStr;
use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use litex_bridge::{CsrGroup, CsrRo, SocInfo};
use wishbone_bridge::PCIeBridge;

#[derive(Parser)]
struct Args {
    soc_info: PathBuf,
    device: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let soc_info_json = fs::read_to_string(args.soc_info)?;
    let soc_info: SocInfo = serde_json::from_str(&soc_info_json)?;

    let &ident_addr = soc_info
        .csr_bases
        .get("identifier_mem")
        .context("identifier CSR base not found in SocInfo")?;
    let ident_offset = ident_addr - soc_info.csr_base()?;

    let bridge = PCIeBridge::new(args.device)?.create()?;
    bridge.connect()?;
    let csr = CsrRo::<256>::backed_by(&bridge, ident_offset);
    let bytes = csr.read()?.map(|x| x as u8);
    println!("{}", CStr::from_bytes_until_nul(&bytes)?.to_str()?);

    Ok(())
}
