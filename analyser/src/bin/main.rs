use clap::Parser;
use dump_analyser::*;
use env_logger::Env;
use std::time::Duration;

fn main() {
    let args = Args::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    log::info!("Analysing {:?}", args.file);

    let reader = PcapFile::new(&args.file);

    // Ignore everything up to the first `LRW`. This is where the process cycle starts.
    let cycle_packets = reader
        // .skip_while(|packet| !matches!(packet.command, Command::Write(Writes::Lrw { .. })))
        .collect::<Vec<_>>();

    let first_packet = cycle_packets.first().expect("Empty dump");

    let start_offset = first_packet.time;

    // let p2 = cycle_packets.clone();

    // let cycles = p2.chunks(args.cycle_packets);

    log::info!("Found {} cycle packets", cycle_packets.len());

    // Pair up sends and receives
    // ---

    let mut pairs = Vec::new();

    for packet in cycle_packets {
        // Newly sent PDU
        if packet.from_master {
            pairs.push(PduStat {
                scenario: args.file.file_stem().unwrap().to_string_lossy().to_string(),
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
