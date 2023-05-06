use anyhow::anyhow;
use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ctrlc;
use ringbuf::HeapRb;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(version, about = "Simple patchbay for routing audio between devices.", long_about = None)]
struct Args {
    /// The source audio device to use.
    #[clap(default_value_t = String::from("default"))]
    source: String,

    /// The sink audio device to use.
    #[clap(default_value_t = String::from("default"))]
    sink: String,

    /// Latency between source and sink in milliseconds.
    #[arg(long, default_value_t = 1.0)]
    latency: f32,

    /// Desired sample rate.
    #[arg(long, default_value_t = 48000)]
    sample_rate: u32,

    /// List available devices and supported configurations.
    #[arg(short, long)]
    list: bool,

    /// Source channels to map (base index 0).
    #[arg(long, num_args = 1.., default_values_t = [0])]
    source_channels: Vec<usize>,

    /// Sink channels to map (base index 0).
    #[arg(long, num_args = 1.., default_values_t = [0])]
    sink_channels: Vec<usize>,
}

fn list_devices(host: &cpal::Host) -> anyhow::Result<()> {
    let print_stream_config = |config: &cpal::SupportedStreamConfigRange| {
        println!(
            "max sample rate: {}, min sample rate: {}, channels: {}, sample format: {}",
            config.max_sample_rate().0,
            config.min_sample_rate().0,
            config.channels(),
            config.sample_format()
        );
    };

    println!("Available devices for host {}:", host.id().name());
    println!();
    for device in host.devices()? {
        let input_configs = match device.supported_input_configs() {
            Ok(configs) => configs.collect(),
            Err(_) => vec![],
        };

        let output_configs = match device.supported_output_configs() {
            Ok(configs) => configs.collect(),
            Err(_) => vec![],
        };

        println!("{}", device.name()?);

        if !input_configs.is_empty() {
            println!();
            println!("Supported input configurations:");

            for config in input_configs.iter() {
                print_stream_config(config);
            }
        }

        if !output_configs.is_empty() {
            println!();
            println!("Supported output configurations:");

            for config in output_configs.iter() {
                print_stream_config(config);
            }
        }

        println!();
    }
    Ok(())
}

fn find_devices(
    host: &cpal::Host,
    source: &str,
    sink: &str,
) -> anyhow::Result<(cpal::Device, cpal::Device)> {
    let source_device = if source == "default" {
        host.default_input_device()
    } else {
        host.input_devices()?
            .find(|x| x.name().map(|y| y == source).unwrap_or(false))
    }
    .expect("failed to find source device");

    let sink_device = if sink == "default" {
        host.default_output_device()
    } else {
        host.output_devices()?
            .find(|x| x.name().map(|y| y == sink).unwrap_or(false))
    }
    .expect("failed to find sink device");

    Ok((source_device, sink_device))
}

fn calculate_buffer_size(latency_ms: f32, sample_rate: u32, channels: u16) -> usize {
    // buffer size in samples based on desired latency
    let latency_frames = (latency_ms / 1_000.0) * sample_rate as f32;
    latency_frames as usize * channels as usize
}

fn validate_mapping(
    source_channels: &Vec<usize>,
    sink_channels: &Vec<usize>,
    source_config: &cpal::StreamConfig,
    sink_config: &cpal::StreamConfig,
) -> anyhow::Result<()> {
    if source_channels.len() != sink_channels.len() {
        return Err(anyhow!(
            "channels must be mapped 1:1 (source channels {}, sink channels {})",
            source_channels.len(),
            sink_channels.len()
        ));
    }

    if *source_channels.iter().max().unwrap() + 1 > source_config.channels as usize {
        return Err(anyhow!(
            "source device doesn't support requested mapping (available channels: {})",
            source_config.channels
        ));
    }

    if *sink_channels.iter().max().unwrap() + 1 > sink_config.channels as usize {
        return Err(anyhow!(
            "sink device doesn't support requested mapping (available channels: {})",
            sink_config.channels
        ));
    }

    Ok(())
}

