# Wireshark EtherCAT dump analyser

A tool to produce statistics from Wireshark or `tshark` dumps.

## Analysing results

```bash
cargo run --bin egui --release

# OR
cd analyser-gui
cargo run --release
```

This program will load **and monitor** a folder called `./dumps` relative to its crate root (or
binary if running standalone). Put Wireshark `.pcapng` files in that folder and they'll show up in
the GUI for graphing.

## Ignore everything below here

> The Postgres method should still work, but it's not efficient. Use the new custom GUI tool
> instead.

## Result processing

```bash
cargo run --release -- --cycle-ops 6 ./dumps/baseline.pcapng
```

NOTE: `--cycle-ops` is ignored. TODO: Remove arg lol

## Importing data into Postgres

```bash
sudo apt install -y postgresql-client
docker-compose up -d postgres adminer
```

Postgres on port 5432 and adminer on 8080.

Import some data with:

```bash
./ingest ./dumps/baseline.pcapng
```

Import all dumps with

```bash
./process-all.sh
```

# Analysis tools

## Redash

This seems to be the best, tied with Zeppelin. SQL can be written to show latencies and histograms
and stuff.

### Setup

- `dc up -d redash`
- Run [this](https://github.com/getredash/redash/issues/5591#issuecomment-928050601):
  `dc run --rm redash create_db`
- <http://localhost:5000>
- Make a user with generally any old garbage as the user/email/password, as long as you remember it.
- Make a Postgres data source with host/user/pass set to `ethercrab`.

### Example latency comparison

```sql
select
  scenario,
  (delta_time_ns :: float / 1000) as delta_time,
  ROW_NUMBER() OVER (
    partition by scenario
    order by
      packet_number asc
  ) counter
from
  ethercrab
where
  command like '%LRW%'
  and scenario like '%merge_integrated%' --or scenario like '%new4card%'
  or scenario like '%ecat-enc-48-49-i350-0usecs-i7-3770-netbenc%'
  or scenario like 'ecat-enc-48-49-i350-0usecs-tadm-net-ltcy-i7-3770-netbench'
order by
  counter asc
limit
  10000
```

### Example latency histogram

```sql
SELECT
  width_bucket(delta_time_ns, 0, 100000, 200) as bucket,
  count(delta_time_ns),
  int8range(min(delta_time_ns), max(delta_time_ns), '[]') as range,
  scenario,
  min(delta_time_ns) as min,
  max(delta_time_ns) as max
from ethercrab
where scenario like '%merge_integrated%'
GROUP BY bucket, scenario

union

SELECT
  width_bucket(delta_time_ns, 0, 100000, 200) as bucket,
  count(delta_time_ns),
  int8range(min(delta_time_ns), max(delta_time_ns), '[]') as range,
  scenario,
  min(delta_time_ns) as min,
  max(delta_time_ns) as max
from ethercrab
where scenario like '%ecat-enc-48-49-i350-0usecs-i7-3770-netbenc%'
GROUP BY bucket, scenario

union

SELECT
  width_bucket(delta_time_ns, 0, 100000, 200) as bucket,
  count(delta_time_ns),
  int8range(min(delta_time_ns), max(delta_time_ns), '[]') as range,
  scenario,
  min(delta_time_ns) as min,
  max(delta_time_ns) as max
from ethercrab
where scenario like 'ecat-enc-48-49-i350-0usecs-tadm-net-ltcy-i7-3770-netbench'
GROUP BY bucket, scenario
```

If anyone knows a better way of doing multiple overlapping histograms than just a `UNION` I'm all
ears.

## Grafana

Grafana starts up on port 3000 but is rubbish for non-time-series stuff, which is a shame. It's left
in the `docker-compose.yaml` for posterity.

## Apache Zeppelin

- Start it with `dc up zeppelin`. It will fail to start because of permissions errors.
- `sudo chown -R 1000:1000 data/zeppelin`
- `dc up -d` should work now

Zeppelin is a pretty good option for analysis

## Jupyter

There's a Jupyter Lab image in `docker-compose` with the Rust stuff preinstalled. I haven't used
this yet, but it might be useful to get more customisable charts using `plotters`.
