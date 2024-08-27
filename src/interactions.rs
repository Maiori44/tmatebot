use std::{env, time::Duration};
use owo_colors::OwoColorize;
use serenity::{
	all::{
		ComponentInteraction,
		Context,
		CreateInputText,
		CreateQuickModal,
		InputTextStyle,
		Message,
		UserId
	},
	Error,
	Result
};
use crate::{
	connections::{Connection, CONNECTIONS},
	executable,
	extensions::MessageExt, Executable, ExecutableArg
};
use phf::{phf_ordered_map, OrderedMap};
use tokio::fs;
use sha256;

impl ExecutableArg for ComponentInteraction {
	fn key(&self) -> String {
		self.data.custom_id.to_lowercase()
	}

	fn requester(&self) -> String {
		self.user.id.bright_blue().to_string()
	}
}

async fn ask_input(
	ctx: &Context,
	interaction: &ComponentInteraction,
	fields: &[(&str, Option<fn(CreateInputText) -> CreateInputText>)],
) -> Result<Vec<String>> {
	let mut modal = CreateQuickModal::new("Awaiting input").timeout(Duration::from_secs(60));
	for (label, func) in fields {
		modal = modal.field({
			let builder = CreateInputText::new(InputTextStyle::Short, *label, "");
			if let Some(func) = func {
				func(builder)
			} else {
				builder
			}
		});
	}
	interaction.quick_modal(ctx, modal).await?.map_or(
		Err(Error::Other("Modal timeout ended")),
		|response| Ok(response.inputs)
	)
}

async fn assert_password(
	ctx: &Context,
	password: &str,
	userid: UserId,
	msg: &mut Message
) -> Result<()> {
	let saved_password = fs::read_to_string(format!("password_{}.dat", userid))
		.await
		.unwrap_or_default();
	if password.is_empty() && saved_password.is_empty() {
		return Err(Error::Other("No passwords defined"));
	}
	if sha256::digest(password) == saved_password {
		Ok(())
	} else {
		msg.edit_content(ctx, "Authorization failed.").await?;
		Err(Error::Other("Password mismatch"))
	}
}

pub static INTERACTIONS: OrderedMap<&str, Executable<ComponentInteraction>> = phf_ordered_map! {
	"login" => executable!(async |ctx, interaction| {
		let [ref timeout, ref password] = ask_input(&ctx, &interaction, &[
			("Timeout", Some(|builder| builder.value(env::var("TIMEOUT").unwrap_or_default()))),
			("Password", None),
		]).await?[..2] else { unreachable!() };
		let mut display = interaction.channel_id.say(&ctx, "Loading...").await?;
		assert_password(&ctx, password, interaction.user.id, &mut display).await?;
		Connection::new(ctx, display).await?;
	}),
	"register" => executable!(async |ctx, interaction| {
		let [ref old_password, ref new_password] = ask_input(&ctx, &interaction, &[
			("Old Password", Some(|builder| builder
				.placeholder("Leave blank when first registering")
				.required(false))),
			("New Passowrd", None),
		]).await?[..2] else { unreachable!() };
		let mut result_msg = interaction.channel_id.say(&ctx, "Loading...").await?;
		match assert_password(&ctx, old_password, interaction.user.id, &mut result_msg).await {
			Ok(()) | Err(Error::Other("No passwords defined")) => {
				fs::write(
					format!("password_{}.dat", interaction.user.id),
					sha256::digest(new_password)
				).await?;
				result_msg.edit_content(ctx, "Passoword updated.").await?;
			},
			Err(e) => Err(e)?,
		};
	}),
	"close" => executable!(async |ctx, interaction| {
		if let Some(connection) = CONNECTIONS.lock().await.remove(&interaction.message.id) {
			connection.terminate().await?;
			interaction.defer(ctx).await?;
		}
	})
};
