use std::{collections::HashMap, io::BufRead, process::Stdio, sync::LazyLock};
use circular_buffer::CircularBuffer;
use serenity::all::{ButtonStyle, Context, Message, MessageId};
use crate::{extensions::MessageExt, Result};
use tokio::{
	io::AsyncReadExt,
	net::unix::pipe,
	process::{Child, Command},
	sync::Mutex,
	task::JoinHandle
};

pub static CONNECTIONS: LazyLock<Mutex<HashMap<MessageId, Connection>>> = LazyLock::new(|| {
	Mutex::new(HashMap::new())
});

#[derive(Debug)]
pub struct Connection {
	process: Child,
	reader: JoinHandle<serenity::Result<()>>,
}

impl Connection {
	pub async fn new(ctx: Context, mut display: Message) -> Result<()> {
		display.edit_button(&ctx, "Close", ButtonStyle::Danger, false).await?;
		let (tx, mut rx) = pipe::pipe()?;
		let fd = tx.into_nonblocking_fd()?;
		CONNECTIONS.lock().await.insert(display.id, Connection {
			process: Command::new("tmate")
				.arg("-F")
				.stdout(fd.try_clone()?)
				.stderr(fd)
				.stdin(Stdio::null())
				.kill_on_drop(true)
				.spawn()?,
			reader: tokio::spawn(async move {
				const N: usize = 16;
				let mut output: CircularBuffer<N, String> = CircularBuffer::new();
				let mut buf = [0u8; 128];
				let mut unfinished = false;
				loop {
					let n = rx.read(&mut buf).await.unwrap_or_default();
					if n == 0 {
						output.push_back(String::from("Session closed"));
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
					let mut display_text = String::from("```\n");
					let mut linebreaks = 0;
					for line in &output {
						if n > 0 && line.contains("0 client currently connected") {
							tokio::spawn(async move {
								if let Some(connection) = CONNECTIONS.lock().await.remove(&display.id) {
									connection.terminate().await.unwrap()
								}
							});
						}
						display_text += &line;
						display_text.push('\n');
						linebreaks += 1;
					}
					for _ in linebreaks..N {
						display_text += " \n";
					}
					display.edit_content(&ctx, display_text + "```").await?;
					if n == 0 {
						break
					}
				}
				display.edit_button(&ctx, "Close", ButtonStyle::Danger, true).await
			})
		});
		Ok(())
	}

	pub async fn terminate(mut self) -> Result<()> {
		self.process.kill().await?;
		self.reader.await??;
		Ok(())
	}
}
