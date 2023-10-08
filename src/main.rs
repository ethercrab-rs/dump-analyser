use clap::Parser;
use env_logger::Env;
use pcap_file::pcapng::{Block, PcapNgReader};
use smoltcp::wire::{EthernetAddress, EthernetFrame};
use std::fs::File;

/// Wireshark EtherCAT dump analyser
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to pcapng file.
    file: String,

    /// Number of PDUs per process data cycle.
    #[arg(long)]
    cycle_ops: usize,
}

fn main() {
    let args = Args::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    log::info!("Analysing {}", args.file);

    let file = File::open(args.file).expect("Error opening file");
    let capture_file = PcapNgReader::new(file).expect("Failed to init PCAP reader");

    let reader = PcapFile {
        capture_file,
        packet_number: 0,
    };

    // Ignore everything up to the first `LRW`. This is where the process cycle starts.
    let packets = reader.skip_while(|packet| {
        // TODO: Open up PDU interface in ethercrab so we can reuse the parser

        todo!()
    });
}

struct PcapFile {
    capture_file: PcapNgReader<File>,

    /// Packet number from Wireshark capture.
    packet_number: usize,
}

impl Iterator for PcapFile {
    type Item = EthernetFrame<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_line()
    }
}

impl PcapFile {
    const MASTER_ADDR: EthernetAddress = EthernetAddress([0x10, 0x10, 0x10, 0x10, 0x10, 0x10]);
    const REPLY_ADDR: EthernetAddress = EthernetAddress([0x12, 0x10, 0x10, 0x10, 0x10, 0x10]);

    fn next_line(&mut self) -> Option<EthernetFrame<Vec<u8>>> {
        while let Some(block) = self.capture_file.next_block() {
            self.packet_number += 1;

            // Check if there is no error
            let block = block.expect("Block error");

            let raw = match block {
                Block::EnhancedPacket(block) => {
                    let buf = block.data.to_owned();

                    let buf2 = buf.iter().copied().collect::<Vec<_>>();

                    EthernetFrame::new_checked(buf2).expect("Failed to parse block")
                }
                Block::InterfaceDescription(_) | Block::InterfaceStatistics(_) => continue,
                other => panic!(
                    "Frame {} is not correct type: {:?}",
                    self.packet_number, other
                ),
            };

            if raw.src_addr() != Self::MASTER_ADDR && raw.src_addr() != Self::REPLY_ADDR {
                panic!(
                    "Frame {} does not have EtherCAT address (has {:?} instead)",
                    self.packet_number,
                    raw.src_addr()
                );
            }

            return Some(raw);
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
