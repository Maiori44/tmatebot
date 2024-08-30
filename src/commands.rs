use std::{fmt::Display, time::Instant};
use owo_colors::OwoColorize;
use serenity::all::{ButtonStyle, EditMessage, Message};
use crate::{connections::CONNECTIONS, executable, extensions::ChannelIdExt, interactions::TIME_UNITS, Executable, ExecutableArg};
use phf::{phf_ordered_map, OrderedMap};

impl ExecutableArg for Message {
	fn key(&self) -> String {
		self.content.to_lowercase()
	}

	fn requester(&self) -> String {
		self.author.id.bright_blue().to_string()
	}
}

fn format_list(name: &str, iter: impl Iterator<Item = impl Display>) -> String {
	let mut result = String::with_capacity(128) + "List of " + name + ":\n";
	result.extend(iter.map(|item| format!("- {item}\n")));
	result
}

const PASSWORD_PROMPT: &'static str = "Authorization is required.";

pub static COMMANDS: OrderedMap<&str, Executable<Message>> = phf_ordered_map! {
	"help" => executable!(async |ctx, msg| {
		msg.channel_id.say(ctx, format_list(
			"available commands",
			COMMANDS.keys().map(|command| format!("`{command}`"))
		)).await?;
	}),
	"ping" => executable!(async |ctx, msg| {
		let start = Instant::now();
		let mut pong = msg.channel_id.say(&ctx, "Loading...").await?;
		let elapsed = start.elapsed();
		pong.edit(ctx, EditMessage::new().content(format!("Bot latency: {elapsed:?}"))).await?;
	}),
	"units" => executable!(async |ctx, msg| {
		msg.channel_id.say(ctx, format_list(
			"valid time units",
			TIME_UNITS.entries().map(|(unit, (_, name))| format!("**`{}`**: `{name}`", *unit as char))
		)).await?;
	}),
	"connect" => executable!(async |ctx, msg| {
		msg.channel_id.send_button(ctx, PASSWORD_PROMPT, "Login", ButtonStyle::Primary).await?;
	}),
	"password" => executable!(async |ctx, msg| {
		msg.channel_id.send_button(ctx, PASSWORD_PROMPT, "Register", ButtonStyle::Primary).await?;
	}),
	"list" => executable!(async |ctx, msg| {
		let connections = CONNECTIONS.lock().await;
		if connections.is_empty() {
			msg.channel_id.say(ctx, "There are currently no connections established.").await?;
		} else {
			msg.channel_id.say(ctx, format_list(
				"established connections",
				connections.iter().map(|(id, connection)| format!(
					"**`{id}`**: created {}, expires {}",
					connection.creation,
					connection.timeout,
				))
			)).await?;
		}
	})
};
