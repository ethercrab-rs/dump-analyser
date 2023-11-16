//! Read EEPROM data from a Wireshark capture.
//!
//! This won't be a full dump - it will only be the segments that were actually read during the
//! capture, but maybe that's enough to aid debugging.

use clap::Parser;
use dump_analyser::PcapFile;
use env_logger::Env;
use ethercrab::{Command, Reads, RegisterAddress, Writes};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

/// Wireshark EtherCAT EEPROM (partial) dump tool.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to pcapng file.
    pub file: PathBuf,
}

fn main() -> Result<(), ethercrab::error::Error> {
    let args = Args::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    log::info!("Reading captured EEPROM data from {:?}", args.file);

    let mut reader = PcapFile::new(&args.file);

    // EEPROM maps for each slave, by address
    let mut slave_maps = HashMap::new();

    /// EEPROM data map for a single device.
    struct SlaveMap {
        /// EEPROM data map.
        data: Vec<u8>,

        /// The address set by the master for reading. Data returned from the slave is written into
        /// [`data`] starting at this address.
        addr: u16,
    }

    while let Some(packet) = reader.next() {
        // EEPROM reader currently only uses FPRD and FPWR so we'll skip anything else.
        let slave_address = match packet.command {
            Command::Read(Reads::Fprd { address, .. })
            | Command::Write(Writes::Fpwr { address, .. }) => address,

            _ => continue,
        };

        let register = packet.command.register().filter(|r| {
            [
                u16::from(RegisterAddress::SiiConfig),
                u16::from(RegisterAddress::SiiControl),
                u16::from(RegisterAddress::SiiAddress),
                u16::from(RegisterAddress::SiiData),
            ]
            .contains(r)
        });

        // Skip packets that aren't EEPROM-related
        let Some(register) = register else { continue };

        // Allocate 65K for each slave's EEPROM. The actual data could be much smaller, but it's
        // small enough to just use the max address (u16::MAX) instead of tracking how big the vec
        // should be.
        let slave_map = slave_maps.entry(slave_address).or_insert(SlaveMap {
            data: vec![0u8; u16::MAX.into()],
            addr: 0,
        });

        // Detect an address set by the master. 6 byte packet is SII control header and 2x u16
        // address (second is ignored).
        if register == u16::from(RegisterAddress::SiiControl)
            && packet.data.len() == 6
            && packet.from_master
        {
            let addr = u16::from_le_bytes(packet.data[2..4].try_into().unwrap());

            // log::info!("{:#06x} Set EEPROM addr to {:#06x}", slave_address, addr);

            slave_map.addr = addr;
        }
        // Response from device
        else if register == u16::from(RegisterAddress::SiiData) && !packet.from_master {
            // Useful for matching with Wireshark prettyprinting
            // let d = packet
            //     .data
            //     .chunks(2)
            //     .map(|chunk| u16::from_le_bytes(chunk.try_into().unwrap()))
            //     .collect::<Vec<_>>();

            let d = packet.data;

            // log::info!(
            //     "{:#06x} EEPROM data at {:#06x} {:#x?}",
            //     slave_address, slave_map.addr, d
            // );

            slave_map.data[usize::from(slave_map.addr)..][..d.len()].copy_from_slice(&d);
        }
    }

    // Now write out each device's EEPROM to a file

    let dir = PathBuf::from("./eeprom-dumps");

    fs::create_dir_all(&dir).expect("Could not create dumps dir");

    for (addr, eeprom) in slave_maps {
        let mut filename = dir.clone();
        filename.push(format!("eeprom-{:#06x}.hex", addr));

        log::info!("Write {}", filename.display());

        let mut f = File::create(&filename).expect("Could not open file for writing");

        f.write_all(&eeprom.data)
            .expect("Failed to write EEPROM dump data");
    }

    Ok(())
}
