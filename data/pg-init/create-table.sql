CREATE TABLE "ethercrab" (
  "id" serial NOT NULL,
  "scenario" character(64) NOT NULL,
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

CREATE INDEX "ethercrab_scenario" ON "ethercrab" ("scenario");

CREATE TABLE "cycles" (
  "id" serial NOT NULL,
  PRIMARY KEY ("id"),
  "scenario" character(64) NOT NULL,
  "packets_per_cycle" integer NOT NULL DEFAULT '1'
);

INSERT INTO "cycles" ("id", "scenario", "packets_per_cycle") VALUES
(4,	'ctr1-1way-card.pcapng                                           ',	1),
(5,	'ctr1-encoder00.head.pcapng                                      ',	1),
(6,	'ctr1-encoder00.tail.pcapng                                      ',	1),
(7,	'ctr1-integrated-asus.pcapng                                     ',	1),
(8,	'ctr1-new4card.pcapng                                            ',	1),
(9,	'ctr1-solenoids-merge_integrated-asus.pcapng                     ',	5),
(2,	'ctr0-solenoids.head.pcapng                                      ',	9),
(3,	'ctr0-solenoids.tail.pcapng                                      ',	9),
(10,	'tuned-adm-balanced-no-smt-2.pcapng                              ',	3),
(11,	'tuned-adm-balanced-no-smt-3-partial.pcapng                      ',	3),
(12,	'tuned-adm-balanced-no-smt.pcapng                                ',	3),
(13,	'tuned-adm-balanced.pcapng                                       ',	3),
(14,	'tuned-adm-latency-perf-no-smt-1-partial.pcapng                  ',	3),
(15,	'tuned-adm-latency-perf-no-smt-2-partial.pcapng                  ',	3),
(16,	'tuned-adm-latency-perf-no-smt-3-partial.pcapng                  ',	3),
(17,	'tuned-adm-latency-performance-1.pcapng                          ',	3),
(18,	'tuned-adm-latency-performance-2.pcapng                          ',	3),
(19,	'tuned-adm-latency-performance-3.pcapng                          ',	3),
(20,	'tuned-adm-latency-performance-4.pcapng                          ',	3),
(21,	'tuned-adm-network-latency-2.pcapng                              ',	3),
(22,	'tuned-adm-network-latency-3.pcapng                              ',	3),
(23,	'tuned-adm-network-latency-4.pcapng                              ',	3),
(24,	'tuned-adm-network-latency-68-69-2.pcapng                        ',	3),
(25,	'tuned-adm-network-latency-68-69-3.pcapng                        ',	3),
(26,	'tuned-adm-network-latency-68-69.pcapng                          ',	3),
(27,	'tuned-adm-network-latency.pcapng                                ',	3),
(1,	'baseline.pcapng                                                 ',	3);


ALTER TABLE "cycles"
ADD CONSTRAINT "cycles_scenario" UNIQUE ("scenario");

ALTER TABLE "ethercrab"
ADD FOREIGN KEY ("scenario") REFERENCES "cycles" ("scenario") ON DELETE NO ACTION ON UPDATE NO ACTION
