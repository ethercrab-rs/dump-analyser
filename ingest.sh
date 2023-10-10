#!/bin/bash

set -e

export PGPASSWORD=ethercrab

FILE=$1
SCENARIO=$(basename "${FILE%.*}")
CSV="${FILE%.pcapng}.csv"

echo "Processing $FILE (scenario $SCENARIO, save to $CSV)"

# Process pcap into csv
cargo run --quiet --release -- --cycle-packets 6 $FILE

psql -h localhost -U ethercrab  -c "insert into cycles (scenario) values ('${SCENARIO}') on conflict do nothing;"

# Clean out old data for this scenario
psql \
    -h localhost \
    -U ethercrab  \
    -c "delete from ethercrab where scenario = '${SCENARIO}';"

# Then import it into postgres
psql \
    -h localhost \
    -U ethercrab  \
    -c "\copy ethercrab(scenario, packet_number, index, command, tx_time_ns, rx_time_ns, delta_time_ns) from '${CSV}' delimiter ',' csv header"
