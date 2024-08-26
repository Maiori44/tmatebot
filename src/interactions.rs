use std::time::Duration;
use owo_colors::OwoColorize;
use serenity::all::{ComponentInteraction, Context, CreateQuickModal, EditMessage};
use phf::{phf_ordered_map, OrderedMap};
use tokio::fs;
use sha256;
use crate::{executable, Executable, ExecutableArg};

impl ExecutableArg for ComponentInteraction {
	fn key(&self) -> String {
		self.data.custom_id.to_owned()
	}

	fn requester(&self) -> String {
		self.user.id.bright_blue().to_string()
	}
}

async fn ask_input(
	ctx: &Context,
	interaction: &ComponentInteraction,
	label: &str
) -> serenity::Result<String> {
	interaction.quick_modal(
		ctx,
		CreateQuickModal::new("Awaiting input")
			.timeout(Duration::from_secs(60))
			.short_field(label)
	).await?.map_or(
		Err(serenity::Error::Other("Modal timeout ended")),
		|response| Ok(response.inputs[0].clone())
	)
}

pub static INTERACTIONS: OrderedMap<&str, Executable<ComponentInteraction>> = phf_ordered_map! {
	"login" => executable!(async |ctx, interaction| {
		let response = ask_input(&ctx, &interaction, "Password").await?;
		let mut display = interaction.channel_id.say(&ctx, "Loading...").await?;
		let password = fs::read_to_string(format!("password_{}.txt", interaction.user.id))
			.await
			.unwrap_or_default();
		if sha256::digest(response) != password {
			display.edit(ctx, EditMessage::new().content("Authorization failed.")).await?;
			return Ok(());
		}
	})
};
