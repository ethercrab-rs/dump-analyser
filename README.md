# Wireshark EtherCAT dump analyser

A tool to produce statistics from Wireshark or `tshark` dumps.

## Running the example

```bash
cd concurrent-lrw
just run <interface>
```

## Result processing

```bash
cargo run --release -- --cycle-ops 6 ./dumps/baseline.pcapng
```

NOTE: `--cycle-ops` is ignored. TODO: Remove arg lol

## Importing data into Postgres

> Grafana is rubbish at doing non-time-series stuff which is a shame, so this section is only really
> useful for PG import

```bash
sudo apt install -y postgresql-client
docker-compose up -d
```

Postgres on port 5432 and adminer on 8080.

Import some data with:

```bash
./ingest ./dumps/baseline.pcapng
```

### Grafana

Grafana starts up on port 3000 but is rubbish for non-time-series stuff, which is a shame. It's left
in the `docker-compose.yaml` for posterity.

## Apache Zeppelin

- Start it with `dc up zeppelin`. It will fail to start because of permissions errors.
- `sudo chown -R 1000:1000 data/zeppelin`
- `dc up -d` should work now
