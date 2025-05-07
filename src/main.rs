use rand::Rng;
// use meval;
use poise::serenity_prelude::{
    self as serenity, CacheHttp, ClientBuilder, CreateAttachment, CreateMessage, GatewayIntents,
    Mentionable, Ready,
};
use regex::Regex;
use sqlx::Pool;
use sqlx::sqlite::SqlitePool;
use std::{fs, path::PathBuf};
struct Data {
    pub db_pool: Pool<sqlx::Sqlite>,
    pub start_time: std::time::Instant,
} // User data, which is stored and accessible in all command invocations
const SHARDS: u32 = 32;
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

// This is the function that Poise will call to get the prefix for a guild
async fn dynamic_prefix_resolver(ctx: Context<'_>) -> Result<Option<String>, Error> {
    // Get the guild ID from the context. If not in a guild (e.g., a DM),
    // return None to signify using the default prefix.
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        _none => return Ok("td!"), // Use default prefix in DMs
    };

    // Query the database for the prefix associated with this guild ID
    // We use query_as! for type-safe fetching.
    // SQLite uses '?' for placeholders
    let prefix_row = sqlx::query_as!(
        PrefixRow,
        "SELECT prefix FROM guild_prefixes WHERE guild_id = ?", // Use ? for placeholder
        guild_id.get() as i64 // Cast Discord's Uid to i64 for database storage
    )
    .fetch_optional(&ctx.data().db_pool) // fetch_optional returns Option<Row>
    .await
    .map_err(|e| {
        // Log the error but don't crash the bot.
        // In a real bot, you might want more sophisticated error handling.
        eprintln!(
            "Database error fetching prefix for guild {}: {}",
            guild_id, e
        );
        // Return the error to the framework to be handled by the on_error hook
        e
    })?;

    // If a prefix was found, return it. Otherwise, return None to use the default.
    Ok(prefix_row.map(|row| row.prefix))
}

// Helper struct to map the query result
struct PrefixRow {
    prefix: String,
}

/// Sets the command prefix for this guild.
/// Requires Administrator permissions.
#[poise::command(guild_only, prefix_command)] // This command can only be used in a guild
async fn set_prefix(
    ctx: Context<'_>,
    #[description = "The new prefix to use (max 10 characters)"] new_prefix: String,
) -> Result<(), Error> {
    // Ensure the command is run in a guild (guild_only attribute already helps, but good practice)
    let guild_id = ctx
        .guild_id()
        .ok_or("This command can only be used in a guild.")?;

    // Check if the user has Administrator permissions
    let member = ctx
        .author_member()
        .await
        .ok_or("Could not retrieve guild member.")?;
    let permissions = member
        .permissions(ctx.cache())
        .map_err(|_| "Could not retrieve member permissions.")?;

    if !permissions.administrator() {
        return Err("You need Administrator permissions to use this command.".into());
    }

    // Basic validation for the new prefix length
    if new_prefix.len() > 10 {
        return Err("The prefix cannot be longer than 10 characters.".into());
    }

    // Use INSERT ... ON CONFLICT syntax for SQLite (requires SQLite 3.24.0+)
    // Or you could use INSERT OR REPLACE INTO ... for simpler cases
    sqlx::query!(
        "INSERT INTO guild_prefixes (guild_id, prefix) VALUES (?, ?)
         ON CONFLICT (guild_id) DO UPDATE SET prefix = excluded.prefix", // Use ? for placeholders
        guild_id.get() as i64, // Cast Discord Uid to i64
        new_prefix
    )
    .execute(&ctx.data().db_pool)
    .await?; // Execute the query

    // Respond to the user confirming the prefix change
    ctx.say(format!(
        "Command prefix for this guild has been set to `{}`.",
        new_prefix
    ))
    .await?;

    Ok(())
}
mod commands {
    use ::serenity::all::GetMessages;
    use poise::CreateReply;

    use super::*;

    /// Funny command that lets users fly.
    #[poise::command(slash_command, prefix_command)]
    pub async fn fly(ctx: Context<'_>, user: serenity::Member) -> Result<(), Error> {
        ctx.say(format!("Fly high {}", user.mention())).await?;
        ctx.say("https://tenor.com/view/fly-human-fly-float-human-airplane-meme-gif-5277954545468410794").await?;
        Ok(())
    }

