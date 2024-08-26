use std::{collections::HashSet, env, error::Error, sync::LazyLock};
use commands::COMMANDS;
use interactions::INTERACTIONS;
use owo_colors::OwoColorize;
use serenity::{
	all::{Context, EventHandler, GatewayIntents, Interaction, Message, Ready, UserId},
	async_trait,
	Client
};

fn load_list() -> Result<HashSet<UserId>, Box<dyn Error>> {
	Ok(env::var("WHITELIST")?.split(',')
		.map(|string_id| UserId::new(string_id.parse().unwrap_or_default()))
		.collect())
}

mod commands;
mod interactions;

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
			msg.reply(ctx, "You are not authorized.").await.ok();
			println!("Refused possible request by unauthorized user {}.", msg.author.id.bright_blue());
			return
		}
		let mut args = msg.content.split_whitespace();
		let Some(command) = COMMANDS.get(args.next().unwrap_or_default()) else { return };
		if let Err(e) = command(&ctx, &msg, args).await {
			println!(
				"{} trying to run `{}` as requested by {}: {e}",
				"Error".red(),
				msg.content.purple(),
				msg.author.id.bright_blue()
			);
		} else {
			println!(
				"Successfully ran `{}` as requested by {}.",
				msg.content.purple(),
				msg.author.id.bright_blue()
			);
		}
	}

	async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
		let Interaction::Component(interaction) = interaction else { return };
		if !WHITELIST.contains(&interaction.user.id) {
			return
		}
		let Some(interaction_fn) = INTERACTIONS.get(&interaction.data.custom_id) else { return };
		interaction_fn(&ctx, &interaction).await.unwrap();
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	dotenvy::dotenv()?;
	let mut client = Client::builder(env::var("TOKEN")?, GatewayIntents::DIRECT_MESSAGES)
		.event_handler(Handler)
		.await?;
	client.start().await?;
	Ok(())
}
