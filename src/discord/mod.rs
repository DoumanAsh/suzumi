use serenity::async_trait; //they use this cancer :(
use serenity::model::prelude::{Ready, Message, Guild, GuildUnavailable, GuildId, Member, ChannelId, PartialGuild, RoleId, Role};
use serenity::client::Context;
use serenity::model::misc::Mentionable;

use crate::data;
use crate::assets::Assets;
use crate::db::DbView;
use crate::utils::OptionExt;

use std::collections::{HashMap, HashSet};

const CMD_HELP_TXT: &str = include_str!("../../HELP.md");

mod commands;
mod emoji;

#[derive(Clone)]
pub struct Config {
    prefix: char,
}

#[derive(Clone)]
struct Info {
    id: u64,
    owner: u64,
}

#[derive(Clone)]
struct State {
    info: Info,
    db: DbView,
    assets: Assets,
}

struct Handler {
    state: State,
    mods: tokio::sync::RwLock<HashSet<u64>>,
    config: Config,
    voice_manager: std::sync::Arc<serenity::prelude::Mutex<serenity::client::bridge::voice::ClientVoiceManager>>
}

pub struct HandlerContext<'a> {
    serenity: &'a Context,
    msg: &'a Message,
    text: &'a str,
    is_mod: bool,
}

impl<'a> serenity::http::CacheHttp for HandlerContext<'a> {
    #[inline(always)]
    fn http(&self) -> &serenity::http::client::Http {
        serenity::http::CacheHttp::http(&self.serenity)
    }

    #[inline(always)]
    fn cache(&self) -> Option<&std::sync::Arc<serenity::cache::Cache>> {
        serenity::http::CacheHttp::cache(&self.serenity)
    }
}

impl Handler {
    async fn update_mod_roles(&self, roles: &HashMap<RoleId, Role>) {
        let mut mods = self.mods.write().await;
        for (id, role) in roles {
            if role.name.eq_ignore_ascii_case("Moderator") || role.name.contains("Staff") {
                mods.insert(id.0);
            }
        }
    }

    async fn is_moderator(&self, roles: &Vec<RoleId>) -> bool {
        let mods = self.mods.read().await;
        for role in roles {
            if mods.contains(&role.0) {
                return true;
            }
        }

        false
    }

    async fn handle_cmd(&self, ctx: HandlerContext<'_>) -> serenity::Result<()> {
        use commands::*;

        let mut split = ctx.text.split_whitespace();
        //First iteration always has some value, even if there are no whitespaces;
        let cmd = split.next().unwrap_certain();

        match xxhash_rust::xxh3::xxh3_64(cmd.as_bytes()) {
            PING => ctx.msg.reply(ctx, "pong!").await.map(|_| ()),
            HELP => self.handle_help(ctx).await,
            ROLL => self.handle_roll(ctx).await.map(|_| ()),
            WHOAMI => self.handle_whoami(ctx).await,
            SET_WELCOME => self.handle_set_welcome(ctx).await,
            _ => ctx.msg.reply(ctx, "Sorry, I do not know such command").await.map(|_| ()),
        }
    }
}

///Discord event handler, contains state which is thread safe.
pub struct Discord {
    state: State,
    config: Config,
    token: crate::cli::TokenStr,
}

impl Discord {
    #[inline]
    pub async fn new(args: crate::cli::Cli, db: DbView, assets: Assets) -> Result<Self, u8> {
        let http = serenity::http::client::Http::new_with_token(args.token.0.as_str());
        let info = match http.get_current_application_info().await {
            Ok(info) => Info {
                id: info.id.0,
                owner: info.owner.id.0
            },
            Err(error) => {
                eprintln!("Unable to acquire self info. Check token. Error: {}", error);
                return Err(1);
            }
        };

        Ok(Self {
            state: State {
                info,
                db,
                assets,
            },
            config: Config {
                prefix: args.prefix,
            },
            token: args.token.0,
        })
    }

