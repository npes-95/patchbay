use crate::cpal_helpers;

use anyhow::anyhow;
use cpal::traits::DeviceTrait;
use ringbuf::HeapRb;

/// Audio channel index.
pub type ChannelCount = u16;

/// Latency between source and sink in milliseconds.
pub type Latency = f32;

/// Sample rate in Hz.
pub type SampleRate = u32;

pub struct Config {
    pub source_name: String,
    pub sink_name: String,
    pub latency: Latency,
    pub sample_rate: SampleRate,
    pub channel_mapping: Vec<(ChannelCount, ChannelCount)>, // source channel -> sink channel
}

pub struct Patchbay {
    source_stream: Box<dyn cpal::traits::StreamTrait>,
    sink_stream: Box<dyn cpal::traits::StreamTrait>,
}

impl Patchbay {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let host = cpal::default_host();
        let source = cpal_helpers::find_device(&host, &config.source_name)?;
        let sink = cpal_helpers::find_device(&host, &config.sink_name)?;
        let (source_stream_config, sink_stream_config) =
            cpal_helpers::find_compatible_stream_configs(&source, &sink, config.sample_rate)?;

        for (mapped_source_channel, mapped_sink_channel) in config.channel_mapping.iter() {
            if *mapped_source_channel > source_stream_config.channels {
                return Err(anyhow!(
                    "source device doesn't support requested mapping (available channels: {})",
                    source_stream_config.channels
                ));
            }

            if *mapped_sink_channel > sink_stream_config.channels {
                return Err(anyhow!(
                    "sink device doesn't support requested mapping (available channels: {})",
                    sink_stream_config.channels
                ));
            }
        }

        let buffer_size = {
            let latency_frames = (config.latency / 1_000.0) * config.sample_rate as f32;
            latency_frames as usize
                * std::cmp::max(source_stream_config.channels, sink_stream_config.channels) as usize
        };

        let ring_buffer = HeapRb::<f32>::new(buffer_size * 2);
        let (mut producer, mut consumer) = ring_buffer.split();

        for _ in 0..buffer_size {
            producer.push(0.0).unwrap();
        }

        let (source_channels, sink_channels): (Vec<_>, Vec<_>) =
            config.channel_mapping.into_iter().unzip();

        let source_cb = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut overrun = false;
            for source_frame in data.chunks(source_stream_config.channels as usize) {
                for channel in source_channels.iter() {
                    if producer.push(source_frame[*channel as usize]).is_err() {
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
            for sink_frame in data.chunks_mut(sink_stream_config.channels as usize) {
                for channel in sink_channels.iter() {
                    let source_channel_sample = match consumer.pop() {
                        Some(s) => s,
                        None => {
                            underrun = true;
                            0_f32
                        }
                    };

                    sink_frame[*channel as usize] = source_channel_sample;
                }
            }
            if underrun {
                eprintln!("underrun: try increasing latency");
            }
        };

        let err_cb = |err: cpal::StreamError| {
            eprintln!("an error occurred on stream: {}", err);
        };

        Ok(Self {
            source_stream: Box::new(source.build_input_stream(
                &source_stream_config,
                source_cb,
                err_cb,
                None,
            )?),
            sink_stream: Box::new(sink.build_output_stream(
                &sink_stream_config,
                sink_cb,
                err_cb,
                None,
            )?),
        })
    }

    pub fn start(&self) -> anyhow::Result<()> {
        self.source_stream.play()?;
        self.sink_stream.play()?;
        Ok(())
    }
}

