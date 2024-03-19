use patchbay::cli;
use patchbay::connection::Connection;
use patchbay::patchbay::Patchbay;
use patchbay::system;
use patchbay::Action;

use anyhow::{anyhow, Result};
use sysinfo::System;
use cpal::traits::{DeviceTrait, HostTrait};
use uuid::Uuid;

use std::env;
use std::io::{Read, Write};
use std::path::Path;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time;

fn list() -> Result<()> {
    for host in system::hosts() {
        if let Ok(host) = host {
            let host_name = host.id().name();
            println!("Devices ({}):", host_name);
            for device in host.devices()? {
                let input_channels = if device.default_input_config().is_ok() {
                    device.default_input_config()?.channels()
                } else {
                    0
                };
                let output_channels = if device.default_output_config().is_ok() {
                    device.default_output_config()?.channels()
                } else {
                    0
                };
                println!(
                    "{} (in: {}, out: {})",
                    device.name()?,
                    input_channels,
                    output_channels
                );
            }
        }
    }
    Ok(())
}

fn set_host(host_name: &str, patchbay: &mut Patchbay) -> Result<()> {
    patchbay.halt()?;
    patchbay.remove_all_connections()?;
    println!("Set host {}", host_name);
    patchbay.set_host(host_name)
}

fn connect(
    source_name: String,
    source_channel: u16,
    sink_name: String,
    sink_channel: u16,
    patchbay: &mut Patchbay,
) -> Result<()> {
    let connection = Connection::new(
        patchbay.host().to_owned(),
        source_name,
        sink_name,
        source_channel,
        sink_channel,
    )?;
    let id = patchbay.add_connection(connection)?;
    println!("Created connection with id {}", id);
    Ok(())
}

fn disconnect(id: &str, patchbay: &mut Patchbay) -> Result<()> {
    if id == "*" {
        patchbay.remove_all_connections()?;
    } else {
        patchbay.remove_connection(&Uuid::parse_str(id)?)?;
    }

    println!("Removed connection {}", id);
    Ok(())
}

fn save(path: &Path, patchbay: &mut Patchbay) -> Result<()> {
    let mut f = std::fs::File::create(path)?;
    f.write_all(serde_json::to_string_pretty(&patchbay)?.as_bytes())?;
    println!("Saved configuration to {:?}", path);
    Ok(())
}

fn load(path: &Path, patchbay: &mut Patchbay) -> Result<()> {
    let mut f = std::fs::File::open(path)?;
    let mut buf = String::new();
    f.read_to_string(&mut buf)?;

    let new = serde_json::from_str(&buf)?;

    patchbay.halt()?;
    patchbay.remove_all_connections()?;
    *patchbay = new;
    patchbay.halt()?;
    println!("Loaded configuration");
    Ok(())
}

fn run_daemon(mut patchbay: Patchbay) -> Result<()> {
    let terminate = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&terminate))?;
    let hundred_millis = time::Duration::from_millis(100);

    patchbay.run()?;

    println!(
        "Started patchbay in non-interactive mode (PID: {})",
        process::id()
    );


    while !terminate.load(Ordering::Relaxed) {
        thread::sleep(hundred_millis);
    }

    patchbay.halt()?;
    Ok(())
}

fn run_repl(mut patchbay: Patchbay) -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut parser = cli::Parser::new();

    loop {
        match cli::prompt("> ", &stdin, &mut stdout) {
            Ok(input) => {
                if input.is_empty() {
                    continue;
                }

                match parser.parse(cli::split_args(&input)) {
                    Ok(action) => {
                        let result = match action {
                            Action::List => list(),
                            Action::Host(host_name) => set_host(&host_name, &mut patchbay),
                            Action::Connect(
                                source_name,
                                source_channel,
                                sink_name,
                                sink_channel,
                            ) => connect(
                                source_name,
                                source_channel,
                                sink_name,
                                sink_channel,
                                &mut patchbay,
                            ),
                            Action::Disconnect(id) => disconnect(&id, &mut patchbay),
                            Action::Print => {
                                print!("{}", patchbay);
                                Ok(())
                            }
                            Action::Start => patchbay.run(),
                            Action::Stop => patchbay.halt(),
                            Action::Save(path) => save(&Path::new(&path), &mut patchbay),
                            Action::Load(path) => load(&Path::new(&path), &mut patchbay),
                            Action::Quit => break,
                        };

                        match result {
                            Ok(_) => continue,
                            Err(e) => eprintln!("{}", e),
                        };
                    }
                    Err(e) => eprintln!("{}", e),
                };
            }
            Err(e) => eprintln!("{}", e),
        };
    }
    Ok(())
}

fn main() -> Result<()> {
    let s = System::new_all();
    for instance in s.processes_by_exact_name("patchbay") {
        if instance.pid().as_u32() != process::id() {
            return Err(anyhow!("Process already started with PID {}", instance.pid()));
        }
    }

    let mut patchbay = Patchbay::new(system::default_host().id().name());

    let args: Vec<String> = env::args().collect();
    let mut daemonize = false;

    for arg in args.iter().skip(1).take(2) {
        if arg == "-d" {
            daemonize = true;
        } else {
            match load(&Path::new(&arg), &mut patchbay) {
                Ok(_) => (),
                Err(e) => {
                    eprintln!("Could not load configuration: {}", e);
                    eprintln!("Continuing with default");
                }
            }
        }
    }

    if daemonize {
        run_daemon(patchbay)
    } else {
        run_repl(patchbay)
    }
}
