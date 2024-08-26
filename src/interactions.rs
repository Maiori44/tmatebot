use std::error::Error;
use serenity::{all::{ComponentInteraction, Context}, futures::future::BoxFuture};
use phf::{phf_ordered_map, OrderedMap};

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

pub static INTERACTIONS: OrderedMap<&str, Interaction> = phf_ordered_map! {
	"login" => interaction!(async |ctx, interaction| {
		interaction.channel_id.say(ctx, "sex???").await?
	})
};
