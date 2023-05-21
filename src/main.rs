use patchbay::cpal_helpers;
use patchbay::patchbay::{ChannelCount, Config, Latency, Patchbay, SampleRate};

use clap::Parser;
use ctrlc;

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(version, about = "Simple patchbay for routing audio between devices.", long_about = None)]
struct Args {
    /// Custom config file.
    #[arg(
        short,
        long,
        value_name = "FILE",
        default_value_t = String::from("~/.config/patchbay/patchbay.toml")
    )]
    config: String,

    /// The source audio device to use.
    #[arg(default_value_t = String::from("default.in"))]
    source: String,

    /// The sink audio device to use.
    #[arg(default_value_t = String::from("default.out"))]
    sink: String,

    /// Audio backend to use.
    #[arg(long, default_value_t = String::from("default"))]
    host: String,

    /// Latency between source and sink in milliseconds.
    #[arg(long, default_value_t = 1.0)]
    latency: Latency,

    /// Desired sample rate.
    #[arg(long, default_value_t = 48000)]
    sample_rate: SampleRate,

    /// List available devices and supported configurations.
    #[arg(short, long)]
    list: bool,

    /// Source channels to map (base index 0).
    #[arg(long, num_args = 1.., default_values_t = [0])]
    source_channels: Vec<ChannelCount>,

    /// Sink channels to map (base index 0).
    #[arg(long, num_args = 1.., default_values_t = [0])]
    sink_channels: Vec<ChannelCount>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.list {
        return cpal_helpers::print_devices(&cpal_helpers::find_host(&args.host)?);
    }

    let cli_config_path = Path::new(&args.config);

    let config = if cli_config_path.exists() {
        let mut f = File::open(cli_config_path)?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        let t = s.parse::<toml::Table>()?;
        t.try_into()?
    } else {
        Config {
            host_name: args.host,
            source_name: args.source,
            sink_name: args.sink,
            latency: args.latency,
            sample_rate: args.sample_rate,
            channel_mapping: args
                .source_channels
                .into_iter()
                .zip(args.sink_channels.into_iter())
                .collect(),
        }
    };

    let p = Patchbay::new(config)?;

    println!("Starting audio loop...");

    let should_play = Arc::new(AtomicBool::new(true));
    let should_play_clone = should_play.clone();

    ctrlc::set_handler(move || {
        should_play_clone.store(false, Ordering::SeqCst);
    })?;

    p.start()?;

    println!("Started.");

    while should_play.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    println!();
    println!("Cleaning up.");

    Ok(())
}
