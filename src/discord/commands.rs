use super::*;
use crate::game::Level;

use core::fmt;

pub const PING: u64 = xxhash_rust::const_xxh3::xxh3_64(b"ping");
pub const HELP: u64 = xxhash_rust::const_xxh3::xxh3_64(b"help");
pub const ROLL: u64 = xxhash_rust::const_xxh3::xxh3_64(b"roll");
pub const WHOAMI: u64 = xxhash_rust::const_xxh3::xxh3_64(b"whoami");
pub const SET_WELCOME: u64 = xxhash_rust::const_xxh3::xxh3_64(b"set_welcome");

//Normally you should prefer to return future, but most of commands are too complicated to avoid
//type erasure, hence hope compiler is able to inline async
impl super::Handler {
    #[inline]
    pub async fn handle_roll(&self, ctx: HandlerContext<'_>) -> serenity::Result<()> {
        //format_args is not thread safe for await
        struct Text(Result<cute_dnd_dice::Roll, cute_dnd_dice::ParseError>);

        impl fmt::Display for Text {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.0.as_ref() {
                    Ok(roll) => write!(f, "Roll {}: {}", roll, roll.roll()),
                    Err(error) => write!(f, "Cannot recognize dice: {}", error)
                }
            }
        }

        let roll = cute_dnd_dice::Roll::from_str(&ctx.text[4..]);
        ctx.msg.reply(ctx, Text(roll)).await.map(|_| ())
    }

    #[inline]
    pub async fn handle_whoami(&self, ctx: HandlerContext<'_>) -> serenity::Result<()> {
        match self.state.db.get::<data::User>(ctx.msg.author.id.0) {
            Ok(user) => {
                let result = ctx.msg.author.direct_message(&ctx, |m| {
                    m.embed(|m| {

                        let level = Level::new(user.exp);
                        m.title("Profile")
                            .field("Level", level.level, true)
                            .field("Exp", level, true)
                            .field("Cash", user.cash, false)
                            .field("Moderator:", ctx.is_mod, false)
                    })
                }).await;

                if result.is_ok() {
                    let _ = ctx.msg.react(&ctx, emoji::OK).await;
                }
                result
            },
            Err(error) => {
                rogu::error!("Unable to get user's info: {}", error);
                ctx.msg.reply(ctx, "Cannot find information :(").await
            },
        }.map(|_| ())
    }

    #[inline]
    pub async fn handle_help(&self, ctx: HandlerContext<'_>) -> serenity::Result<()> {
        let result = ctx.msg.author.direct_message(&ctx, |msg| {
            msg.content(CMD_HELP_TXT)
        }).await;

        match result {
            Ok(_) => {
                let _ = ctx.msg.react(&ctx, emoji::OK).await;
                Ok(())
            },
            Err(err) => Err(err)
        }
    }

    #[inline]
    pub async fn handle_set_welcome(&self, ctx: HandlerContext<'_>) -> serenity::Result<()> {
        match ctx.is_mod {
            false => {
                let _ = ctx.msg.react(&ctx, emoji::KINSHI).await;
            },
            true => {
                //let self.state.db.get::<data::Server();

                let _ = ctx.msg.react(&ctx, emoji::OK).await;
            },
        }

        Ok(())
    }
}
