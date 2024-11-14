# Wireshark EtherCAT dump analyser

A tool to produce statistics from Wireshark or `tshark` dumps.

## Analysing results

```bash
cargo run --bin egui --release [optional path to dumps folder]

# OR
cd analyser-gui
cargo run --release [optional path to dumps folder]
```

This program will load **and monitor** a folder given as the first argument to the program, or will
default to `./dumps` relative to where it's executed from if no arg is provided. Put Wireshark
`.pcapng` files in that folder and they'll show up in the GUI for graphing.

## Creating partial EEPROM images from Wireshark captures

This program will extract EEPROM traffic out of a Wireshark capture and write it into a binary file
under `./eeprom-dumps`, one for each discovered SubDevice in the capture file.

\*_It is currently in a testing stage and doesn't work that well. YMMV._

```bash
cargo run --bin wireshark-eeprom --release [path to capture file]
```