    /// Prefix command.
    #[poise::command(slash_command, prefix_command)]
    pub async fn writeprefix(
        ctx: Context<'_>,
        #[description = "The prefix you would like for the server."] name: String,
    ) -> Result<(), Error> {
        Ok(())
    }

    /// Meme command that can be able to translate meme files from the repository into video attachments.
    #[poise::command(slash_command, prefix_command)]
    pub async fn meme(
        ctx: Context<'_>,
        #[description = "The name of the meme (without extension)"] name: String,
    ) -> Result<(), Error> {
        let memes_path = PathBuf::from("./memes");
        let mut found_meme: Option<PathBuf> = None;

        if let Ok(entries) = fs::read_dir(&memes_path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Some(file_name_with_ext) = entry.file_name().to_str() {
                        if let Some((file_name_without_ext, _)) =
                            file_name_with_ext.rsplit_once('.')
                        {
                            if file_name_without_ext == name {
                                found_meme = Some(entry.path());
                                break;
                            }
                        }
                    }
                }
            }
        }

        match found_meme {
            Some(meme_path) => {
                ctx.send(
                    CreateReply::default()
                        .content("Here's your meme sir.")
                        .ephemeral(true),
                )
                .await?;
                ctx.channel_id()
                    .send_message(
                        ctx.http(),
                        CreateMessage::default().add_file(
                            serenity::CreateAttachment::path(meme_path)
                                .await
                                .expect("Attachment has failed to send."),
                        ),
                    )
                    .await?;
            }
            _none => {
                ctx.send(CreateReply::default().content(format!(
                    "Hush now... the meme named '{}' seems to elude us in the `./memes` folder.",
                    name
                )).ephemeral(true))
                .await?;
            }
        }

        Ok(())
    }

    /// Shows an embed about the bot and the authors of the bot.
    #[poise::command(slash_command, prefix_command)]
    pub async fn about(ctx: Context<'_>) -> Result<(), Error> {
        let embed = serenity::CreateEmbed::new()
            .title("The Dragon Of Dojima")
            .description(format!(
                "This discord bot mainly has components of fun, and moderation. It is written in rust, and hosted on github."
            ))
            .field(
                "Author",
                format!(
                    "Made by <@1221614686865461259>"
                ),
                false,
            )
            .field(
                "Hosting Service",
                format!(
                    "Self-hosted"
                ),
                false,
            )
            .field(
                "Support Server",
                format!(
                    "https://discord.gg/D3WEJ46QrQ"
                ),
                false,
            )
            .field(
                "Version #",
                format!(
                    "0.5v"
                ),
                false,
            )
            .color(serenity::Color::DARK_RED);

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
        Ok(())
    }

    /// Show help menu with all available commands
    #[poise::command(slash_command, prefix_command)]
    pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
        let prefix: &str = "td!";
        let ctx_id = ctx.id();
        let prev_button_id = format!("{}prev", ctx_id);
        let next_button_id = format!("{}next", ctx_id);
        let embed1 = serenity::CreateEmbed::new()
            .title("Bot Commands Help")
            .description(format!(
                "Use `{}` before commands or `/` for slash commands\n\
                [Support Server](https://discord.gg/D3WEJ46QrQ)",
                prefix
            ))
            .field("# GENERAL COMMANDS", "Silly, general commands that can be used by anyone.", false)
            .field(format!("{prefix}hello <user>"), "Greet a specific user or everyone", false)
            .field(format!("{prefix}ping"), "It shows the shard id of the current context, api latency and uptime.", false)
            .field(format!("{prefix}say <message>"), "Relays a message with your own message redirected with the bot.", false)
            .field(format!("{prefix}sync"), "Registers application commands globally. (Owner Only)", false)
            .field(format!("{prefix}echo <message> <messageid> <user>"), "Relays a message, replies to a message, or privately message a user with a message. (Owner Only)", false)
            .field(format!("{prefix}facts"), "Gets a random fact.", false)
            .field(format!("{prefix}joryu"), "Generates a random quote from Kiryu Kazuma from the hit game series: Yakuza.", false)
            .field(format!("{prefix}about"), "Shows information about the bot.", false)
            .field(format!("{prefix}roll <min> <max>"), "Generate random number between min and max", false)
            .field(format!("{prefix}solve <expression>"), "Calculate math expressions", false)
            .field(format!("{prefix}fly <user>"), "Funny command that doesn't actually let people fly.", false)
            .color(serenity::Color::DARK_RED);
        let embed2 = serenity::CreateEmbed::new()
            .title("Bot Commands Help")
            .description(format!(
                "Use `{}` before commands or `/` for slash commands\n\
                [Support Server](https://discord.gg/D3WEJ46QrQ)",
                prefix
            ))
            .field(
                "# MODERATION COMMANDS",
                "Commands that are used to moderate a user, by banning, kicking, or muting (todo)",
                false,
            )
            .field(
                format!("{prefix}ban <user> <reason>"),
                "Ban a user with the specified reason.",
                false,
            )
            .field(
                format!("{prefix}unban <user> <reason>"),
                "Unban a user with the specified reason.",
                false,
            )
            .field(
                format!("{prefix}kick <user> <reason>"),
                "Kick a user with the specified reason.",
                false,
            )
            .color(serenity::Color::DARK_RED);
        let reply = {
            let components = serenity::CreateActionRow::Buttons(vec![
                serenity::CreateButton::new(&prev_button_id).emoji('◀'),
                serenity::CreateButton::new(&next_button_id).emoji('▶'),
            ]);

            poise::CreateReply::default()
                .embed(embed1.clone())
                .ephemeral(true)
                .components(vec![components])
        };

        let pages: &[serenity::CreateEmbed] = &[embed1.clone(), embed2.clone()];
        ctx.send(reply).await?;

        // Loop through incoming interactions with the navigation buttons
        let mut current_page = 0;
        while let Some(press) = serenity::collector::ComponentInteractionCollector::new(ctx)
            // We defined our button IDs to start with `ctx_id`. If they don't, some other command's
            // button was pressed
            .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
            // Timeout when no navigation button has been pressed for 24 hours
            .timeout(std::time::Duration::from_secs(3600 * 24))
            .await
        {
            // Depending on which button was pressed, go to next or previous page
            if press.data.custom_id == next_button_id {
                current_page += 1;
                if current_page >= pages.len() {
                    current_page = 0;
                }
            } else if press.data.custom_id == prev_button_id {
                current_page = current_page.checked_sub(1).unwrap_or(pages.len() - 1);
            } else {
                // This is an unrelated button interaction
                continue;
            }

            // Update the message with the new page contents
            press
                .create_response(
                    ctx.serenity_context(),
                    serenity::CreateInteractionResponse::UpdateMessage(
                        serenity::CreateInteractionResponseMessage::new()
                            .embed(pages[current_page].clone()),
                    ),
                )
                .await?;
        }

        Ok(())
    }

    /// Greet a specific user or everyone
    #[poise::command(slash_command, prefix_command)]
    pub async fn hello(ctx: Context<'_>, user: Option<serenity::User>) -> Result<(), Error> {
        let greeting = match user {
            Some(user) => format!("👋 Hey there, {}!", user.name),
            _none => "👋 Hello everyone!".to_string(),
        };
        ctx.say(greeting).await?;
        Ok(())
    }

    fn calc_inner(expr: &str) -> Option<f64> {
        let ops: &[(char, fn(f64, f64) -> f64)] = &[
            ('+', |a, b| a + b),
            ('-', |a, b| a - b),
            ('*', |a, b| a * b),
            ('/', |a, b| a / b),
        ];
        for &(operator, operator_fn) in ops {
            if let Some((a, b)) = expr.split_once(operator) {
                let result: f64 = (operator_fn)(a.trim().parse().ok()?, b.trim().parse().ok()?);
                return Some(result);
            }
        }
        None
    }

    /// Calculate simple math expressions.
    #[poise::command(slash_command, prefix_command, aliases("calc", "calculator"))]
    pub async fn solve(ctx: Context<'_>, expr: String) -> Result<(), Error> {
        match calc_inner(&expr) {
            Some(result) => ctx.say(format!("Result: {}", result)).await?,
            _none => ctx.say("Failed to evaluate expression!").await?,
        };
        Ok(())
    }

    /// Ping command: shows shard id of the current context, api latency and uptime.
    #[poise::command(slash_command, prefix_command)]
    pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
        // 1. Measure message round-trip latency
        let now = std::time::Instant::now();
        let reply = ctx.say("Pinging...").await?;
        let api_latency = now.elapsed();

        // 2. Get the current shard latency
        let shard: serenity::ShardId = ctx.serenity_context().shard_id;

        // 3. Calculate uptime
        let uptime = ctx.data().start_time.elapsed();

        // 4. Format response
        let response = format!(
            "Pong!\n\
            • Shard ID: {}\n\
            • API latency: {} ms\n\
            • Uptime: {}",
            shard,
            api_latency.as_millis(),
            format_durationu(uptime)
        );

        // 5. Edit the original reply with result.
        reply
            .edit(ctx, poise::CreateReply::default().content(response))
            .await?;
        Ok(())
    }

    // Helper function to format Duration as H:M:S
    fn format_durationu(d: std::time::Duration) -> String {
        let secs = d.as_secs();
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        let secs = secs % 60;
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    }

    /// Relays a message with your own message redirected with the bot.
    #[poise::command(slash_command)]
    pub async fn say(
        ctx: Context<'_>,
        #[description = "Message to relay to the public."] message: String,
    ) -> Result<(), Error> {
        // Delete original message if prefix command
        ctx.defer().await?;
        ctx.say(message).await?;
        Ok(())
    }

    /// Relays a message, replies to a message, or privately message a user with a message. (Owner Only)
    #[poise::command(slash_command, prefix_command, owners_only)]
    pub async fn echo(
        ctx: Context<'_>,
        #[description = "Message to relay to the public."] message: Option<String>,
        #[description = "Message to reply to."] messageid: Option<serenity::MessageId>,
        #[description = "User to privately message."] user: Option<serenity::User>,
        #[description = "Attachment to display."] attachment: Option<serenity::Attachment>,
    ) -> Result<(), Error> {
        // Delete original message if prefix command
        if let poise::Context::Prefix(prefix_ctx) = ctx {
            prefix_ctx.msg.delete(&ctx.serenity_context()).await?;
        } else {
            ctx.send(
                poise::CreateReply::default()
                    .ephemeral(true)
                    .content("Sent!")
                    .reply(true),
            )
            .await?;
        }
        match attachment {
            Some(attachment) => {
                match messageid {
                    Some(messageid) => match message {
                        Some(message) => {
                            serenity::Message::reply_ping(
                                &serenity::ChannelId::message(
                                    ctx.channel_id(),
                                    ctx.http(),
                                    messageid,
                                )
                                .await?,
                                ctx.http(),
                                message,
                            )
                            .await?;
                            return Ok(());
                        }
                        ref _none => {}
                    },
                    _none => {}
                }
                match user {
                    Some(user) => match message {
                        Some(message) => {
                            user.direct_message(
                                ctx.http(),
                                CreateMessage::default().content(message.clone()).add_files(
                                    CreateAttachment::url(ctx.http(), &attachment.url).await,
                                ),
                            )
                            .await?;
                            return Ok(());
                        }
                        _none => {
                            user.direct_message(
                                ctx.http(),
                                CreateMessage::default().add_files(
                                    CreateAttachment::url(ctx.http(), &attachment.url).await,
                                ),
                            )
                            .await?;
                            return Ok(());
                        }
                    },
                    _none => {}
                }
                match message {
                    Some(message) => {
                        ctx.channel_id()
                            .send_message(
                                &ctx.serenity_context().http(),
                                CreateMessage::default().content(message.clone()).add_files(
                                    CreateAttachment::url(ctx.http(), &attachment.url).await,
                                ),
                            )
                            .await?;
                    }
                    _none => {
                        ctx.channel_id()
                            .send_message(
                                &ctx.serenity_context().http(),
                                CreateMessage::default().add_files(
                                    CreateAttachment::url(ctx.http(), &attachment.url).await,
                                ),
                            )
                            .await?;
                    }
                }
                return Ok(());
            }
            _none => {}
        }
        match message {
            Some(message) => {
                match messageid {
                    Some(messageid) => {
                        serenity::Message::reply_ping(
                            &serenity::ChannelId::message(ctx.channel_id(), ctx.http(), messageid)
                                .await?,
                            ctx.http(),
                            message,
                        )
                        .await?;
                        return Ok(());
                    }
                    _none => {}
                }
                match user {
                    Some(user) => {
                        user.direct_message(
                            ctx.http(),
                            CreateMessage::default().content(message.clone()),
                        )
                        .await?;
                        return Ok(());
                    }
                    _none => {}
                }
                ctx.channel_id()
                    .say(&ctx.serenity_context().http(), message.clone())
                    .await?;
                return Ok(());
            }
            _none => {}
        }
        return Ok(());
    }

    /// Registers application commands globally. (Owner Only)
    #[poise::command(slash_command, prefix_command, owners_only)]
    pub async fn sync(ctx: Context<'_>) -> Result<(), Error> {
        poise::samples::register_application_commands(ctx, true).await?;
        ctx.say("Properly registered the application commands globally.")
            .await?;
        return Ok(());
    }

    /// Get random messages from Joryu (The Man Who Erased His Name)
    #[poise::command(slash_command, prefix_command)]
    pub async fn joryu(ctx: Context<'_>) -> Result<(), Error> {
        static MESSAGES: &[&str] = &[
            "aaaaaaaaaAAAAAAAAAAAAAAAA",
            "Joryu, The Dragon of Dojima!",
            "Hail John Yakuza",
            "John Yakuza rapes anyone who dares speak.",
            "-# shh",
            "🤫",
            "idk",
            "KUZEEE!!!!!!",
            "Haruka?",
            "Thing is, I have cancer...",
            "Are you sure?",
            "I'd tiger drop",
            "Hello.",
            "John Yakuza hates anyone who speaks loudly of him.",
            "That's rad",
            "Shinitai yatsu dake-- Kakatte koi!",
            "KIRYUUUUU!!!",
            "what??",
        ];

        let fact = rand::seq::IndexedRandom::choose(MESSAGES, &mut rand::rng()).unwrap();
        ctx.say(format!("{}", fact)).await?;
        Ok(())
    }

    /// Get a random interesting fact
    #[poise::command(slash_command, prefix_command)]
    pub async fn facts(ctx: Context<'_>) -> Result<(), Error> {
        static FACTS: &[&str] = &[
            "Honey never spoils - 3000-year-old honey found in Egyptian tombs is still edible!",
            "Octopuses have three hearts and blue blood",
            "Bananas are berries but strawberries aren't",
            "The Eiffel Tower grows 15cm taller in summer due to thermal expansion",
            "A day on Venus is longer than its year",
            "There's enough DNA in your body to stretch to stretch to Pluto and back 17 times",
            "The first computer virus was created in 1983",
            "A group of flamingos is called a 'flamboyance'",
            "The inventor of the frisbee was turned into a frisbee after death",
            "You can't hum while holding your nose closed",
        ];

        let fact = rand::seq::IndexedRandom::choose(FACTS, &mut rand::rng()).unwrap();
        ctx.say(format!("**Did you know?**\n{}", fact)).await?;
        Ok(())
    }

    /// Generate random number between min and max
    #[poise::command(slash_command, prefix_command)]
    pub async fn roll(
        ctx: Context<'_>,
        #[description = "Minimum value"] min: i32,
        #[description = "Maximum value"] max: i32,
    ) -> Result<(), Error> {
        if min >= max {
            ctx.say("❌ Minimum value must be less than maximum!")
                .await?;
            return Ok(());
        }

        let result = rand::rng().random_range(min..=max);
        ctx.say(format!("Your random number: {}", result)).await?;
        Ok(())
    }

    /// Ban a user from the server
    #[poise::command(slash_command, prefix_command, required_permissions = "BAN_MEMBERS")]
    pub async fn ban(
        ctx: Context<'_>,
        #[description = "User to ban"] user: serenity::User,
        #[description = "Reason for ban"] reason: Option<String>,
    ) -> Result<(), Error> {
        let guild_id = ctx.guild_id().expect("Must be used in guild");
        let reason = reason.unwrap_or_else(|| "No reason provided".to_string());

        guild_id
            .ban_with_reason(&ctx.serenity_context(), user.id, 0, &reason)
            .await?;
        ctx.say(format!("Banned {} | Reason: {}", user.tag(), reason))
            .await?;
        Ok(())
    }

    /// Unban a previously banned user
    #[poise::command(slash_command, prefix_command, required_permissions = "BAN_MEMBERS")]
    pub async fn unban(
        ctx: Context<'_>,
        #[description = "User to unban"] user: serenity::User,
    ) -> Result<(), Error> {
        let guild_id = ctx.guild_id().expect("Must be used in guild");
        guild_id.unban(&ctx.serenity_context(), user.id).await?;
        ctx.say(format!("Unbanned {}", user.tag())).await?;
        Ok(())
    }

    /// Kick a user from the server
    #[poise::command(slash_command, prefix_command, required_permissions = "KICK_MEMBERS")]
    pub async fn kick(
        ctx: Context<'_>,
        #[description = "User to kick"] user: serenity::User,
        #[description = "Reason for kick"] reason: Option<String>,
    ) -> Result<(), Error> {
        let guild_id = ctx.guild_id().expect("Must be used in guild");
        let reason = reason.unwrap_or_else(|| "No reason provided".to_string());

        guild_id
            .kick_with_reason(&ctx.serenity_context(), user.id, &reason)
            .await?;
        ctx.say(format!(" Kicked {} | Reason: {}", user.tag(), reason))
            .await?;
        Ok(())
    }

    /// Deletes a specified amount of messages.
    #[poise::command(
        slash_command,
        prefix_command,
        required_permissions = "MANAGE_MESSAGES",
        aliases("clean", "clear", "bulkdel")
    )]
    pub async fn purge(
        ctx: Context<'_>,
        #[description = "Target user"] user: serenity::User,
        #[description = "Number of messages to delete"] mut amount: u8,
    ) -> Result<(), Error> {
        let channel_id = ctx.channel_id();
        let mut total_deleted = 0;
        let mut last_message_id = ctx.id();

        while amount > 0 {
            // Fetch up to 100 messages before last_message_id
            let fetch_limit = std::cmp::min(100, amount);
            let retriever = GetMessages::new()
                .limit(fetch_limit)
                .before(last_message_id);
            let messages = channel_id.messages(ctx.http(), retriever).await?;

            // Filter messages from the target user
            let filtered_messages: Vec<_> = messages
                .iter()
                .filter(|msg| msg.author.id == user.id)
                .map(|msg| msg.id)
                .collect();

            if filtered_messages.is_empty() {
                break; // No more messages from user found
            }

            // Delete filtered messages in bulk
            channel_id.delete_messages(&ctx, &filtered_messages).await?;

            let deleted_count = filtered_messages.len() as u8;
            total_deleted += deleted_count;
            amount = amount.saturating_sub(deleted_count);

            // Update last_message_id for pagination
            last_message_id = messages
                .last()
                .map(|msg| u64::from(msg.id))
                .unwrap_or(last_message_id);
        }

        ctx.say(format!(
            "Deleted {} messages from {}",
            total_deleted, user.name
        ))
        .await?
        .delete(ctx)
        .await?;

        Ok(())
    }
}

