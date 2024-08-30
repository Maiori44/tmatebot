use std::{fmt::Display, time::Instant};
use owo_colors::OwoColorize;
use serenity::all::{ButtonStyle, CreateMessage, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption, EditMessage, Message};
use crate::{
	connections::CONNECTIONS,
	extensions::ChannelIdExt,
	interactions::TIME_UNITS,
	Executable,
	ExecutableArg
};
use phf::{phf_ordered_map, OrderedMap};

macro_rules! executable {
	(async |$ctx:ident, $msg:ident| $code:block) => {
		crate::executable!(async |$ctx, $msg| $code)
	};
	(async |$ctx:ident, $msg:ident, $connections:ident| $code:block) => {
		crate::executable!(async |$ctx, $msg| {
			let $connections = CONNECTIONS.lock().await;
			if $connections.is_empty() {
				$msg.channel_id.say($ctx, "There are currently no connections established.").await?;
			} else $code
		})
	}
}

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
	"list" => executable!(async |ctx, msg, connections| {
		msg.channel_id.say(ctx, format_list(
			"established connections",
			connections.iter().map(|(id, connection)| format!(
				"**`{id}`**: created {} by `{}`, expires {}",
				connection.creation,
				connection.creator,
				connection.timeout,
			))
		)).await?;
	}),
	"close" => executable!(async |ctx, msg, connections| {
		let menu = CreateSelectMenu::new(
			"close menu",
			CreateSelectMenuKind::String {
				options: connections.iter().map(|(id, connection)| {
					CreateSelectMenuOption::new(
						id.to_string(),
						id.to_string()
					).description(format!("Created by {}", connection.creator))
				}).collect()
			}
		).placeholder("Select connections to close").min_values(1).max_values(connections.len() as u8);
		msg.channel_id.send_message(ctx, CreateMessage::new().content("_ _").select_menu(menu)).await?;
	})
};
