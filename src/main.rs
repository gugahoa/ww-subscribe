use ww_subscription::models::*;
use ww_subscription::*;

use std::collections::HashMap;
use std::env;
use std::thread;
use std::time::Duration;

use futures::stream::Stream;
use futures::sync::mpsc;
use telebot::bot;
use tokio_core::reactor::Core;

use diesel::prelude::*;
use rayon::prelude::*;
use telebot::functions::*;
use tokio::prelude::*;

fn telegram_bot(receiver: mpsc::UnboundedReceiver<(i32, String)>) {
    thread::spawn(move || {
        let mut lp = Core::new().unwrap();
        let bot = bot::RcBot::new(
            lp.handle(),
            &env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN env var not set"),
        )
        .update_interval(200);

        let conn = establish_connection();

        let handle = bot.new_cmd("/subscribe").and_then(move |(bot, msg)| {
            use ww_subscription::schema::subscriptions::dsl::*;

            let text = if let Some(text) = msg.text {
                text
            } else {
                return bot
                    .message(msg.chat.id, "Expected text, found none".into())
                    .send();
            };

            let result = diesel::insert_into(subscriptions)
                .values(&NewSubscription {
                    chat_id: msg.chat.id as i32,
                    novel: &text,
                })
                .execute(&conn);

            match result {
                Ok(_) => {
                    println!(
                        "Successfully inserted subscription from chat_id={} to novel={}",
                        msg.chat.id, text
                    );
                    bot.message(msg.chat.id, "Success".into()).send()
                }
                Err(e) => {
                    println!("Failed to insert subscription. err={:?}", e);
                    bot.message(msg.chat.id, "Fail".into()).send()
                }
            }
        });

        bot.register(handle);

        let bot_clone = bot.clone();
        lp.handle().spawn(
            receiver
                .and_then(move |(chat, novel): (i32, String)| {
                    bot_clone
                        .message(chat as i64, novel)
                        .send()
                        .map(|_| ())
                        .map_err(|_| ())
                        .into_future()
                })
                .map_err(|e| panic!("error={:?}", e))
                .for_each(|_| Ok(())),
        );

        bot.run(&mut lp).unwrap();
    });
}

fn main() {
    let connection = establish_connection();
    let (sender, receiver) = mpsc::unbounded();

    telegram_bot(receiver);

    loop {
        let channel = rss::Channel::from_url("https://www.wuxiaworld.com/feed/chapters")
            .expect("Could not fetch RSS feed");

        let novels_map: HashMap<&str, &str> = channel
            .items()
            .into_iter()
            .map(|novel| (novel.categories()[0].name(), novel.link()))
            .filter(|(_name, link)| link.is_some())
            .map(|(name, link)| (name, link.unwrap()))
            .collect();

        let mut existing_novels: HashMap<String, i32> = {
            use ww_subscription::schema::novels::dsl::*;
            novels
                .filter(name.eq_any(novels_map.keys()))
                .load::<Novel>(&connection)
                .expect("Failed to query novels")
                .into_iter()
                .map(|novel| (novel.name, novel.id))
                .collect()
        };

        let sent_novels: Vec<String> = {
            use ww_subscription::schema::novel_history::dsl::*;
            novel_history
                .filter(link.eq_any(novels_map.values()))
                .load::<NovelHistory>(&connection)
                .expect("Failed to query novel history")
                .into_iter()
                .map(|novel| novel.link)
                .collect()
        };

        let inserted_novels: HashMap<String, i32> =
            novels_map
                .keys()
                .into_iter()
                .map(|novel| String::from(*novel))
                .filter(|novel| !existing_novels.contains_key(novel))
                .map(|novel| {
                    let new_novel = NewNovel { name: &novel };

                    use ww_subscription::schema::novels::dsl::*;
                    println!("Inserting new novel: {}", novel);
                    diesel::insert_into(novels)
                        .values(&new_novel)
                        .returning(id)
                        .get_results(&connection)
                        .ok()
                        .and_then(|result| result.into_iter().next())
                        .map(|novel_id| (novel, novel_id))
                })
                .filter(Option::is_some)
                .map(Option::unwrap)
                .collect();

        existing_novels.extend(inserted_novels);
        novels_map.par_iter().for_each(|(novel, link)| {
            if sent_novels.contains(&(*link).to_owned()) {
                return;
            }

            let connection = establish_connection();
            use ww_subscription::schema::novel_history;
            let result = diesel::insert_into(novel_history::table)
                .values(&NewNovelHistory {
                    novel_id: existing_novels.get(*novel).unwrap().clone(),
                    link: (*link).to_owned(),
                })
                .execute(&connection)
                .map(|_| {
                    use ww_subscription::schema::subscriptions;
                    let subs = subscriptions::table
                        .filter(subscriptions::novel.eq(novel))
                        .load::<Subscription>(&connection)
                        .expect("Failed to select subscriptions");

                    subs.par_iter().for_each(|sub| {
                        sender
                            .unbounded_send((
                                sub.chat_id,
                                format!("New chapter released for novel {}\nLink: {}", novel, link),
                            ))
                            .expect(&format!(
                                "Failed to send notification to user={} for novel={}",
                                sub.chat_id, sub.novel
                            ));
                    });
                });

            match result {
                Ok(_) => println!(
                    "Created new novel history for novel {} link {}",
                    novel, link
                ),
                Err(err) => eprintln!(
                    "Failed to create new novel history for novel {}, err = {}",
                    novel, err
                ),
            }
        });

        thread::sleep(Duration::from_secs(10));
    }
}
