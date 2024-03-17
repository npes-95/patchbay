use crate::Action;

use anyhow::{anyhow, Result};
use clap::Arg;

use std::ffi::OsString;
use std::io::Write;

pub struct Parser {
    command: clap::Command,
}

impl Parser {
    pub fn new() -> Self {
        // strip out usage
        const PARSER_TEMPLATE: &str = "\
            {all-args}
        ";
        // strip out name/version
        const CMD_TEMPLATE: &str = "\
            {about-with-newline}\n\
            {usage-heading}\n    {usage}\n\
            \n\
            {all-args}{after-help}\
        ";

        Parser {
            command: clap::Command::new("repl")
                .multicall(true)
                .arg_required_else_help(true)
                .subcommand_required(true)
                .subcommand_value_name("CMD")
                .subcommand_help_heading("COMMANDS")
                .help_template(PARSER_TEMPLATE)
                .subcommand(
                    clap::Command::new("list")
                        .alias("ls")
                        .about("List hosts and devices available on system.")
                        .help_template(CMD_TEMPLATE),
                )
                .subcommand(
                    clap::Command::new("host")
                        .arg(Arg::new("name").required(true))
                        .about("Select host.")
                        .help_template(CMD_TEMPLATE),
                )
                .subcommand(
                    clap::Command::new("connect")
                        .alias("c")
                        .alias("con")
                        .alias("conn")
                        .arg(Arg::new("source name").required(true))
                        .arg(Arg::new("source channel").required(true))
                        .arg(Arg::new("sink name").required(true))
                        .arg(Arg::new("sink channel").required(true))
                        .about("Create connection between two channels on a source device and a sink device.")
                        .help_template(CMD_TEMPLATE),
                )
                .subcommand(
                    clap::Command::new("disconnect")
                        .alias("d")
                        .arg(Arg::new("id").required(true))
                        .about("Delete connection.")
                        .help_template(CMD_TEMPLATE),
                )
                .subcommand(
                    clap::Command::new("print")
                        .alias("p")
                        .about("Print patchbay state.")
                        .help_template(CMD_TEMPLATE),
                )
                .subcommand(
                    clap::Command::new("start")
                        .about("Start audio loop.")
                        .help_template(CMD_TEMPLATE),
                )
                .subcommand(
                    clap::Command::new("stop")
                        .about("Stop audio loop.")
                        .help_template(CMD_TEMPLATE),
                )
                .subcommand(
                    clap::Command::new("save")
                        .arg(Arg::new("path").required(true))
                        .about("Save patchbay state to JSON configuration file.")
                        .help_template(CMD_TEMPLATE),
                )
                .subcommand(
                    clap::Command::new("load")
                        .arg(Arg::new("path").required(true))
                        .about("Load patchbay state from JSON configuration file.")
                        .help_template(CMD_TEMPLATE),
                )
                .subcommand(
                    clap::Command::new("quit")
                        .alias("q")
                        .alias("exit")
                        .about("Quit patchbay.")
                        .help_template(CMD_TEMPLATE),
                ),
        }
    }