fn select_stream_configs(
    source_device: &cpal::Device,
    sink_device: &cpal::Device,
    sample_rate: u32,
) -> anyhow::Result<(cpal::StreamConfig, cpal::StreamConfig)> {
    // get max channels supported by source/sink
    let source_max_channels = source_device
        .supported_input_configs()?
        .max_by_key(|c| c.channels())
        .ok_or(anyhow!(
            "source device doesn't support any stream configurations"
        ))?
        .channels();

    let sink_max_channels = sink_device
        .supported_output_configs()?
        .max_by_key(|c| c.channels())
        .ok_or(anyhow!(
            "sink device doesn't support any stream configurations"
        ))?
        .channels();

    // get sample rates supported by max channel config
    let mut max_channel_source_config_ranges = source_device
        .supported_input_configs()?
        .filter(|c| c.channels() == source_max_channels);
    let mut max_channel_sink_config_ranges = sink_device
        .supported_output_configs()?
        .filter(|c| c.channels() == sink_max_channels);

    // select config with desired sample rate
    let source_config_range = max_channel_source_config_ranges
        .find(|c| {
            c.min_sample_rate() <= cpal::SampleRate(sample_rate)
                && c.max_sample_rate() >= cpal::SampleRate(sample_rate)
        })
        .ok_or(anyhow!(
            "source device doesn't support requested sample rate"
        ))?;
    let sink_config_range = max_channel_sink_config_ranges
        .find(|c| {
            c.min_sample_rate() <= cpal::SampleRate(sample_rate)
                && c.max_sample_rate() >= cpal::SampleRate(sample_rate)
        })
        .ok_or(anyhow!("sink device doesn't support requested sample rate"))?;

    Ok((
        source_config_range
            .with_sample_rate(cpal::SampleRate(sample_rate))
            .into(),
        sink_config_range
            .with_sample_rate(cpal::SampleRate(sample_rate))
            .into(),
    ))
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let host = cpal::default_host();

    if args.list {
        return list_devices(&host);
    }

    let (source_device, sink_device) = find_devices(&host, &args.source, &args.sink)?;
    let (source_config, sink_config) =
        select_stream_configs(&source_device, &sink_device, args.sample_rate)?;

    validate_mapping(
        &args.source_channels,
        &args.sink_channels,
        &source_config,
        &sink_config,
    )?;

    println!("Mapping:");
    for (source_channel, sink_channel) in args.source_channels.iter().zip(args.sink_channels.iter())
    {
        println!(
            "{} {} --> {} {}",
            source_device.name()?,
            source_channel,
            sink_channel,
            sink_device.name()?
        );
    }

    println!();

    let buffer_size = calculate_buffer_size(
        args.latency,
        args.sample_rate,
        if source_config.channels > sink_config.channels {
            source_config.channels
        } else {
            sink_config.channels
        },
    );

    let rb = HeapRb::<f32>::new(buffer_size * 2);
    let (mut producer, mut consumer) = rb.split();

    for _ in 0..buffer_size {
        producer.push(0.0).unwrap();
    }

    let source_cb = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        let mut overrun = false;
        for source_frame in data.chunks(source_config.channels as usize) {
            for channel in args.source_channels.iter() {
                if producer.push(source_frame[*channel]).is_err() {
                    overrun = true;
                }
            }
        }
        if overrun {
            eprintln!("overrun: try increasing latency",);
        }
    };

    let sink_cb = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        let mut underrun = false;
        for sink_frame in data.chunks_mut(sink_config.channels as usize) {
            for channel in args.sink_channels.iter() {
                let source_channel_sample = match consumer.pop() {
                    Some(s) => s,
                    None => {
                        underrun = true;
                        0_f32
                    }
                };

                sink_frame[*channel] = source_channel_sample;
            }
        }
        if underrun {
            eprintln!("underrun: try increasing latency");
        }
    };

    let err_cb = |err: cpal::StreamError| {
        eprintln!("an error occurred on stream: {}", err);
    };

    println!("Starting audio loop...");

    let playing = Arc::new(AtomicBool::new(true));
    let p = playing.clone();

    ctrlc::set_handler(move || {
        p.store(false, Ordering::SeqCst);
    })?;

    let source_stream =
        source_device.build_input_stream(&source_config, source_cb, err_cb, None)?;
    let sink_stream = sink_device.build_output_stream(&sink_config, sink_cb, err_cb, None)?;

    source_stream.play()?;
    sink_stream.play()?;

    println!("Started.");

    while playing.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    println!("Cleaning up.");

    drop(source_stream);
    drop(sink_stream);

    Ok(())
}