struct Handler;

#[serenity::async_trait]
impl serenity::EventHandler for Handler {
    async fn ready(&self, context: poise::serenity_prelude::Context, _: Ready) {
        use serenity::gateway::ActivityData;
        use serenity::model::user::OnlineStatus;
        static MESSAGES: &[&str] = &[
            "Joryu, The Man Who Erased His Name",
            "John Yakuza, the one and only...",
            "I am The Dragon of Dojima.",
            "Blockuza 3",
            "ICHIBANN!!",
            "AKIYAMA",
            "John Yakuza",
            "The Myth, The Man, The Golfer.",
            "Bitch slap for haruka",
            "Dame dame",
            "I will bury all of you...",
            "JUDGEMENT",
            "Breaking za law",
            "Breaking za world",
            "If I'm coming down, I'm bringing you all with me.",
            "Life is like a trampoline. The lower you fall, the higher you go.",
            "I'm nothing like you. You think of the yakuza as a way to die. To me... being yakuza... It's a way to live.",
            "If you’re so desperate to write yourself a title, write it in your own blood not other’s.",
            "You walk alone in the dark long enough, It starts to feel like the light'll never come. You stop wanting to even take the next step. But there's not a person in this world who knows whats waiting down that road. All we can do is choose. Stand still and cry... Or make the choice to take the next step.",
            "You're mine, punk.",
            "Some are born with talent, and some aren't. That's true. But that said... Those with talent never make it through talent alone. You have to overcome. Find boundaries, and break them. The only way to grow is by overcoming challenges.",
            "Today's been a very bad day... and its put me in a real shitty mood. Just your bad luck to run into me",
            "Jo Amon?",
            "KIRYU-CHAN! NO",
            "You lay one god damn finger on Makoto Makimura... And I'll bury the Tojo Clan. I'll crush it down to the last man. This, I swear to you!",
            "I'll be the one who will kill you, not this disease.",
            "I'll be damned. The Punk Kid's finally turned... turned into a true Yakuza.",
            "Yo... Kiryu-Chan!",
            "Guess I needed them more than they needed me...",
            "KUZEEE!!!!!!",
            "Haruka?",
            "Thing is, I have cancer...",
            "Are you sure?",
            "I'd tiger drop.",
            "That's rad.",
            "Totally",
            "Shinitai yatsu dake-- Kakatte koi!",
            "KIRYUUUUU!!!",
        ];

        if context.shard_id == serenity::ShardId(SHARDS - 1) {
            println!("Client has started.");
        }
        tokio::spawn(async move {
            loop {
                context.set_presence(
                    Some(ActivityData::custom(format!(
                        "{}",
                        rand::seq::IndexedRandom::choose(MESSAGES, &mut rand::rng()).unwrap()
                    ))),
                    OnlineStatus::Online,
                );
                tokio::time::sleep(std::time::Duration::from_millis(50000)).await;
            }
        });
    }
    async fn message(
        &self,
        context: poise::serenity_prelude::Context,
        message: poise::serenity_prelude::Message,
    ) {
        static INSULTS: &[&str] = &[
            "Microsoft? I'm sorry, did you mean, Microdick?",
            "Windows, more like Winblows.",
            "Windows Update: ruining your day since 1995.",
            "Bill Gates built a monopoly just to prove mediocrity scales.",
            "Microsoft: turning simple problems into enterprise-level disasters.",
            "Windows Defender? It couldn't defend a paperclip from Clippy.",
            "Microsoft Edge? More like Soft-Edged Insecurity.",
            "Microsoft Word crashes more than my self-esteem.",
        ];

        let comprehensive_pattern = r"(?i)\b(Microsoft|MS|Windows|Win|XP|Vista|NT)\b";
        let re_comprehensive = Regex::new(comprehensive_pattern).unwrap(); // Handle errors properly!
        let messagere = re_comprehensive.is_match(&message.content.clone());
        if messagere && !message.author.bot {
            let insult = rand::seq::IndexedRandom::choose(INSULTS, &mut rand::rng()).unwrap();
            let _ = message.reply_ping(context.http, *insult).await;
        }
    }
}

