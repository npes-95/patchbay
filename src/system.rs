use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{self, HostUnavailable};

pub fn hosts() -> impl Iterator<Item = Result<cpal::Host, HostUnavailable>> {
    cpal::available_hosts()
        .into_iter()
        .map(|id| cpal::host_from_id(id))
}

pub fn default_host() -> cpal::Host {
    cpal::default_host()
}

pub fn find_host(name: &str) -> Result<cpal::Host> {
    Ok(cpal::host_from_id(
        cpal::available_hosts()
            .into_iter()
            .find(|host| host.name() == name)
            .ok_or(anyhow!("Could not find host '{}'", name))?,
    )?)
}

pub fn find_input_device(host_name: &str, device_name: &str) -> Result<cpal::Device> {
    find_host(host_name)?
        .input_devices()?
        .find(|device| match device.name() {
            Ok(name) => name == device_name,
            _ => false,
        })
        .ok_or(anyhow!("Could not find input device '{}'", device_name))
}

pub fn find_output_device(host_name: &str, device_name: &str) -> Result<cpal::Device> {
    find_host(host_name)?
        .output_devices()?
        .find(|device| match device.name() {
            Ok(name) => name == device_name,
            _ => false,
        })
        .ok_or(anyhow!("Could not find output device '{}'", device_name))
}
