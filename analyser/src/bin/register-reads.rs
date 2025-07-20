//! Recover register reads from a given Wireshark capture file.

use clap::Parser;
use clap_num::maybe_hex;
use dump_analyser::PcapFile;
use env_logger::Env;
use ethercrab::{Command, Reads, Writes};
use std::path::PathBuf;

/// Wireshark EtherCAT EEPROM (partial) dump tool.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to pcapng file.
    pub file: PathBuf,

    /// Registers to recover the data for.
    #[clap(long, num_args = 1.., value_delimiter = ',', value_parser=maybe_hex::<u16>)]
    pub registers: Vec<u16>,
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

fn command_subdevice_address(command: &Command) -> Option<u16> {
    match command {
        Command::Nop => None,
        Command::Read(read) => match read {
            Reads::Aprd { address, .. }
            | Reads::Fprd { address, .. }
            | Reads::Brd { address, .. }
            | Reads::Frmw { address, .. } => Some(*address),
            Reads::Lrd { .. } => None,
        },
        Command::Write(write) => match write {
            Writes::Bwr { address, .. }
            | Writes::Apwr { address, .. }
            | Writes::Fpwr { address, .. } => Some(*address),
            Writes::Lwr { .. } | Writes::Lrw { .. } => None,
        },
    }
}

fn main() -> Result<(), ethercrab::error::Error> {
    let args = Args::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    log::info!("Recovering register reads from {:?}", args.file);

    let mut reader = PcapFile::new(&args.file);

    // DELETEME
    let mut n = 0;

    while let Some(packet) = reader.next() {
        // Only print received values
        if packet.from_master {
            continue;
        }

        for pdu in packet.pdus {
            let info = command_register(&pdu.command)
                .filter(|r| args.registers.contains(r))
                .zip(command_subdevice_address(&pdu.command));
            // let register = command_register(&pdu.command);

            // Skip packets that aren't what we're looking for
            let Some((register, configured_address)) = info else {
                continue;
            };

            let pretty = match pdu.data.len() {
                4 => u32::from_le_bytes(pdu.data.as_slice().try_into().unwrap()).to_string(),
                _ => String::new(),
            };

            println!(
                "{:#06x} {:#06x} {:02x?} {}",
                configured_address,
                register,
                pdu.data.as_slice(),
                pretty
            );

            if n > 50 {
                break;
            }

            n += 1;
        }
    }

    Ok(())
}
