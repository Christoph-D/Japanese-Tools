use irc::{
    client::{ClientStream, prelude::*},
    error,
};
use rand::{Rng, distr::Alphanumeric, rng};
use std::collections::HashMap;
use std::io::Write;

pub trait ClientInterface {
    fn send(&mut self, command: Command) -> error::Result<()>;
    fn stream(&mut self) -> error::Result<ClientStream>;
}

impl ClientInterface for Client {
    fn send(&mut self, command: Command) -> error::Result<()> {
        self::Client::send(self, command)
    }
    fn stream(&mut self) -> error::Result<ClientStream> {
        self::Client::stream(self)
    }
}

pub struct Bot {
    commands: HashMap<&'static str, CommandFn>,
    client: Box<dyn ClientInterface>,
    magic_key: String,
}

type CommandFn = fn(&Bot, &str) -> Response;

enum Response {
    None,
    Quit(Option<String>),
    Reply(String),
    Join(String),
    Part(String),
    Privmsg(String, String),
}

impl Bot {
    pub fn new<T: ClientInterface + 'static>(client: T) -> Self {
        let mut commands: HashMap<&'static str, CommandFn> = HashMap::new();
        commands.insert("version", version_command);
        commands.insert("help", help_command);

        let magic_key: String = rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();

        println!("Today's magic key for admin commands: {}", magic_key);
        std::io::stdout().flush().unwrap();

        Bot {
            commands,
            client: Box::new(client),
            magic_key,
        }
    }

    pub fn stream(&mut self) -> error::Result<ClientStream> {
        self.client.stream()
    }

    pub fn quit(&mut self, message: Option<String>) -> error::Result<()> {
        self.execute_response(Response::Quit(message), "")
    }

    pub fn handle_message(&mut self, message: &Message) -> error::Result<()> {
        match message.command {
            Command::PRIVMSG(ref target, ref m) => {
                let sender = match message.response_target() {
                    Some(sender) => sender,
                    None => return Ok(()),
                };
                self.handle_privmsg(target, m, sender)
            }
            _ => Ok(()),
        }
    }

    pub fn handle_privmsg(
        &mut self,
        target: &str,
        message: &str,
        sender: &str,
    ) -> error::Result<()> {
        self.execute_response(self.generate_response(target, message, sender), sender)
    }

    fn execute_response(&mut self, response: Response, sender: &str) -> error::Result<()> {
        match response {
            Response::None => Ok(()),
            Response::Quit(msg) => self.client.send(Command::QUIT(Some(
                msg.unwrap_or_else(|| "さようなら".to_string()),
            ))),
            Response::Reply(msg) => {
                self.execute_response(Response::Privmsg(sender.to_string(), msg), sender)
            }
            Response::Join(channel) => self.client.send(Command::JOIN(channel, None, None)),
            Response::Part(channel) => self.client.send(Command::PART(channel, None)),
            Response::Privmsg(target, msg) => {
                // Limit maximum number of lines and line length.
                for msg in msg.split("\n").take(4) {
                    self.client
                        .send(Command::PRIVMSG(target.to_string(), limit_length(msg, 410).to_string()))?;
                }
                Ok(())
            }
        }
    }

    fn generate_response(&self, target: &str, message: &str, _sender: &str) -> Response {
        let message = if let Some(msg) = message.strip_prefix('!') {
            msg
        } else if target.starts_with('#') {
            // Channel message without ! prefix
            return Response::None;
        } else {
            // Private message to the bot works without ! prefix
            message
        };

        // Check for magic key commands
        if message.starts_with(&self.magic_key) {
            return self.do_special_command(message[self.magic_key.len()..].trim_start());
        }

        let (command, args) = self.parse_message(message);
        if let Some(command_handler) = self.commands.get(command.as_str()) {
            return command_handler(self, &args);
        }
        Response::None
    }

    fn do_special_command(&self, cmd: &str) -> Response {
        let (command, args) = cmd.split_once(' ').unwrap_or((cmd, ""));
        match command {
            "die" => {
                if !args.is_empty() {
                    Response::Quit(Some(args.to_string()))
                } else {
                    Response::Quit(None)
                }
            }
            "join" => {
                if !args.is_empty() {
                    Response::Join(args.to_string())
                } else {
                    Response::Reply("Missing channel name".to_string())
                }
            }
            "part" => {
                if !args.is_empty() {
                    Response::Part(args.to_string())
                } else {
                    Response::Reply("Missing channel name".to_string())
                }
            }
            "privmsg" => {
                let args_parts: Vec<&str> = args.splitn(2, ' ').collect();
                if args_parts.len() == 2 {
                    Response::Privmsg(args_parts[0].to_string(), args_parts[1].to_string())
                } else {
                    Response::Reply("Usage: privmsg <target> <message>".to_string())
                }
            }
            _ => Response::Reply("Unknown admin command".to_string()),
        }
    }

    fn parse_message(&self, message: &str) -> (String, String) {
        if let Some((cmd, args)) = message.split_once(' ') {
            (cmd.to_string(), args.to_string())
        } else if let Some((cmd, args)) = message.split_once('　') {
            (cmd.to_string(), args.to_string())
        } else {
            (message.to_string(), String::new())
        }
    }
}

fn version_command(_bot: &Bot, _args: &str) -> Response {
    Response::Reply("A very simple bot with 日本語 support.".to_string())
}

fn help_command(bot: &Bot, _args: &str) -> Response {
    let mut command_names: Vec<&str> = bot.commands.keys().copied().collect();
    command_names.sort();
    let mut commands_list: Vec<String> = command_names
        .iter()
        .map(|cmd| format!("!{}", cmd))
        .collect();
    commands_list.sort();
    let commands_list = commands_list.join(", ");
    Response::Reply(format!("Known commands: {}", commands_list))
}

/// Safely limits the length of a unicode string.
fn limit_length(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    for (limit, _) in s.char_indices().rev() {
        if limit <= max_bytes {
            return &s[..limit];
        }
    }
    ""
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limit_length_ascii() {
        assert_eq!(limit_length("hello", 5), "hello");
        assert_eq!(limit_length("hello world", 5), "hello");
        assert_eq!(limit_length("hello", 10), "hello");
        assert_eq!(limit_length("hello", 0), "");
    }

    #[test]
    fn test_limit_length_japanese() {
        assert_eq!(limit_length("こんにちは", 15), "こんにちは"); // 5 chars * 3 bytes = 15
        assert_eq!(limit_length("こんにちは", 12), "こんにち"); // 4 chars * 3 bytes = 12
        assert_eq!(limit_length("こんにちは", 3), "こ"); // 1 char * 3 bytes = 3
        assert_eq!(limit_length("こんにちは", 2), ""); // Not enough for 1 char
        for i in 1..=16 {
            limit_length("こんにちは", i);  // assert no panics
        }
    }

    #[test]
    fn test_limit_length_mixed() {
        assert_eq!(limit_length("hello こんにちは", 10), "hello こ"); // 6 ASCII + 1 Japanese = 9 bytes
    }

    #[test]
    fn test_limit_length_empty_string() {
        assert_eq!(limit_length("", 10), "");
        assert_eq!(limit_length("", 0), "");
    }
}