    pub async fn start(self) {
        loop {
            let voice_manager = serenity::client::bridge::voice::ClientVoiceManager::new(1, self.state.info.id.into());

            let handler = Handler {
                state: self.state.clone(),
                mods: tokio::sync::RwLock::new(HashSet::new()),
                config: self.config.clone(),
                voice_manager: std::sync::Arc::new(serenity::prelude::Mutex::new(voice_manager)),
            };

            let mut client = match serenity::client::Client::builder(self.token.as_str()).event_handler(handler).await {
                Ok(client) => client,
                Err(error) => {
                    eprintln!("Unable to connect to discord. Error: {}", error);
                    continue;
                }
            };

            loop {
                if let Err(error) = client.start().await {
                    eprintln!("Client failure. Error: {}", error);
                }
            }
        }
    }
}

#[async_trait]
impl serenity::client::EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        let content = msg.content.trim();

        let result = if msg.guild_id.is_none() {
            //Handle only commands in DM
            if !content.starts_with(self.config.prefix) {
                return;
            }

            let content = content.trim_start_matches(self.config.prefix);

            let context = HandlerContext {
                serenity: &ctx,
                msg: &msg,
                text: content,
                is_mod: false,
            };
            self.handle_cmd(context).await
        } else {
            if !content.starts_with(self.config.prefix) {
                //TODO: handle non-commands
                return;
            }

            let content = content.trim_start_matches(self.config.prefix);

            let is_mod = match msg.member(&ctx.http).await {
                Ok(member) => self.is_moderator(&member.roles).await,
                Err(error) => {
                    //It is supposed to fail when message is DM, but we handle DM differently
                    rogu::warn!("Member info is unavailable, cannot determine moderator status. Error: {}", error);
                    false
                }
            };


            let context = HandlerContext {
                serenity: &ctx,
                msg: &msg,
                text: content,
                is_mod,
            };
            self.handle_cmd(context).await
        };

        if let Err(error) = result {
            rogu::error!("Failed to deliver message: {}", error);
        }
    }

    async fn cache_ready(&self, ctx: Context, servers: Vec<GuildId>) {
        rogu::info!("Joined {} number of server", servers.len());

        for server in servers {
            let info = match server.to_partial_guild(&ctx.http).await {
                Ok(info) => info,
                Err(error) => {
                    rogu::error!("Failed to get info of server {}. Error: {}", server.0, error);
                    continue;
                }
            };

            self.update_mod_roles(&info.roles).await;
        }
    }

    async fn guild_update(&self, _: Context, _: Option<Guild>, update: PartialGuild) {
        self.update_mod_roles(&update.roles).await;
    }

    async fn guild_delete(&self, _: Context, server: GuildUnavailable, _: Option<Guild>) {
        self.state.db.delete::<data::Server>(server.id.0);
    }

    async fn guild_member_addition(&self, ctx: Context, server: GuildId, member: Member) {
        if member.user.bot {
            return;
        }

        let server_id = server.0;
        let server_info: data::Server = match self.state.db.get(server_id) {
            Ok(server_info) => server_info,
            Err(error) => {
                rogu::error!("Unable to get server info: {}", error);
                return;
            }
        };

        if server_info.welcome_ch == 0 {
            return;
        }

        let name = member.user.name.as_str();
        let mention = member.user.mention();

        let welcome_ch = ChannelId(server_info.welcome_ch);
        let result = match self.state.assets.gen_welcome(name) {
            Some(img) => {
                let mut buffer = Vec::new();
                if let Err(error) = img.write_to(&mut buffer, image::ImageOutputFormat::Png) {
                    rogu::error!("Unexpected error generating image: {}", error);
                    return;
                }

                let attach = serenity::http::AttachmentType::Bytes {
                    data: buffer.as_slice().into(),
                    filename: "welcome.png".to_owned(),
                };

                welcome_ch.send_files(&ctx.http, Some(attach), |msg| msg.content("Welcome")).await
            },
            None => welcome_ch.send_message(&ctx.http, |msg| {
                msg.content(format_args!("{}: Welcome to the server!", mention))
            }).await
        };

        if let Err(error) = result {
            rogu::error!("Unable to post welcome: {}", error);
        }
    }

    async fn ready(&self, _: Context, _: Ready) {
        rogu::debug!("Connected");
    }
}