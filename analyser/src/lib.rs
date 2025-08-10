pub mod pdu;

use clap::Parser;
use pcap_file::pcapng::blocks::interface_description::InterfaceDescriptionOption;
use pcap_file::pcapng::blocks::section_header::SectionHeaderOption;
use pcap_file::pcapng::{Block, PcapNgReader};
use pdu::{parse_pdu, Frame};
use serde_with::serde_as;
use serde_with::DurationNanoSeconds;
use smoltcp::wire::{EthernetFrame, EthernetProtocol};
use std::path::Path;
use std::path::PathBuf;
use std::{fs::File, time::Duration};

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

    pub cpu: String,

    pub if_name: String,

    pub timestamp_resolution: u8,

    pub os: String,
}

impl std::fmt::Debug for PcapFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PcapFile")
            .field("packet_number", &self.packet_number)
            .field("scenario", &self.scenario)
            .field("cpu", &self.cpu)
            .field("if_name", &self.if_name)
            .field("timestamp_resolution", &self.timestamp_resolution)
            .field("os", &self.os)
            .finish()
    }
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

        let mut capture_file = PcapNgReader::new(file).expect("Failed to init PCAP reader");

        let section = capture_file.section();

        let cpu = section
            .options
            .iter()
            .find_map(|opt| match opt {
                SectionHeaderOption::Hardware(hw) => Some(hw.to_string()),
                _ => None,
            })
            .unwrap_or_else(|| "(unknown hardware)".to_string());

        let os = section
            .options
            .iter()
            .find_map(|opt| match opt {
                SectionHeaderOption::OS(os) => Some(os.to_string()),
                _ => None,
            })
            .unwrap_or_else(|| "(unknown OS)".to_string());

        let mut if_name = "(unnamed interface)".to_string();
        // Default to nanosecons
        let mut timestamp_resolution = 9;

        while let Some(block) = capture_file.next_block() {
            let block = block.unwrap();

            match block {
                Block::EnhancedPacket(_) => panic!("Encountered packet block before header!"),
                Block::InterfaceDescription(i) => {
                    if let Some(name) = i.options.iter().find_map(|opt| match opt {
                        InterfaceDescriptionOption::IfName(n) => Some(n.to_string()),
                        _ => None,
                    }) {
                        if_name = name.to_string()
                    }

                    if let Some(resolution) = i.options.iter().find_map(|opt| match opt {
                        InterfaceDescriptionOption::IfTsResol(ts) => Some(*ts),
                        _ => None,
                    }) {
                        timestamp_resolution = resolution;
                    }

                    break;
                }
                _ => (),
            }
        }

        let scenario = path.file_stem().unwrap().to_string_lossy().to_string();

        Self {
            capture_file,
            packet_number: 0,
            scenario,
            cpu,
            os,
            if_name,
            timestamp_resolution,
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

            if raw.ethertype() != ETHERCAT_ETHERTYPE {
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

            // TODO: Support multiple PDUs
            let Some(first_pdu) = packet.pdus.first() else {
                continue;
            };

            // Newly sent PDU
            if packet.from_master {
                pairs.push(PduStat {
                    scenario: self.scenario.clone(),
                    packet_number: packet.wireshark_packet_number,
                    index: first_pdu.index,
                    tx_time: packet.time - start_offset,
                    rx_time: Duration::default(),
                    delta_time: Duration::default(),
                    command: first_pdu.command.to_string(),
                });
            }
            // Response to existing sent PDU
            else {
                // Find last sent PDU with this receive PDU's same index
                let Some(sent) = pairs
                    .iter_mut()
                    .rev()
                    .find(|stat| stat.index == first_pdu.index)
                else {
                    // First packet has nothing before it. This will ignore failures for captures
                    // started _during_ an EtherCAT session.
                    if packet.wireshark_packet_number == 1 {
                        continue;
                    }

                    let search_idx = first_pdu.index;

                    let n = 64;

                    let prevs = pairs
                        .iter_mut()
                        .rev()
                        .take(n)
                        .map(|stat| stat.index.to_string())
                        .collect::<Vec<_>>()
                        .join(",");

                    panic!(
                        "Packet number {}: Could not find sent packet, looking for index {}, prev {} indices: {:?}",
                        packet.wireshark_packet_number, search_idx, n, prevs
                    );
                };

                sent.rx_time = packet.time - start_offset;

                sent.delta_time = sent.rx_time - sent.tx_time;
            }
        }

        pairs
    }
}
