mod pdu;

use clap::Parser;
use env_logger::Env;
use ethercrab::{Command, Writes};
use pcap_file::pcapng::{Block, PcapNgReader};
use pdu::{parse_pdu, Frame};
use serde_with::serde_as;
use serde_with::DurationNanoSeconds;
use smoltcp::wire::{EthernetAddress, EthernetFrame, EthernetProtocol};
use std::path::PathBuf;
use std::{fs::File, time::Duration};

const MASTER_ADDR: EthernetAddress = EthernetAddress([0x10, 0x10, 0x10, 0x10, 0x10, 0x10]);
const REPLY_ADDR: EthernetAddress = EthernetAddress([0x12, 0x10, 0x10, 0x10, 0x10, 0x10]);
const ETHERCAT_ETHERTYPE_RAW: u16 = 0x88a4;
const ETHERCAT_ETHERTYPE: EthernetProtocol = EthernetProtocol::Unknown(ETHERCAT_ETHERTYPE_RAW);

/// Wireshark EtherCAT dump analyser
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to pcapng file.
    file: PathBuf,

    /// Number of PDUs per process data cycle, both requests and responses from the network.
    #[arg(long)]
    cycle_packets: usize,
}

/// A single PDU cycle.
#[serde_as]
#[derive(Debug, serde::Serialize)]
struct PduStat {
    scenario: String,

    packet_number: usize,

    index: u8,

    command: String,

    #[serde_as(as = "DurationNanoSeconds")]
    #[serde(rename = "tx_time_ns")]
    tx_time: Duration,

    #[serde_as(as = "DurationNanoSeconds")]
    #[serde(rename = "rx_time_ns")]
    rx_time: Duration,

    #[serde_as(as = "DurationNanoSeconds")]
    #[serde(rename = "delta_time_ns")]
    delta_time: Duration,
}

fn main() {
    let args = Args::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    log::info!("Analysing {:?}", args.file);

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

    let first_packet = cycle_packets.first().expect("Empty dump");

    let start_offset = first_packet.time;

    let p2 = cycle_packets.clone();

    let cycles = p2.chunks(args.cycle_packets);

    log::info!(
        "Found {} cycles with {} req/res pairs in each",
        cycles.len(),
        args.cycle_packets,
    );

    // Pair up sends and receives
    // ---

    let mut pairs = Vec::new();

    for packet in cycle_packets {
        // Newly sent PDU
        if packet.from_master {
            pairs.push(PduStat {
                scenario: args.file.file_name().unwrap().to_string_lossy().to_string(),
                packet_number: packet.wireshark_packet_number,
                index: packet.index,
                tx_time: packet.time - start_offset,
                rx_time: Duration::default(),
                delta_time: Duration::default(),
                command: packet.command.to_string(),
            });
        }
        // Response to existing sent PDU
        else {
            // Find last sent PDU with this receive PDU's same index
            let sent = pairs
                .iter_mut()
                .rev()
                .find(|stat| stat.index == packet.index)
                .expect("Could not find sent packet");

            sent.rx_time = packet.time - start_offset;

            sent.delta_time = sent.rx_time - sent.tx_time;
        }
    }

    // Write PDU metadata to file
    // ---

    let mut out_path = args.file.clone();

    out_path.set_extension("csv");

    let mut wtr = csv::Writer::from_path(&out_path).expect("Unable to create writer");

    for packet in pairs {
        wtr.serialize(packet).expect("Serialize");
    }

    log::info!("Done, wrote {:?}", out_path);
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

            let (raw, timestamp) = match block {
                Block::EnhancedPacket(block) => {
                    let buf = block.data.to_owned();

                    let buf = buf.iter().copied().collect::<Vec<_>>();

                    (
                        EthernetFrame::new_checked(buf).expect("Failed to parse block"),
                        block.timestamp,
                    )
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

            let mut frame = parse_pdu(raw).expect("Faild to parse frame");

            frame.time = timestamp;
            frame.wireshark_packet_number = self.packet_number;

            return Some(frame);
        }

        None
    }
}
