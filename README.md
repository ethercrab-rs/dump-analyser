# Wireshark EtherCAT dump analyser

A tool to produce statistics from Wireshark or `tshark` dumps.

## Example

For a capture with 6 sent packets per PDI cycle:

```bash
cargo run --release -- --cycle-ops 6 ./baseline.pcapng
```
