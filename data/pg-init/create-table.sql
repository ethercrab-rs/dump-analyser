CREATE TABLE "ethercrab" (
  "id" serial NOT NULL,
  "scenario" character(64) NOT NULL,
  "packet_number" integer NOT NULL,
  "index" smallint NOT NULL,
  "command" character(32) NOT NULL,
  "tx_time_ns" bigint NOT NULL,
  "rx_time_ns" bigint NOT NULL,
  "delta_time_ns" bigint NOT NULL,
  PRIMARY KEY ("id")
);
