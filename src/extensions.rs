use serenity::{all::{ButtonStyle, CacheHttp, ChannelId, CreateButton, CreateMessage, EditMessage, Message}, Result};

pub trait MessageExt {
	async fn edit_content(
		&mut self,
		cache_http: impl CacheHttp,
		content: impl Into<String>,
	) -> Result<()>;
}

impl MessageExt for Message {
	async fn edit_content(
		&mut self,
		cache_http: impl CacheHttp,
		content: impl Into<String>,
	) -> Result<()> {
		self.edit(cache_http, EditMessage::new().content(content)).await
	}
}

pub trait ChannelIdExt {
	async fn send_button(
		self,
		cache_http: impl CacheHttp,
		content: impl Into<String>,
		label: impl Into<String>,
		style: ButtonStyle,
	) -> Result<Message>;
}

impl ChannelIdExt for ChannelId {
	async fn send_button(
		self,
		cache_http: impl CacheHttp,
		content: impl Into<String>,
		label: impl Into<String>,
		style: ButtonStyle,
	) -> Result<Message> {
		let label = label.into();
		self.send_message(
			cache_http,
			CreateMessage::new()
				.content(content)
				.button(CreateButton::new(&label).label(label).style(style))
		).await
	}
}
