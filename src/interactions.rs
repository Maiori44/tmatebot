use std::{env, time::Duration};
use owo_colors::OwoColorize;
use serenity::{
	all::{
		ComponentInteraction,
		ComponentInteractionDataKind,
		Context,
		CreateInputText,
		CreateQuickModal,
		EditMessage,
		InputTextStyle,
		Message,
		MessageId,
		UserId
	},
	Error,
	Result
};
use crate::{
	connections::{self, Connection, CONNECTIONS},
	executable,
	extensions::MessageExt,
	Executable,
	ExecutableArg
};
use phf::{phf_ordered_map, OrderedMap};
use tokio::{fs, time::Instant};
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

pub static TIME_UNITS: OrderedMap<u8, (u64, &'static str)> = phf_ordered_map! {
	b's' => (1, "Seconds"),
	b'm' => (60, "Minutes"),
	b'h' => (3600, "Hours"),
	b'd' => (86400, "Days"),
	b'w' => (604800, "Weeks"),
	b'y' => (31536000, "Years"),
};

pub static INTERACTIONS: OrderedMap<&str, Executable<ComponentInteraction>> = phf_ordered_map! {
	"login" => executable!(async |ctx, interaction| {
		let [ref password, ref timeout] = ask_input(&ctx, &interaction, &[
			("Password", None),
			("Timeout", Some(|builder| builder.value(env::var("TIMEOUT").unwrap_or_default()))),
		]).await?[..2] else { unreachable!() };
		let mut display = interaction.channel_id.say(&ctx, "Loading...").await?;
		let instant = Instant::now();
		assert_password(&ctx, password, interaction.user.id, &mut display).await?;
		let Ok(timeout_num) = timeout[..timeout.len() - 1].parse::<u64>() else {
			display.edit_content(ctx, "Invalid timeout: must contain a number.").await?;
			Err(Error::Other("Invalid timeout number"))?;
			unreachable!();
		};
		let Some((timeout_multiplier, _)) = TIME_UNITS.get(
			&timeout.as_bytes().last().copied().unwrap_or_default().to_ascii_lowercase()
		) else {
			display.edit_content(ctx, "Invalid timeout: must end with a valid time unit.").await?;
			Err(Error::Other("Invalid timeout time unit"))?;
			unreachable!();
		};
		Connection::new(
			ctx,
			display,
			interaction.user.id,
			instant,
			timeout_num * timeout_multiplier
		).await?;
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
			connection.close().await?;
			interaction.defer(ctx).await?;
		}
	}),
	"close via menu" => executable!(async |ctx, mut interaction| {
		let ComponentInteractionDataKind::StringSelect { values } = &interaction.data.kind else {
			unreachable!()
		};
		let result = connections::gatekeep(values.iter()
			.map_while(|id|Some(MessageId::new(id.parse::<u64>().ok()?)))).await?;
		let prev_content = interaction.message.content.clone();
		interaction.message.edit(
			&ctx,
			EditMessage::new().select_menu(connections::menu().await).content(if prev_content == "_ _" {
				result
			} else {
				format!("{prev_content}\n{result}")
			})
		).await?;
		interaction.defer(ctx).await?;
	})
};
