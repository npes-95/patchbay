use crate::connection::Connection;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use std::collections::HashMap;
use std::fmt;

#[derive(Serialize, Deserialize)]
pub struct Patchbay {
    host: String,
    connections: HashMap<Uuid, Connection>,
    #[serde(skip)]
    running: bool,
}

impl Patchbay {
    pub fn new(host: &str) -> Self {
        Patchbay {
            host: host.to_owned(),
            connections: HashMap::new(),
            running: false,
        }
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn set_host(&mut self, host: &str) -> Result<()> {
        self.host = host.to_string();
        Ok(())
    }

    pub fn add_connection(&mut self, connection: Connection) -> Result<Uuid> {
        // make sure connection is the in the correct state
        // (sometimes audio streams are auto started)
        if self.running {
            connection.run()?;
        } else {
            connection.halt()?;
        }

        let id = Uuid::new_v4();

        self.connections.insert(id, connection);

        Ok(id)
    }

    pub fn remove_connection(&mut self, id: &Uuid) -> Result<()> {
        let c = self
            .connections
            .get(id)
            .ok_or(anyhow!("Connection {} does not exist.", id))?;
        c.halt()?;
        self.connections.remove(id);
        Ok(())
    }

    pub fn remove_all_connections(&mut self) -> Result<()> {
        self.connections
            .iter()
            .try_for_each(|(_, connection)| connection.halt())?;
        self.connections.clear();
        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        self.connections
            .iter()
            .try_for_each(|(_, connection)| connection.run())?;
        self.running = true;
        Ok(())
    }

    pub fn halt(&mut self) -> Result<()> {
        self.connections
            .iter()
            .try_for_each(|(_, connection)| connection.halt())?;
        self.running = false;
        Ok(())
    }
}

impl fmt::Display for Patchbay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Running: {}", self.running)?;
        writeln!(f, "--")?;
        writeln!(f, "Host: {}", self.host)?;
        writeln!(f, "--")?;
        writeln!(f, "Connections:")?;
        for (id, c) in self.connections.iter() {
            writeln!(f, "{}: {}", id, c)?;
        }
        Ok(())
    }
}
