use std::{collections::HashMap, io::BufRead, process::Stdio, sync::LazyLock, time::Duration};
use circular_buffer::CircularBuffer;
use owo_colors::OwoColorize;
use serenity::{
	all::{
		ButtonStyle,
		Context,
		CreateSelectMenu,
		CreateSelectMenuKind,
		CreateSelectMenuOption,
		Message,
		MessageId,
		UserId
	},
	Error
};
use crate::{extensions::MessageExt, Result};
use tokio::{
	io::AsyncReadExt,
	net::unix::pipe,
	process::{Child, Command},
	sync::Mutex,
	task::JoinHandle, time::{self, Instant}
};

pub static CONNECTIONS: LazyLock<Mutex<HashMap<MessageId, Connection>>> = LazyLock::new(|| {
	Mutex::new(HashMap::new())
});

#[derive(Debug)]
pub struct Connection {
	process: Child,
	pub creator: String,
	pub creation: String,
	pub timeout: String,
	reader: JoinHandle<serenity::Result<()>>,
}

impl Connection {
	pub async fn start(
		ctx: Context,
		mut display: Message,
		creator: UserId,
		instant: Instant,
		timeout: u64
	) -> Result<()> {
		println!(
			"{} connection {} that lasts {} seconds.",
			"Starting".yellow().bold(),
			display.id.bright_blue(),
			timeout.green(),
		);
		display.edit_button(&ctx, "Close", ButtonStyle::Danger, false).await?;
		let (tx, mut rx) = pipe::pipe()?;
		let fd = tx.into_nonblocking_fd()?;
		let display_creation = display.timestamp.unix_timestamp() as u64;
		let timeout_str = format!("<t:{}:R>", display_creation + timeout);
		let timeout = instant + Duration::from_secs(timeout - 1);
		CONNECTIONS.lock().await.insert(display.id, Connection {
			process: Command::new("tmate")
				.arg("-F")
				.stdout(fd.try_clone()?)
				.stderr(fd)
				.stdin(Stdio::null())
				.kill_on_drop(true)
				.spawn()?,
			creator: creator.to_user(&ctx).await?.global_name.unwrap_or_else(|| String::from("???")),
			creation: format!("<t:{display_creation}:f>"),
			timeout: timeout_str.clone(),
			reader: tokio::spawn(async move {
				const N: usize = 16;
				let mut output: CircularBuffer<N, String> = CircularBuffer::new();
				let mut buf = [0u8; 128];
				let mut unfinished = false;
				loop {
					let (mut expiring, n) = match time::timeout_at(timeout, rx.read(&mut buf)).await {
						Ok(n) => (false, n.unwrap_or_default()),
						Err(_) => (true, 1)
					};
					if !expiring {
						if n == 0 {
							output.push_back(String::from("Session closed"));
							println!(
								"{} connection {}.",
								"Closing".yellow().bold(),
								display.id.bright_blue()
							)
						}
						let bytes = &buf[0..n];
						let mut lines = bytes.lines();
						if unfinished {
							if let (Some(back), Some(Ok(line))) = (output.back_mut(), lines.next()) {
								*back += line.as_str();
							}
						}
						output.extend(lines.map_while(|line| line.ok()));
						unfinished = bytes.last().copied().unwrap_or(b'\n') != b'\n';
					}
					let mut display_text = if n > 0 {
						format!("Session expires {timeout_str}.```\n")
					} else {
						String::from("Session expired.```\n")
					};
					let mut linebreaks = 0;
					for line in &output {
						expiring |= line.ends_with("0 client currently connected");
						display_text += line;
						display_text.push('\n');
						linebreaks += 1;
					}
					if expiring && n > 0 {
						println!("Timeout reached for {}.", display.id.bright_blue());
						tokio::spawn(async move {
							if let Some(connection) = CONNECTIONS.lock().await.remove(&display.id) {
								if let Err(e) = connection.close().await {
									println!(
										"{} trying to timeout connection {}: {e}.",
										"Error".red().bold(),
										display.id.bright_blue()
									)
								}
							}
						});
					}
					for _ in linebreaks..N {
						display_text += " \n";
					}
					display.edit_content(&ctx, display_text + "```").await?;
					if n == 0 {
						break
					}
				}
				display.edit_button(&ctx, "Close", ButtonStyle::Danger, true).await?;
				println!(
					"{} closed connection {}.",
					"Successfully".green().bold(),
					display.id.bright_blue()
				);
				Ok(())
			})
		});
		Ok(())
	}

	pub async fn close(mut self) -> Result<()> {
		self.process.kill().await?;
		self.reader.await??;
		Ok(())
	}
}

pub async fn gatekeep(ids: impl Iterator<Item = MessageId>) -> Result<String> {
	async fn close(id: MessageId, connections: &mut HashMap<MessageId, Connection>) -> Result<()> {
		if let Some(connection) = connections.remove(&id) {
			connection.close().await?;
			Ok(())
		} else {
			Err(Error::Other("connection not found"))?;
			unreachable!()
		}
	}

	let mut connections = CONNECTIONS.lock().await;
	let mut result = String::with_capacity(128);
	for id in ids {
		result += &format!("**`{id}`** {}\n", match close(id, &mut connections).await {
			Ok(()) => String::from("was closed successfully."),
			Err(e) => format!("could not be closed: {e}."),
		});
	}
	Ok(result)
}

pub async fn menu() -> CreateSelectMenu {
	let connections = CONNECTIONS.lock().await;
	let menu = CreateSelectMenu::new(
		"close via menu",
		CreateSelectMenuKind::String {
			options: if connections.is_empty() {
				vec![CreateSelectMenuOption::new("You're not supposed to read this", "lmao")]
			} else {
				connections.iter().map(|(id, connection)| {
					CreateSelectMenuOption::new(
						id.to_string(),
						id.to_string()
					).description(format!("Created by {}", connection.creator))
				}).collect()
			}
		}
	);
	if connections.len() > 0 {
		menu.placeholder("Select connections to close").min_values(1).max_values(connections.len() as u8)
	} else {
		menu.placeholder("No connections left to close").disabled(true)
	}
}
