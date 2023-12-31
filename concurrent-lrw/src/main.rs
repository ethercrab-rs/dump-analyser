//! Demonstrate sorting slaves into multiple slave groups.
//!
//! This demo is designed to be used with the following slave devices:
//!
//! - EK1100 (or EK1501 if using fibre)
//! - EL2889 (2 bytes of outputs)
//! - EL2828 (1 byte of outputs)

use ethercrab::{
    error::Error, slave_group::PreOp, Client, ClientConfig, Command, PduStorage, RegisterAddress,
    SlaveGroup, SlaveState, Timeouts,
};
use smol::stream::StreamExt;
use std::{sync::Arc, time::Duration};
use thread_priority::{
    set_thread_priority_and_policy, thread_native_id, RealtimeThreadSchedulePolicy, ThreadPriority,
    ThreadPriorityValue, ThreadSchedulePolicy,
};

const MAIN_THREAD_PRIO: u8 = 90u8;
const NET_THREAD_PRIO: u8 = 91u8;

/// Maximum number of slaves that can be stored. This must be a power of 2 greater than 1.
const MAX_SLAVES: usize = 16;
/// Maximum PDU data payload size - set this to the max PDI size or higher.
const MAX_PDU_DATA: usize = 1100;
/// Maximum number of EtherCAT frames that can be in flight at any one time.
const MAX_FRAMES: usize = 16;

static PDU_STORAGE: PduStorage<MAX_FRAMES, MAX_PDU_DATA> = PduStorage::new();

type Slow<S = PreOp> = SlaveGroup<4, 8, S>;
type Fast<S = PreOp> = SlaveGroup<3, 5, S>;

#[derive(Default)]
struct Groups {
    slow_outputs: Slow,
    fast_outputs: Fast,
}

fn main() -> Result<(), Error> {
    let interface = std::env::args()
        .nth(1)
        .expect("Provide network interface as first argument.");

    let client = spawn_ethercat_master(&interface);

    let thread_id = thread_native_id();
    set_thread_priority_and_policy(
        thread_id,
        ThreadPriority::Crossplatform(ThreadPriorityValue::try_from(MAIN_THREAD_PRIO).unwrap()),
        ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Fifo),
    )
    .expect("could not set thread priority. Are the PREEMPT_RT patches in use?");

    let client = Arc::new(client);

    smol::block_on(async {
        // Read configurations from slave EEPROMs and configure devices.
        let groups = client
            .init::<MAX_SLAVES, _>(|groups: &Groups, slave| match slave.name() {
                "EL2889" | "EK1100" | "EK1501" | "EL2008" => Ok(&groups.slow_outputs),
                "EL2828" => Ok(&groups.fast_outputs),
                _ => Err(Error::UnknownSlave),
            })
            .await
            .expect("Init");

        let Groups {
            slow_outputs,
            fast_outputs,
        } = groups;

        let mut fast_outputs = fast_outputs.into_op(&client).await.expect("PRE-OP -> OP");
        let mut slow_outputs = slow_outputs.into_op(&client).await.expect("PRE-OP -> OP");

        let mut tick = smol::Timer::interval(Duration::from_micros(1000));

        let limit = 5000;
        let mut count = 0;

        while let Some(_) = tick.next().await {
            smol::future::zip(
                slow_tick(&mut slow_outputs, &client),
                fast_tick(&mut fast_outputs, &client),
            )
            .await;

            count += 1;

            if count >= limit {
                break;
            }
        }
    });

    Ok(())
}

fn spawn_ethercat_master(ethercat_nic: &str) -> Client<'static> {
    let (tx, rx, pdu_loop) = PDU_STORAGE.try_split().expect("Split");

    let client = Client::new(
        pdu_loop,
        Timeouts {
            state_transition: Duration::from_secs(5),
            // Same timeout as EL2008 watchdog
            pdu: Duration::from_millis(100),
            ..Default::default()
        },
        ClientConfig::default(),
    );

    let tx_rx_task = ethercrab::std::tx_rx_task(ethercat_nic, tx, rx).expect("Spawn");

    // Spawn EtherCrab network TX/RX task in the background
    std::thread::spawn(move || {
        let thread_id = thread_native_id();
        set_thread_priority_and_policy(
            thread_id,
            ThreadPriority::Crossplatform(ThreadPriorityValue::try_from(NET_THREAD_PRIO).unwrap()),
            ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Fifo),
        )
        .expect("could not set thread priority. Are the PREEMPT_RT patches in use?");

        smol::block_on(tx_rx_task).expect("TX/RX task");
    });

    client
}

async fn slow_tick(slow_outputs: &mut Slow<ethercrab::slave_group::Op>, client: &Arc<Client<'_>>) {
    slow_outputs.tx_rx(client).await.expect("TX/RX");

    for slave in slow_outputs.iter(&client) {
        let (_i, o) = slave.io_raw();

        for byte in o.iter_mut() {
            *byte = byte.wrapping_sub(1);
        }

        let _ = Command::fprd(slave.configured_address(), RegisterAddress::AlStatus.into())
            .receive::<SlaveState>(&client)
            .await;
    }
}

async fn fast_tick(fast_outputs: &mut Fast<ethercrab::slave_group::Op>, client: &Arc<Client<'_>>) {
    fast_outputs.tx_rx(client).await.expect("TX/RX");

    for slave in fast_outputs.iter(client) {
        let (_i, o) = slave.io_raw();

        for byte in o.iter_mut() {
            *byte = byte.wrapping_add(1);
        }
    }
}