#[tokio::main]
async fn main() {
    // Get the discord token set in `Secrets.toml`

    // Load environment variables from a .env file
    dotenv().ok();

    // Get the Discord bot token and database URL from environment variables
    let token = env::var("DISCORD_TOKEN").expect("Expected a DISCORD_TOKEN in the environment");
    // For SQLite, the DATABASE_URL is typically a file path, e.g., "sqlite:database.db"
    let database_url =
        env::var("DATABASE_URL").expect("Expected a DATABASE_URL in the environment");

    // Set up the SQLx database connection pool for SQLite
    let pool = SqlitePool::connect(&database_url).await?; // Use SqlitePool

    // Run database migrations (optional but recommended for managing schema changes)
    // Ensure you have a 'migrations' directory with your SQL migration files.
    // You'll need the sqlx-cli installed (`cargo install sqlx-cli`).
    // To create a migration: `sqlx migrate add create_guild_prefixes_table`
    // To run migrations: `sqlx migrate run`
    let migrator = Migrator::new(Path::new("./migrations")).await?;
    migrator.run(&pool).await?;

    let framework: poise::Framework<_, _> = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                set_prefix(),
                commands::help(),
                commands::hello(),
                commands::ping(),
                commands::echo(),
                commands::ban(),
                commands::unban(),
                commands::say(),
                commands::kick(),
                commands::facts(),
                commands::roll(),
                commands::solve(),
                commands::about(),
                commands::joryu(),
                commands::fly(),
                commands::meme(),
                commands::purge(),
                commands::sync(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                case_insensitive_commands: false,
                mention_as_prefix: true,
                dynamic_prefix: Some(|ctx| Box::pin(dynamic_prefix_resolver(ctx))), // Use our custom resolver
                /*
                dynamic_prefix: Some(
                    |ctx| {
                        get_prefix(&ctx.data, ctx.guild_id)
                    }
                ),
                */
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                println!("Registering commands...");
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                let dbpool = SqlitePool::connect(&database_url).await?; // Use SqlitePool
                println!("Registered commands.");
                Ok(Data {
                    start_time: std::time::Instant::now(),
                    db_pool: dbpool,
                })
            })
        })
        .build();

    let mut client = ClientBuilder::new(discord_token, GatewayIntents::all())
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("The client has unexpectedly crashed.");

    println!("Starting client...");
    client
        .start_shards(SHARDS)
        .await
        .expect("Sharding has failed")
}
