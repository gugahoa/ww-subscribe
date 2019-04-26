#[macro_use]
extern crate diesel;

pub mod models;
pub mod schema;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use std::env;

use telebot::functions::*;
use models::*;
use tokio_core::reactor::Core;
use std::thread;
use futures::sync::mpsc;
use telebot::bot;
use tokio::prelude::*;

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}

pub fn telegram_bot(receiver: mpsc::UnboundedReceiver<(i32, String)>) {
    thread::spawn(move || {
        let mut lp = Core::new().unwrap();
        let bot = bot::RcBot::new(
            lp.handle(),
            &env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN env var not set"),
        )
            .update_interval(200);

        let conn = establish_connection();

        let handle = bot.new_cmd("/subscribe").and_then(move |(bot, msg)| {
            use schema::subscriptions;

            let text = if let Some(text) = msg.text {
                text
            } else {
                return bot
                    .message(msg.chat.id, "Expected text, found none".into())
                    .send();
            };

            let result = diesel::insert_into(subscriptions::table)
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

