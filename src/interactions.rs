use std::{error::Error, time::Duration};
use serenity::{all::{ComponentInteraction, Context, CreateQuickModal, EditMessage}, futures::future::BoxFuture};
use phf::{phf_ordered_map, OrderedMap};
use tokio::fs;
use sha256;

type Interaction = for<'a> fn(
	&'a Context,
	&'a ComponentInteraction,
) -> BoxFuture<'a, Result<(), Box<dyn Error>>>;

macro_rules! interaction {
	(async |$ctx:ident, $interaction:ident| $code:block) => {
		|$ctx, $interaction| {
			Box::pin(async move {
				$code;
				return Ok(());
			})
		}
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

pub static INTERACTIONS: OrderedMap<&str, Interaction> = phf_ordered_map! {
	"login" => interaction!(async |ctx, interaction| {
		let response = ask_input(ctx, interaction, "Password").await?;
		let mut display = interaction.channel_id.say(ctx, "Loading...").await?;
		let password = fs::read_to_string(format!("password_{}.txt", interaction.user.id))
			.await
			.unwrap_or_default();
		if sha256::digest(response) != password {
			display.edit(ctx, EditMessage::new().content("Authorization failed.")).await?;
		}
	})
};
