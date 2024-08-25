use std::{error::Error, str::SplitWhitespace};
use serenity::{all::{Context, Message}, futures::future::BoxFuture};
use phf::{phf_ordered_map, OrderedMap};

type Command = for<'a> fn(
	&'a Context,
	&'a Message,
	SplitWhitespace<'a>
) -> BoxFuture<'a, Result<(), Box<dyn Error>>>;

macro_rules! command {
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
	"help" => command!(async |ctx, msg, _args| {
		msg.reply(ctx, format!(
			"List of available commands:```diff\n{}```",
			COMMANDS.keys().map(|key| format!("+ {key}\n")).collect::<String>()
		)).await?
	})
};
