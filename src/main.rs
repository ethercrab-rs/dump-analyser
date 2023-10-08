mod pdu;

use clap::Parser;
use env_logger::Env;
use ethercrab::{Command, Writes};
use pcap_file::pcapng::{Block, PcapNgReader};
use pdu::{parse_pdu, Frame};
use smoltcp::wire::{EthernetAddress, EthernetFrame, EthernetProtocol};
use std::fs::File;

const MASTER_ADDR: EthernetAddress = EthernetAddress([0x10, 0x10, 0x10, 0x10, 0x10, 0x10]);
const REPLY_ADDR: EthernetAddress = EthernetAddress([0x12, 0x10, 0x10, 0x10, 0x10, 0x10]);
const ETHERCAT_ETHERTYPE_RAW: u16 = 0x88a4;
const ETHERCAT_ETHERTYPE: EthernetProtocol = EthernetProtocol::Unknown(ETHERCAT_ETHERTYPE_RAW);

/// Wireshark EtherCAT dump analyser
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to pcapng file.
    file: String,

    /// Number of PDUs per process data cycle, both requests and responses from the network.
    #[arg(long)]
    cycle_packets: usize,
}

/// A single PDU cycle.
#[derive(Debug, serde::Serialize)]
struct PduStat {
    //
}

fn main() {
    let args = Args::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    log::info!("Analysing {}", args.file);

    let file = File::open(&args.file).expect("Error opening file");
    let capture_file = PcapNgReader::new(file).expect("Failed to init PCAP reader");

    let reader = PcapFile {
        capture_file,
        packet_number: 0,
    };

    // Ignore everything up to the first `LRW`. This is where the process cycle starts.
    let cycle_packets = reader
        .skip_while(|packet| !matches!(packet.command, Command::Write(Writes::Lrw { .. })))
        .collect::<Vec<_>>();

    let p2 = cycle_packets.clone();

    let cycles = p2.chunks(args.cycle_packets);

    log::info!(
        "Found {} cycles with {} req/res pairs in each",
        cycles.len(),
        args.cycle_packets,
    );

    // TODO: Pair up sends and receives ugh

    // Write PDU metadata

    let out_path = args.file.replace(".pcapng", ".csv");

    // TODO: Nice file name
    let mut wtr = csv::Writer::from_path(&out_path).expect("Unable to create writer");

    for packet in cycle_packets {
        wtr.serialize(PduStat {}).expect("Serialize");
    }

    log::info!("Done, wrote {}", out_path);
}

struct PcapFile {
    capture_file: PcapNgReader<File>,

    /// Packet number from Wireshark capture.
    packet_number: usize,
}

impl Iterator for PcapFile {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_line()
    }
}

impl PcapFile {
    fn next_line(&mut self) -> Option<Frame> {
        while let Some(block) = self.capture_file.next_block() {
            self.packet_number += 1;

            // Check if there is no error
            let block = block.expect("Block error");

            let raw = match block {
                Block::EnhancedPacket(block) => {
                    let buf = block.data.to_owned();

                    let buf = buf.iter().copied().collect::<Vec<_>>();

                    EthernetFrame::new_checked(buf).expect("Failed to parse block")
                }
                Block::InterfaceDescription(_) | Block::InterfaceStatistics(_) => continue,
                other => panic!(
                    "Frame {} is not correct type: {:?}",
                    self.packet_number, other
                ),
            };

            if raw.src_addr() != MASTER_ADDR && raw.src_addr() != REPLY_ADDR {
                panic!(
                    "Frame {} does not have EtherCAT address (has {:?} instead)",
                    self.packet_number,
                    raw.src_addr()
                );
            }

            let frame = parse_pdu(raw).expect("Faild to parse frame");

            return Some(frame);
        }

        None
    }

    // fn next_line_is_send(&mut self) -> EthernetFrame<Vec<u8>> {
    //     let next = self.next_line();

    //     assert_eq!(next.src_addr(), Self::MASTER_ADDR);

    //     next
    // }

    // fn next_line_is_reply(&mut self) -> EthernetFrame<Vec<u8>> {
    //     let next = self.next_line();

    //     assert_eq!(next.src_addr(), Self::REPLY_ADDR);

    //     next
    // }
}
