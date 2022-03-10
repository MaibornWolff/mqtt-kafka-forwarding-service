use argh::FromArgs;
use log::LevelFilter;
use message::BenchmarkMessage;
use simple_logger::SimpleLogger;
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

mod consume;
mod message;
mod produce;
mod setup;

static KAFKA_TOPIC: &str = "stresstest";
static MQTT_TOPIC: &str = "stresstest";

#[derive(FromArgs)]
/// Benchmark tool for the mqtt-kafa-forwarding-service
struct Args {
    #[argh(option)]
    /// number of messages to send
    messages: usize,

    #[argh(option)]
    /// messages per second to send, defaults to unbounded
    msg_rate: Option<u32>,

    #[argh(option)]
    /// qos to use for MQTT publishes, defaults to 1 (At-least-once)
    qos: Option<u8>,

    #[argh(switch, short = 'w')]
    /// is the forwarding-service wrapping messages in json (wrap_as_json: true)
    wrapped_payload: bool,
}

#[tokio::main(worker_threads = 4)]
async fn main() {
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    let args: Args = argh::from_env();
    if let Some(qos) = args.qos {
        if qos > 2 {
            eprintln!("ERROR: QoS must be between 0 and 2");
            return;
        }
    }
    let qos = produce::map_qos(args.qos);

    println!("Running setup steps");
    setup::create_stresstest_topic().await;

    println!("Preparing benchmark run");
    let stop_signal = Arc::new(AtomicBool::new(false));
    let mut prod = produce::BenchmarkProducer::new(stop_signal.clone());
    let mut cons = consume::BenchmarkConsumer::new();
    cons.subscribe().await;
    let receive = tokio::spawn(async move { cons.run(args.wrapped_payload).await });

    println!("Starting benchmark");
    let produce_duration = prod.run(qos, args.messages, args.msg_rate).await;

    println!("Waiting for all messages to be received in kafka");
    let (result,) = tokio::join!(receive);
    let (receive_duration, received_messages) =
        result.expect("Error when waiting for messages to be received");

    print_stats(
        args.messages,
        produce_duration,
        receive_duration,
        received_messages,
    );
    prod.stop().await;
    stop_signal.store(true, Ordering::Relaxed);
}

fn print_stats(
    count: usize,
    publish_duration: Duration,
    receive_duration: Duration,
    messages: Vec<BenchmarkMessage>,
) {
    let received = messages.iter().map(|msg| msg.id).collect::<HashSet<u64>>();
    let expected = (0..count as u64).collect::<HashSet<u64>>();
    let not_received: Vec<&u64> = expected.difference(&received).collect();
    let correct_received = count - not_received.len();

    let receive_time_elapsed = receive_duration.as_millis();
    let msg_per_sec = match receive_time_elapsed {
        0 => 0,
        _ => ((received.len() as u128) * 1000) / receive_time_elapsed,
    };

    if received.len() < count || not_received.len() > 0 {
        println!("WARNING: Not all messages received correctly")
    }
    println!("===========================================================");
    println!("BENCHMARK COMPLETE!");
    println!("===========================================================");
    println!("Published messages:  {}", count);
    println!("Received messages:   {}", received.len());
    println!("Correct messages:    {}", correct_received);
    println!("Publish time:        {}ms", publish_duration.as_millis());
    println!("Receive time:        {}ms", receive_time_elapsed);
    println!("Messages per second: {}", msg_per_sec);
    println!("===========================================================");
}
