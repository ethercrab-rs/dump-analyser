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

fn command_register(command: &Command) -> Option<u16> {
    match command {
        Command::Nop => None,
        Command::Read(read) => match read {
            Reads::Aprd { register, .. }
            | Reads::Fprd { register, .. }
            | Reads::Brd { register, .. }
            | Reads::Frmw { register, .. } => Some(*register),
            Reads::Lrd { .. } => None,
        },
        Command::Write(write) => match write {
            Writes::Bwr { register, .. }
            | Writes::Apwr { register, .. }
            | Writes::Fpwr { register, .. } => Some(*register),
            Writes::Lwr { .. } | Writes::Lrw { .. } => None,
        },
    }
}

fn main() -> Result<(), ethercrab::error::Error> {
    let args = Args::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    log::info!("Reading captured EEPROM data from {:?}", args.file);

    let mut reader = PcapFile::new(&args.file);

    // EEPROM maps for each slave, by address
    let mut eeprom_images = HashMap::new();

    /// EEPROM data map for a single device.
    struct SubDeviceImage {
        /// EEPROM data map.
        data: Vec<u8>,

        /// The address set by the master for reading. Data returned from the slave is written into
        /// [`data`] starting at this address.
        eeprom_addr: u16,
    }

    log::info!("{:?}", reader);

    let mut addr = 0;

    while let Some(packet) = reader.next() {
        // EEPROM reader currently only uses FPRD and FPWR so we'll skip anything else.
        let slave_address = match packet.command {
            Command::Read(Reads::Fprd { address, .. })
            | Command::Write(Writes::Fpwr { address, .. }) => address,

            _ => continue,
        };

        let register = command_register(&packet.command).filter(|r| {
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
        let eeprom_image = eeprom_images
            .entry(slave_address)
            .or_insert(SubDeviceImage {
                data: vec![0u8; usize::from(u16::MAX) * 2],
                eeprom_addr: 0,
            });

        // Detect an address set by the master. 6 byte packet is SII control header and 2x u16
        // address (second is ignored).
        if register == u16::from(RegisterAddress::SiiControl)
            && packet.data.len() == 6
            && packet.from_master
        {
            let eeprom_addr = u16::from_le_bytes(packet.data[2..4].try_into().unwrap());

            log::debug!(
                "{:#06x} Set EEPROM addr to {:#06x}",
                slave_address,
                eeprom_addr
            );

            addr = eeprom_addr;
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

            log::debug!(
                "{:#06x} EEPROM data at {:#06x} {:02x?} {:?}",
                slave_address,
                addr,
                d,
                d.iter()
                    .map(|byte| char::from_u32(u32::from(*byte))
                        .filter(|c| c.is_alphanumeric() || c.is_ascii_punctuation())
                        .unwrap_or('.'))
                    .collect::<String>()
            );

            eeprom_image.data[usize::from(eeprom_image.eeprom_addr)..][..d.len()]
                .copy_from_slice(&d);

            let Some(next_addr) = addr.checked_mul(2) else {
                log::warn!("Invalid EEPROM address {:#06x}", u32::from(addr) * 2);

                break;
            };

            // Turn EEPROM word addressing into bytes
            eeprom_image.eeprom_addr = next_addr;
        }
    }

    // Now write out each device's EEPROM to a file

    // let dir = PathBuf::from("./eeprom-dumps");
    let dir = args.file.parent().unwrap().to_path_buf();
    let base_file_name = args.file.file_stem().unwrap().to_string_lossy();

    fs::create_dir_all(&dir).expect("Could not create dumps dir");

    for (addr, eeprom) in eeprom_images {
        let mut filename = dir.clone();
        filename.push(format!("{}-eeprom-{:#06x}.hex", base_file_name, addr));

        log::info!("Write {}", filename.display());

        let mut f = File::create(&filename).expect("Could not open file for writing");

        f.write_all(&eeprom.data)
            .expect("Failed to write EEPROM dump data");
    }

    Ok(())
}
