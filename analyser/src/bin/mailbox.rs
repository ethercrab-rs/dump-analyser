//! Recover mailbox transactions from a Wireshark capture.

use clap::Parser;
use clap_num::maybe_hex;
use dump_analyser::{mailbox::MailboxHeader, PcapFile};
use env_logger::Env;
use ethercrab::{Command, Reads, RegisterAddress, Writes};
use ethercrab_wire::EtherCrabWireRead;
use smoltcp::config;
use std::path::PathBuf;

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

    log::info!("Recovering mailbox transactions from {:?}", args.file);

    let mut reader = PcapFile::new(&args.file);

    // DELETEME
    // let mut n = 0;

    while let Some(packet) = reader.next() {
        for pdu in packet.pdus {
            let info = command_register(&pdu.command).zip(command_subdevice_address(&pdu.command));
            // let register = command_register(&pdu.command);

            // Skip packets that aren't what we're looking for
            let Some((register, configured_address)) = info else {
                continue;
            };

            let is_mailbox = register > 0x1000
                && matches!(
                    pdu.command,
                    Command::Read(Reads::Fprd { .. }) | Command::Write(Writes::Fpwr { .. })
                );

            // A rough copy of https://gitlab.com/wireshark/wireshark/-/blob/master/plugins/epan/ethercat/packet-ethercat-datagram.c#L1630
            if !is_mailbox {
                continue;
            }
            // {
            //     println!(
            //         "{:#06x} do something with mailbox at register {:#06x}",
            //         configured_address, register
            //     );
            // }

            let Ok(header) = MailboxHeader::unpack_from_slice(&pdu.data) else {
                continue;
            };

            println!(
                "MainDevice {} {:#06x} mailbox header {:?}",
                if packet.from_master { "-->" } else { "<--" },
                configured_address,
                header
            );
        }
    }

    Ok(())
}
