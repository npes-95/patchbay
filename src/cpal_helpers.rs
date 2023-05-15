use anyhow::anyhow;
use cpal::traits::{DeviceTrait, HostTrait};

pub fn find_host(host_name: &str) -> anyhow::Result<cpal::Host> {
    let host = if host_name == "default" {
        cpal::default_host()
    } else {
        cpal::host_from_id(
            cpal::available_hosts()
                .into_iter()
                .find(|host| host.name() == host_name)
                .ok_or(anyhow!("unable to find host '{}'", host_name))?,
        )?
    };

    Ok(host)
}

pub fn find_device(host: &impl HostTrait, device_name: &str) -> anyhow::Result<impl DeviceTrait> {
    let device = match device_name {
        "default.in" => host
            .default_input_device()
            .ok_or(anyhow!("unable to get default input device"))?,
        "default.out" => host
            .default_output_device()
            .ok_or(anyhow!("unable to get default output device"))?,
        _ => host
            .output_devices()?
            .find(|device| match device.name() {
                Ok(name) => name == device_name,
                _ => false,
            })
            .ok_or(anyhow!("unable to find device '{}'", device_name))?,
    };

    Ok(device)
}

pub fn find_compatible_stream_configs(
    source_device: &impl DeviceTrait,
    sink_device: &impl DeviceTrait,
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

pub fn print_devices(host: &cpal::Host) -> anyhow::Result<()> {
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
