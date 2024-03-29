pub mod pdu;

use clap::Parser;
use pcap_file::pcapng::{Block, PcapNgReader};
use pdu::{parse_pdu, Frame};
use serde_with::serde_as;
use serde_with::DurationNanoSeconds;
use smoltcp::wire::{EthernetAddress, EthernetFrame, EthernetProtocol};
use std::path::Path;
use std::path::PathBuf;
use std::{fs::File, time::Duration};

const MASTER_ADDR: EthernetAddress = EthernetAddress([0x10, 0x10, 0x10, 0x10, 0x10, 0x10]);
const REPLY_ADDR: EthernetAddress = EthernetAddress([0x12, 0x10, 0x10, 0x10, 0x10, 0x10]);
const ETHERCAT_ETHERTYPE_RAW: u16 = 0x88a4;
const ETHERCAT_ETHERTYPE: EthernetProtocol = EthernetProtocol::Unknown(ETHERCAT_ETHERTYPE_RAW);

/// Wireshark EtherCAT dump analyser
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to pcapng file.
    pub file: PathBuf,

    /// Number of PDUs per process data cycle, both requests and responses from the network.
    #[arg(long)]
    pub cycle_packets: usize,
}

/// A single PDU cycle, also a single CSV row.
#[serde_as]
#[derive(Debug, serde::Serialize)]
pub struct PduStat {
    pub scenario: String,

    /// Wireshark packet number.
    pub packet_number: usize,

    /// EtherCAT PDU index.
    pub index: u8,

    pub command: String,

    #[serde_as(as = "DurationNanoSeconds")]
    #[serde(rename = "tx_time_ns")]
    pub tx_time: Duration,

    #[serde_as(as = "DurationNanoSeconds")]
    #[serde(rename = "rx_time_ns")]
    pub rx_time: Duration,

    #[serde_as(as = "DurationNanoSeconds")]
    #[serde(rename = "delta_time_ns")]
    pub delta_time: Duration,
}

pub struct PcapFile {
    pub capture_file: PcapNgReader<File>,

    /// Packet number from Wireshark capture.
    pub packet_number: usize,

    pub scenario: String,
}

impl Iterator for PcapFile {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_line()
    }
}

impl PcapFile {
    pub fn new(path: &Path) -> Self {
        let file = File::open(&path)
            .map_err(|e| {
                log::error!("Failed to open PCAP file {}: {}", path.display(), e);

                e
            })
            .expect("Error opening file");
        let capture_file = PcapNgReader::new(file).expect("Failed to init PCAP reader");

        Self {
            capture_file,
            packet_number: 0,
            scenario: path.file_stem().unwrap().to_string_lossy().to_string(),
        }
    }

    pub fn next_line(&mut self) -> Option<Frame> {
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
                continue;
            }

            let mut frame = parse_pdu(raw).expect("Faild to parse frame");

            frame.time = timestamp;
            frame.wireshark_packet_number = self.packet_number;

            return Some(frame);
        }

        None
    }

    pub fn match_tx_rx(&mut self) -> Vec<PduStat> {
        let mut start_offset = None;

        let mut pairs = Vec::new();

        while let Some(packet) = self.next_line() {
            let start_offset = *start_offset.get_or_insert(packet.time);

            // Newly sent PDU
            if packet.from_master {
                pairs.push(PduStat {
                    scenario: self.scenario.clone(),
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

        pairs
    }
}
