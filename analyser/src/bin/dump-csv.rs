use clap::Parser;
use dump_analyser::*;
use env_logger::Env;

fn main() {
    let args = Args::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    log::info!("Analysing {:?}", args.file);

    let mut reader = PcapFile::new(&args.file);

    let pairs = reader.match_tx_rx();

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
