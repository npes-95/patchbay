# patchbay

simple routing between audio devices

## usage

patchbay can be run interactively or as a daemon.

### command line arguments

```
patchbay [OPTIONS] [CONFIG PATH]

Arguments:
  [CONFIG PATH] Path to the configuration file (optional)

Options:
  -d
      Run in daemon mode
```

### interactive commands

```
list        List hosts and devices available on system.
host        Select host.
connect     Create connection between two channels on a source device and a sink device.
disconnect  Delete connection.
print       Print patchbay state.
start       Start audio loop.
stop        Stop audio loop.
save        Save patchbay state to JSON configuration file.
load        Load patchbay state from JSON configuration file.
quit        Quit patchbay.
help        Print this message or the help of the given subcommand(s)
```

## configuration

it is recommended to configure patchbay in interactive mode and export the configuration as JSON

```
# sample-config.json
{
  "host": "<host-name>",                    # string
  "connections": {
    "<connection-id>": {                    # uuid
      "host_name": "<host-name>",           # string
      "source_name": "<source-name>",       # string
      "sink_name": "<sink-name>",           # string
      "source_channel": <source-channel>,   # u16
      "sink_channel": <sink-channel>        # u16
    },
    ...
  }
}
```

## install

```
git clone https://github.com/npes-95/patchbay.git
cd patchbay
cargo install --path .
```

## open issues

* dynamic sample rate selection unsupported (limited to 48kHz)
* command history unsupported
* untested on Linux and Windows
