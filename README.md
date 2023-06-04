# patchbay

simple patching for audio devices

## usage

```
patchbay [OPTIONS] [SOURCE] [SINK]

Arguments:
  [SOURCE]  The source audio device to use [default: default.in]
  [SINK]    The sink audio device to use [default: default.out]

Options:
  -c, --config <FILE>
          Custom config file [default: ~/.config/patchbay/patchbay.toml]
      --host <HOST>
          Audio backend to use [default: default]
      --latency <LATENCY>
          Latency between source and sink in milliseconds [default: 1]
      --sample-rate <SAMPLE_RATE>
          Desired sample rate [default: 48000]
  -l, --list
          List available devices and supported configurations
      --source-channels <SOURCE_CHANNELS>...
          Source channels to map (base index 0) [default: 0]
      --sink-channels <SINK_CHANNELS>...
          Sink channels to map (base index 0) [default: 0]
  -h, --help
          Print help
  -V, --version
          Print version
```

## configuration

patchbay looks for a configuration file in `~/.config/patchbay/patchbay.toml`, unless it is specified using the `-c` flag.

example:

```
host_name = "default"
source_name = "default.in"
sink_name = "default.out"
latency = 2.0
sample_rate = 44100
channel_mapping = [[0,0]]
```

## install

```
git clone https://github.com/npes-95/patchbay.git
cd patchbay
cargo install --path .
```
