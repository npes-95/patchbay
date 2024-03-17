pub mod cli;
pub mod connection;
pub mod patchbay;
pub mod system;

#[derive(Debug, PartialEq)]
pub enum Action {
    List,
    Host(String),
    Connect(String, u16, String, u16),
    Disconnect(String),
    Print,
    Start,
    Stop,
    Save(String),
    Load(String),
    Quit,
}
