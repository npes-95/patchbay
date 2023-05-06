# patchbay

simple patching for audio devices

## usage

```
patchbay [OPTIONS] [SOURCE] [SINK]

Arguments:
  [SOURCE]  The source audio device to use [default: default]
  [SINK]    The sink audio device to use [default: default]

Options:
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

## install

```
git clone https://github.com/npes-95/patchbay.git
cd patchbay
cargo install --path .
```
