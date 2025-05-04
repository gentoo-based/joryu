use rand::Rng;
// use meval;
use poise::serenity_prelude::{self as serenity, CacheHttp, ClientBuilder, CreateMessage, GatewayIntents, Mentionable, Ready};
use std::time::{Duration, Instant};
use regex::Regex;
use anyhow::Context as _;
use shuttle_runtime::SecretStore;
use shuttle_serenity::ShuttleSerenity;

struct Data {
    pub start_time: Instant,
} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Funny command that lets users fly.
#[poise::command(slash_command, prefix_command)]
async fn fly(ctx: Context<'_>, user: serenity::Member) -> Result<(), Error> {

    ctx.say(format!("Fly high {}", user.mention())).await?;
    ctx.say("https://tenor.com/view/fly-human-fly-float-human-airplane-meme-gif-5277954545468410794").await?;
    Ok(())
    
}


/// Shows an embed about the bot and the authors of the bot.
#[poise::command(slash_command, prefix_command)]
async fn about(ctx: Context<'_>) -> Result<(), Error> {
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
                "https://shuttle.dev"
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
async fn help(ctx: Context<'_>) -> Result<(), Error> {
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
        .field("# MODERATION COMMANDS", "Commands that are used to moderate a user, by banning, kicking, or muting (todo)", false)
        .field(format!("{prefix}ban <user> <reason>"), "Ban a user with the specified reason.", false)
        .field(format!("{prefix}unban <user> <reason>"), "Unban a user with the specified reason.", false)
        .field(format!("{prefix}timeout <user> <time> <reason> "), "Time a user out with the specified reason and the specified time.", false)
        .field(format!("{prefix}kick <user> <reason>"), "Kick a user with the specified reason.", false)
        .color(serenity::Color::DARK_RED);
    let reply = {
        let components = serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new(&prev_button_id).emoji('◀'),
            serenity::CreateButton::new(&next_button_id).emoji('▶'),
        ]);

        poise::CreateReply::default()
            .embed(embed1.clone())
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
async fn hello(ctx: Context<'_>, user: Option<serenity::User>) -> Result<(), Error> {
    let greeting = match user {
        Some(user) => format!("👋 Hey there, {}!", user.name),
        _none => "👋 Hello everyone!".to_string(),
    };
    ctx.say(greeting).await?;
    Ok(())
}

fn calc_inner(expr: &str) -> Option<f64> {
    let ops: &[(char, fn(f64, f64) -> f64)] = &[
        ('+', |a, b| a + b), ('-', |a, b| a - b), ('*', |a, b| a * b), ('/', |a, b| a / b)
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
#[poise::command(slash_command, prefix_command)]
pub async fn solve(ctx: Context<'_>, expr: String) -> Result<(), Error> {
    match calc_inner(&expr) {
        Some(result) => ctx.say(format!("Result: {}", result)).await?,
        _none => ctx.say("Failed to evaluate expression!").await?,
    };
    Ok(())
}

/// Ping command: shows shard id of the current context, api latency and uptime.
#[poise::command(slash_command, prefix_command)]
async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    // 1. Measure message round-trip latency
    let now = Instant::now();
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
    reply.edit(ctx, poise::CreateReply::default().content(response)).await?;
    Ok(())
}

// Helper function to format Duration as H:M:S
fn format_durationu(d: Duration) -> String {
    let secs = d.as_secs();
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let secs = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, secs)
}

/// Relays a message with your own message redirected with the bot.
#[poise::command(slash_command)]
async fn say(
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
async fn echo(
    ctx: Context<'_>,
    #[description = "Message to relay to the public."] message: String,
    #[description = "Message to reply to."] messageid: Option<serenity::MessageId>,
    #[description = "User to privately message."] user: Option<serenity::Member>
) -> Result<(), Error> {
    // Delete original message if prefix command
    if let poise::Context::Prefix(prefix_ctx) = ctx {
        prefix_ctx.msg.delete(&ctx.serenity_context()).await?;
    } else {
        ctx.defer_ephemeral().await?;
        ctx.say("Sent!").await?;
    }
    if let Some(messaged) = messageid {
        poise::serenity_prelude::Message::reply_ping(
            &ctx.channel_id().message(ctx.http(), messaged).await?,
            ctx.http(),
            message.clone(),
        )
        .await?;
        /* Deprecated
                else if let Some(_attachment) = attachment {
                    ctx.say("I'm sorry but attachments can't be used with replies. Use it seperately.").await?;
                }
        */
    } else if let Some(user) = user {
        user.user.direct_message(ctx.http(), CreateMessage::default().content(message)).await?;
        return Ok(());
    }
    ctx.channel_id().say(&ctx.serenity_context().http(), message.clone()).await?;
    return Ok(());
}

/// Registers application commands globally. (Owner Only)
#[poise::command(slash_command, prefix_command, owners_only)]
async fn sync(
    ctx: Context<'_>
) -> Result<(), Error> {
    poise::samples::register_application_commands(ctx, true).await?;
    return Ok(());
}

/// Get random messages from Joryu (The Man Who Erased His Name)
#[poise::command(slash_command, prefix_command)]
async fn joryu(ctx: Context<'_>) -> Result<(), Error> {
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
        "what??"
    ];

    let fact = rand::seq::IndexedRandom::choose(MESSAGES, &mut rand::rng()).unwrap();
    ctx.say(format!("{}", fact)).await?;
    Ok(())
}

/// Get a random interesting fact
#[poise::command(slash_command, prefix_command)]
async fn facts(ctx: Context<'_>) -> Result<(), Error> {
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
    ctx.say(format!("📚 **Did you know?**\n{}", fact)).await?;
    Ok(())
}

/// Generate random number between min and max
#[poise::command(slash_command, prefix_command)]
async fn roll(
    ctx: Context<'_>,
    #[description = "Minimum value"] min: i32,
    #[description = "Maximum value"] max: i32,
) -> Result<(), Error> {
    if min >= max {
        ctx.say("❌ Minimum value must be less than maximum!").await?;
        return Ok(());
    }

    let result = rand::rng().random_range(min..=max);
    ctx.say(format!("🎲 Your random number: {}", result)).await?;
    Ok(())
}

/// Ban a user from the server
#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "BAN_MEMBERS"
)]
async fn ban(
    ctx: Context<'_>,
    #[description = "User to ban"] user: serenity::User,
    #[description = "Reason for ban"] reason: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().expect("Must be used in guild");
    let reason = reason.unwrap_or_else(|| "No reason provided".to_string());

    guild_id
        .ban_with_reason(&ctx.serenity_context(), user.id, 0, &reason)
        .await?;
    ctx.say(format!("🔨 Banned {} | Reason: {}", user.tag(), reason)).await?;
    Ok(())
}

