# Wireshark EtherCAT dump analyser

A tool to produce statistics from Wireshark or `tshark` dumps.

## Example

For a capture with 6 sent packets per PDI cycle:

```bash
cargo run --release -- --cycle-ops 6 ./baseline.pcapng
```

## Importing data into Postgres

> Grafana is rubbish at doing non-time-series stuff which is a shame, so this section is only really
> useful for PG import

```bash
sudo apt install -y postgresql-client
docker-compose up -d
```

Postgres on port 5432 and adminer on 8080.

Import some data with:

```
PGPASSWORD=ethercrab \
psql \
    -h localhost \
    -U ethercrab  \
    -c "\copy ethercrab(scenario, packet_number, index, command, tx_time_ns, rx_time_ns, delta_time_ns) from './baseline.csv' DELIMITER E',' csv header"
```

### Grafana

Grafana starts up on port 3000 but is rubbish for non-time-series stuff, which is a shame. It's left
in the `docker-compose.yaml` for posterity.

## Apache Zeppelin

- Start it with `dc up zeppelin`. It will fail to start because of permissions errors.
- `sudo chown -R 1000:1000 data/zeppelin`
- `dc up -d` should work now