    pub fn parse<I, T>(&mut self, tokens: I) -> Result<Action>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let matches = self.command.try_get_matches_from_mut(tokens)?;
        match matches.subcommand() {
            Some(("list", _)) => Ok(Action::List),
            Some(("host", sub_matches)) => Ok(Action::Host(
                sub_matches
                    .get_one::<String>("name")
                    .ok_or(anyhow!("Host name missing"))?
                    .to_owned(),
            )),
            Some(("connect", sub_matches)) => Ok(Action::Connect(
                sub_matches
                    .get_one::<String>("source name")
                    .ok_or(anyhow!("Source name missing"))?
                    .to_owned(),
                sub_matches
                    .get_one::<String>("source channel")
                    .ok_or(anyhow!("Source channel missing"))?
                    .to_owned()
                    .parse()?,
                sub_matches
                    .get_one::<String>("sink name")
                    .ok_or(anyhow!("Sink name missing"))?
                    .to_owned(),
                sub_matches
                    .get_one::<String>("sink channel")
                    .ok_or(anyhow!("Sink channel missing"))?
                    .to_owned()
                    .parse()?,
            )),
            Some(("disconnect", sub_matches)) => Ok(Action::Disconnect(
                sub_matches
                    .get_one::<String>("id")
                    .ok_or(anyhow!("Connection id missing"))?
                    .to_owned(),
            )),
            Some(("print", _)) => Ok(Action::Print),
            Some(("start", _)) => Ok(Action::Start),
            Some(("stop", _)) => Ok(Action::Stop),
            Some(("save", sub_matches)) => Ok(Action::Save(
                sub_matches
                    .get_one::<String>("path")
                    .ok_or(anyhow!("Save file path missing"))?
                    .to_owned(),
            )),
            Some(("load", sub_matches)) => Ok(Action::Load(
                sub_matches
                    .get_one::<String>("path")
                    .ok_or(anyhow!("Load file path missing"))?
                    .to_owned(),
            )),
            Some(("quit", _)) => Ok(Action::Quit),
            _ => panic!(),
        }
    }
}

pub fn prompt(
    prefix: &str,
    stdin: &std::io::Stdin,
    stdout: &mut std::io::Stdout,
) -> Result<String> {
    // TODO: handle arrow keys
    // TODO: provide history functionality
    // need to used termion
    let mut buf = String::new();
    print!("{}", prefix);
    stdout.flush()?;
    stdin.read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}

pub fn split_args(input: &str) -> Vec<&str> {
    let mut quoted = false;
    input
        .split(|c: char| {
            if c == '"' || c == '\'' {
                quoted = !quoted;
            }
            !quoted && c.is_whitespace()
        })
        .map(|t| t.trim_matches('"'))
        .map(|t| t.trim_matches('\''))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_action(r: Result<Action>, action: Action) {
        assert!(r.is_ok());
        assert_eq!(r.unwrap(), action);
    }

    #[test]
    fn list() {
        let mut p = Parser::new();
        check_action(p.parse(vec!["list"]), Action::List);
    }

    #[test]
    fn connect() {
        let mut p = Parser::new();
        for alias in ["connect", "c", "con", "conn"] {
            check_action(
                p.parse(vec![alias, "d1", "3", "d2", "2"]),
                Action::Connect("d1".to_string(), 3, "d2".to_string(), 2),
            );
        }
    }

    #[test]
    fn disconnect() {
        let mut p = Parser::new();
        for alias in ["disconnect", "d"] {
            check_action(
                p.parse(vec![alias, "uuid"]),
                Action::Disconnect("uuid".to_string()),
            );
        }
    }

    #[test]
    fn print() {
        let mut p = Parser::new();
        for alias in ["print", "p"] {
            check_action(p.parse(vec![alias]), Action::Print);
        }
    }

    #[test]
    fn start() {
        let mut p = Parser::new();
        check_action(p.parse(vec!["start"]), Action::Start);
    }

    #[test]
    fn stop() {
        let mut p = Parser::new();
        check_action(p.parse(vec!["stop"]), Action::Stop);
    }

    #[test]
    fn save() {
        let mut p = Parser::new();
        check_action(
            p.parse(vec!["save", "foo/bar"]),
            Action::Save("foo/bar".to_string()),
        );
    }

    #[test]
    fn load() {
        let mut p = Parser::new();
        check_action(
            p.parse(vec!["load", "foo/bar"]),
            Action::Load("foo/bar".to_string()),
        );
    }

    #[test]
    fn quit() {
        let mut p = Parser::new();
        for alias in ["quit", "q", "exit"] {
            check_action(p.parse(vec![alias]), Action::Quit);
        }
    }

    #[test]
    fn split() {
        let s =
            "These are whitespace and \"quote delimited\" tokens with \'double and single\' quotes";
        assert_eq!(
            split_args(s),
            [
                "These",
                "are",
                "whitespace",
                "and",
                "quote delimited",
                "tokens",
                "with",
                "double and single",
                "quotes",
            ]
        );
    }
}
