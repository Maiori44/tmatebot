use std::{collections::HashSet, env, error::Error, sync::LazyLock};
use commands::COMMANDS;
use interactions::INTERACTIONS;
use owo_colors::OwoColorize;
use serenity::{
	all::{Context, EventHandler, GatewayIntents, Interaction, Message, Ready, UserId}, async_trait, futures::future::BoxFuture, Client
};
use phf::OrderedMap;

mod extensions;
mod connections;
mod commands;
mod interactions;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;
pub type Executable<T> = fn(Context, T) -> BoxFuture<'static, Result<()>>;

macro_rules! executable {
	(async |$ctx:ident, $($arg:ident)+| $code:block) => {
		|$ctx, $($arg)+| {
			Box::pin(async move {
				$code;
				return Ok(());
			})
		}
	}
}

pub(crate) use executable;

pub trait ExecutableArg {
	fn key(&self) -> String;
	fn requester(&self) -> String;
}

async fn execute<T: ExecutableArg>(
	map: &OrderedMap<&str, Executable<T>>,
	ctx: Context,
	arg: T,
) {
	let key = arg.key();
	let requester = arg.requester();
	let Some(to_execute) = map.get(&key) else { return };
	if let Err(e) = to_execute(ctx, arg).await {
		println!(
			"{} trying to run `{}` as requested by {requester}: {e}.",
			"Error".red().bold(),
			key.purple()
		);
	} else {
		println!(
			"{} ran `{}` as requested by {requester}.",
			"Successfully".green().bold(),
			key.purple()
		);
	}
}

fn load_list() -> Result<HashSet<UserId>> {
	Ok(env::var("WHITELIST")?.split(',')
		.map_while(|string_id| Some(UserId::new(string_id.parse().ok()?)))
		.collect())
}

static WHITELIST: LazyLock<HashSet<UserId>> = LazyLock::new(|| load_list().unwrap_or_default());

struct Handler;

#[async_trait]
impl EventHandler for Handler {
	async fn ready(&self, _: Context, _: Ready) {
		println!("Bot ready!");
	}

	async fn message(&self, ctx: Context, msg: Message) {
		if msg.author.bot {
			return
		}
		if !WHITELIST.contains(&msg.author.id) {
			println!(
				"Refused possible request by unauthorized user {}.",
				msg.author.id.bright_blue()
			);
			msg.channel_id.say(ctx, "You are not authorized.").await.ok();
			return
		}
		execute(&COMMANDS, ctx, msg).await;
	}

	async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
		if let Some(interaction) = interaction.as_modal_submit() {
			interaction.defer(ctx).await.ok();
			return
		}
		let Interaction::Component(interaction) = interaction else { return };
		if !WHITELIST.contains(&interaction.user.id) {
			println!(
				"Refused possible interaction with unauthorized user {}.",
				interaction.id.bright_blue()
			);
			return
		}
		execute(&INTERACTIONS, ctx, interaction).await;
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	dotenvy::dotenv()?;
	let mut client = Client::builder(env::var("TOKEN")?, GatewayIntents::DIRECT_MESSAGES)
		.event_handler(Handler)
		.await?;
	client.start().await?;
	Ok(())
}
