//! Read EEPROM data from a Wireshark capture.
//!
//! This won't be a full dump - it will only be the segments that were actually read during the
//! capture, but maybe that's enough to aid debugging.

use clap::Parser;
use dump_analyser::PcapFile;
use env_logger::Env;
use ethercrab::{Command, RegisterAddress, Writes};
use std::{path::PathBuf, time::Duration};

/// Wireshark EtherCAT EEPROM (partial) dump tool.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to pcapng file.
    pub file: PathBuf,
}

fn main() -> Result<(), ethercrab::error::Error> {
    // Open capture file

    // For each packet

    // Parse into EtherCAT packet, at least up to ADP/ADO so we can group by slave

    // Get slave record from hashmap

    // If packet is EEPROM address

    // Set auto inc address in slave writer

    // If packet is EEPROM data

    // Write slice, make slave writer increment address internally

    let args = Args::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    log::info!("Reading captured EEPROM data from {:?}", args.file);

    let mut reader = PcapFile::new(&args.file);

    while let Some(packet) = reader.next() {
        // dbg!(packet.command);

        let slave_address = u32::from_le_bytes(packet.command.address()) as u16;
        let register = packet.command.register().filter(|r| {
            [
                u16::from(RegisterAddress::SiiConfig),
                u16::from(RegisterAddress::SiiControl),
                u16::from(RegisterAddress::SiiAddress),
                u16::from(RegisterAddress::SiiData),
            ]
            .contains(r)
        });

        if let Some(register) = register {
            // Detect an address set. 6 byte packet is SII control header and 2x u16 address.
            let eeprom_addr = if register == u16::from(RegisterAddress::SiiControl)
                && packet.data.len() == 6
                && packet.from_master
            {
                Some(u16::from_le_bytes(packet.data[2..4].try_into().unwrap()))
            } else {
                None
            };

            let eeprom_data =
                if register == u16::from(RegisterAddress::SiiData) && !packet.from_master {
                    Some(packet.data)
                } else {
                    None
                };

            eeprom_addr
                .map(|addr| println!("{:#06x} Set EEPROM addr to {:#06x}", slave_address, addr));

            eeprom_data.map(|d| println!("{:#06x} EEPROM data {:#06x?}", slave_address, d));
        }

        // dbg!(slave_address, register);
    }

    Ok(())
}
