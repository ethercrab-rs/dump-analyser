# Wireshark EtherCAT dump analyser

A tool to produce statistics from Wireshark or `tshark` dumps.

## Example

For a capture with 6 sent packets per PDI cycle:

```bash
cargo run --release -- --cycle-ops 6 ./baseline.pcapng
```

## Analysing with Postgres/Grafana

```bash
sudo apt install -y postgresql-client
docker-compose up -d
```

Grafana is now on port 3000, postgres on port 5432 and adminer on 8080.

Import some data with:

```
PGPASSWORD=ethercrab \
psql \
    -h localhost \
    -U ethercrab  \
    -c "\copy ethercrab(scenario, packet_number, index, command, tx_time_ns, rx_time_ns, delta_time_ns) from './baseline.csv' DELIMITER E',' csv header"
```
