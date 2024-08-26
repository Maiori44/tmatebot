use std::time::Instant;
use owo_colors::OwoColorize;
use serenity::all::{CreateButton, CreateMessage, EditMessage, Message};
use phf::{phf_ordered_map, OrderedMap};
use crate::{executable, Executable, ExecutableArg};

impl ExecutableArg for Message {
	fn key(&self) -> String {
		self.content.to_owned()
	}

	fn requester(&self) -> String {
		self.author.id.bright_blue().to_string()
	}
}

pub static COMMANDS: OrderedMap<&str, Executable<Message>> = phf_ordered_map! {
	"help" => executable!(async |ctx, msg| {
		msg.channel_id.say(ctx, format!(
			"List of available commands:```diff\n{}```",
			COMMANDS.keys().map(|key| format!("+ {key}\n")).collect::<String>()
		)).await?;
	}),
	"ping" => executable!(async |ctx, msg| {
		let start = Instant::now();
		let mut pong = msg.channel_id.say(&ctx, "Loading...").await?;
		let elapsed = start.elapsed();
		pong.edit(ctx, EditMessage::new().content(format!("Bot latency: {elapsed:?}"))).await?;
	}),
	"connect" => executable!(async |ctx, msg| {
		msg.channel_id.send_message(
			ctx,
			CreateMessage::new()
				.content("A password is required.")
				.button(CreateButton::new("login").label("Login"))
		).await?;
	}),
};
