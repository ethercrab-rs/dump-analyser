#!/bin/bash

export RUST_LOG=''

for f in dumps/*.pcapng; do
    ./ingest.sh $f
done
