#!/bin/bash

export PGPASSWORD=ethercrab

# Process pcaps into csv
for f in dumps/*.pcapng; do
    cargo run --release -- --cycle-packets 6 $f
done

psql \
    -h localhost \
    -U ethercrab  \
    -c "truncate ethercrab"

# Then import them into postgres
for f in dumps/*.csv; do
    psql \
        -h localhost \
        -U ethercrab  \
        -c "\copy ethercrab(scenario, packet_number, index, command, tx_time_ns, rx_time_ns, delta_time_ns) from '$f' DELIMITER E',' csv header"
done

psql -h localhost -U ethercrab  -c "insert into cycles (scenario) select distinct scenario from ethercrab on duplicate key ignore;"