/// Unban a previously banned user
#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "BAN_MEMBERS"
)]
async fn unban(
    ctx: Context<'_>,
    #[description = "User to unban"] user: serenity::User,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().expect("Must be used in guild");
    guild_id.unban(&ctx.serenity_context(), user.id).await?;
    ctx.say(format!("✅ Unbanned {}", user.tag())).await?;
    Ok(())
}

/// Kick a user from the server
#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "KICK_MEMBERS"
)]
async fn kick(
    ctx: Context<'_>,
    #[description = "User to kick"] user: serenity::User,
    #[description = "Reason for kick"] reason: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().expect("Must be used in guild");
    let reason = reason.unwrap_or_else(|| "No reason provided".to_string());

    guild_id
        .kick_with_reason(&ctx.serenity_context(), user.id, &reason)
        .await?;
    ctx.say(format!("👢 Kicked {} | Reason: {}", user.tag(), reason)).await?;
    Ok(())
}

struct Handler;

#[serenity::async_trait]
impl serenity::EventHandler for Handler {
    async fn ready(&self, context: poise::serenity_prelude::Context, _: Ready) {
        use serenity::gateway::ActivityData;
        use serenity::model::user::OnlineStatus;
        static MESSAGES: &[&str] = &[
            "aaaaaaaaaAAAAAAAAAAAAAAAA",
            "Joryu, The Man Who Erased His Name",
            "John Yakuza, the one and only...",
            "I am The Dragon of Dojima.",
            "Blockuza 3",
            "ICHIBANN!!",
            "AKIYAMA",
            "Life is like a trampoline. The lower you fall, the higher you go.",
            "I'm nothing like you. You think of the yakuza as a way to die. To me... being yakuza... It's a way to live.",
            "If you’re so desperate to write yourself a title, write it in your own blood not other’s.",
            "You walk alone in the dark long enough, It starts to feel like the light'll never come. You stop wanting to even take the next step. But there's not a person in this world who knows whats waiting down that road. All we can do is choose. Stand still and cry... Or make the choice to take the next step.",
            "You're mine, punk.",
            "Some are born with talent, and some aren't. That's true. But that said... Those with talent never make it through talent alone. You have to overcome. Find boundaries, and break them. The only way to grow is by overcoming challenges.",
            "Today's been a very bad day... and its put me in a real shitty mood. Just your bad luck to run into me",
            "Jo Amon?",
            "You lay one god damn finger on Makoto Makimura... And I'll bury the Tojo Clan. I'll crush it down to the last man. This, I swear to you!",
            "I'll be the one who will kill you, not this disease.",
            "I'll be damned. The Punk Kid's finally turned... turned into a true Yakuza.",
            "Yo... Kiryu-Chan!",
            "Guess I needed them more than they needed me...",
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
        ];
        let message = rand::seq::IndexedRandom::choose(MESSAGES, &mut rand::rng()).unwrap();
        let activity = ActivityData::custom(format!("{}", message));
        let status = OnlineStatus::Online;

        println!("Started shard: {}", context.shard_id);
        context.set_presence(Some(activity.clone()), status);
        if context.shard_id == serenity::ShardId(0) {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(50000)).await;
                context.set_presence(Some(activity.clone()), status);
            }
        }
    }
    async fn message(&self, context: poise::serenity_prelude::Context, message: poise::serenity_prelude::Message) {

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
        if messagere && message.author.id != 1290208793694572597 {
            let insult = rand::seq::IndexedRandom::choose(INSULTS, &mut rand::rng()).unwrap();
            let _ = message.reply_ping(context.http, *insult).await;
        }
    }    
} 


#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secret_store: SecretStore) -> ShuttleSerenity {
    // Get the discord token set in `Secrets.toml`
    let discord_token = secret_store
    .get("DISCORD_TOKEN")
    .context("'DISCORD_TOKEN' was not found")?;

    let framework: poise::Framework<_, _> = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                help(),
                hello(),
                ping(),
                echo(),
                ban(),
                unban(),
                say(),
                kick(),
                facts(),
                roll(),
                solve(),
                about(),
                joryu(),
                fly()
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("td!".into()),
                case_insensitive_commands: false,
                mention_as_prefix: true,
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
                println!("Registered commands.");
                Ok(Data {
                    start_time: Instant::now(),
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
    let _ = client.start_shards(32).await;
    Ok(client.into())
}
