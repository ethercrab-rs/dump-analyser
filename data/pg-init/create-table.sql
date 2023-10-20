CREATE TABLE "ethercrab" (
  "id" serial NOT NULL,
  "scenario" character varying(64) NOT NULL,
  -- Wireshark packet number
  "packet_number" integer NOT NULL,
  -- EtherCAT PDU index
  "index" smallint NOT NULL,
  "command" character(32) NOT NULL,
  "tx_time_ns" bigint NOT NULL,
  "rx_time_ns" bigint NOT NULL,
  "delta_time_ns" bigint NOT NULL,
  PRIMARY KEY ("id")
);

create database redash;
grant all privileges on database redash to ethercrab;

create database latency;
grant all privileges on database latency to ethercrab;

CREATE INDEX "ethercrab_scenario" ON "ethercrab" ("scenario");

CREATE TABLE "cycles" (
  "id" serial NOT NULL,
  PRIMARY KEY ("id"),
  "scenario" character varying(64) NOT NULL,
  "packets_per_cycle" integer NOT NULL DEFAULT '1'
);

ALTER TABLE "cycles"
ADD CONSTRAINT "cycles_scenario" UNIQUE ("scenario");

ALTER TABLE "ethercrab"
ADD FOREIGN KEY ("scenario") REFERENCES "cycles" ("scenario") ON DELETE NO ACTION ON UPDATE NO ACTION
