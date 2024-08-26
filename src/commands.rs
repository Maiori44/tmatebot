use std::{error::Error, str::SplitWhitespace, time::Instant};
use serenity::{
	all::{Context, CreateButton, CreateMessage, EditMessage, Message},
	futures::future::BoxFuture
};
use phf::{phf_ordered_map, OrderedMap};

type Command = for<'a> fn(
	&'a Context,
	&'a Message,
	SplitWhitespace<'a>
) -> BoxFuture<'a, Result<(), Box<dyn Error>>>;

macro_rules! command {
	(async |$ctx:ident, $message:ident| $code:block) => {
		command!(async |$ctx, $message, _args| $code)
	};
	(async |$ctx:ident, $message:ident, $args:ident| $code:block) => {
		|$ctx, $message, $args| {
			Box::pin(async move {
				$code;
				return Ok(());
			})
		}
	}
}

pub static COMMANDS: OrderedMap<&str, Command> = phf_ordered_map! {
	"help" => command!(async |ctx, msg| {
		msg.channel_id.say(ctx, format!(
			"List of available commands:```diff\n{}```",
			COMMANDS.keys().map(|key| format!("+ {key}\n")).collect::<String>()
		)).await?
	}),
	"ping" => command!(async |ctx, msg| {
		let start = Instant::now();
		let mut pong = msg.channel_id.say(&ctx, "Loading...").await?;
		let elapsed = start.elapsed();
		pong.edit(ctx, EditMessage::new().content(format!("Bot latency: {elapsed:?}"))).await?;
	}),
	"connect" => command!(async |ctx, msg| {
		msg.channel_id.send_message(
			&ctx,
			CreateMessage::new()
				.content("A password is required.")
				.button(CreateButton::new("login").label("Login"))
		).await?;
	})
};
