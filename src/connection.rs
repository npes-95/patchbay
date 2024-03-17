use crate::system;

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use ringbuf::HeapRb;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};

use std::fmt;
use std::time::Duration;

const LATENCY: Duration = Duration::from_millis(2);
const SAMPLE_RATE: u32 = 48000;

#[derive(Serialize, Deserialize)]
struct ConnectionMetadata {
    host_name: String,
    source_name: String,
    sink_name: String,
    source_channel: u16,
    sink_channel: u16,
}

pub struct Connection {
    source_stream: cpal::Stream,
    sink_stream: cpal::Stream,
    metadata: ConnectionMetadata,
}

impl Connection {
    pub fn new(
        host_name: String,
        source_name: String,
        sink_name: String,
        source_channel: u16,
        sink_channel: u16,
    ) -> Result<Self> {
        let source_device = system::find_input_device(&host_name, &source_name)?;
        let sink_device = system::find_output_device(&host_name, &sink_name)?;

        let (source_config, sink_config) = Self::find_matching_configs(
            &source_device,
            &sink_device,
            source_channel,
            sink_channel,
        )?;

        let max_channels = std::cmp::max(source_config.channels, sink_config.channels);
        let ringbuf = Self::create_ringbuf(SAMPLE_RATE, &LATENCY, max_channels);
        let (mut producer, mut consumer) = ringbuf.split();

        let source_cb = move |samples: &[f32], _: &cpal::InputCallbackInfo| {
            producer.push_iter(
                &mut samples
                    .iter()
                    .cloned()
                    .skip(source_channel as usize)
                    .step_by(source_config.channels as usize),
            );
        };

        let sink_cb = move |samples: &mut [f32], _: &cpal::OutputCallbackInfo| {
            samples
                .iter_mut()
                .skip(sink_channel as usize)
                .step_by(sink_config.channels as usize)
                .for_each(|sample| *sample = consumer.pop().unwrap_or(0_f32));
        };

        let err_cb = |err: cpal::StreamError| {
            eprintln!("Streaming error: {}", err);
        };

        Ok(Connection {
            source_stream: source_device.build_input_stream(
                &source_config,
                source_cb,
                err_cb,
                None,
            )?,
            sink_stream: sink_device.build_output_stream(&sink_config, sink_cb, err_cb, None)?,
            metadata: ConnectionMetadata {
                host_name,
                source_name,
                source_channel,
                sink_name,
                sink_channel,
            },
        })
    }

    pub fn run(&self) -> Result<()> {
        self.source_stream.play()?;
        self.sink_stream.play()?;
        Ok(())
    }

    pub fn halt(&self) -> Result<()> {
        self.sink_stream.pause()?;
        self.source_stream.pause()?;
        Ok(())
    }

    fn from_metadata(metadata: ConnectionMetadata) -> Result<Self> {
        Self::new(
            metadata.host_name,
            metadata.source_name,
            metadata.sink_name,
            metadata.source_channel,
            metadata.sink_channel,
        )
    }

    fn find_matching_configs(
        source_device: &cpal::Device,
        sink_device: &cpal::Device,
        source_channel: u16,
        sink_channel: u16,
    ) -> Result<(cpal::StreamConfig, cpal::StreamConfig)> {
        let sample_rate = cpal::SampleRate(SAMPLE_RATE);

        // TODO: find common sample rate
        let mut supported_source_configs = source_device
            .supported_input_configs()?
            .filter(|config| config.channels() >= source_channel)
            .filter(|config| config.min_sample_rate() <= sample_rate)
            .filter(|config| config.max_sample_rate() >= sample_rate);

        let mut supported_sink_configs = sink_device
            .supported_output_configs()?
            .filter(|config| config.channels() >= sink_channel)
            .filter(|config| config.min_sample_rate() <= sample_rate)
            .filter(|config| config.max_sample_rate() >= sample_rate);

        let source_config_range = supported_source_configs
            .next()
            .ok_or(anyhow!("Could not find supported source configuration"))?;
        let sink_config_range = supported_sink_configs
            .next()
            .ok_or(anyhow!("Could not find supported sink configuration"))?;

        Ok((
            source_config_range.with_sample_rate(sample_rate).config(),
            sink_config_range.with_sample_rate(sample_rate).config(),
        ))
    }

    fn create_ringbuf(sample_rate: u32, latency: &Duration, max_channels: u16) -> HeapRb<f32> {
        let buffer_size = {
            let latency_frames = (latency.as_secs_f32() / 1.0) * sample_rate as f32;
            latency_frames as usize * max_channels as usize
        };

        HeapRb::<f32>::new(buffer_size * 2)
    }
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}({}) -> {}({}) [{}; {}Hz; {}ms] ",
            self.metadata.source_name,
            self.metadata.source_channel,
            self.metadata.sink_name,
            self.metadata.sink_channel,
            self.metadata.host_name,
            SAMPLE_RATE,
            LATENCY.as_millis()
        )?;
        Ok(())
    }
}

impl Serialize for Connection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.metadata.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Connection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let metadata = ConnectionMetadata::deserialize(deserializer)?;
        Ok(Connection::from_metadata(metadata).map_err(D::Error::custom)?)
    }
}
