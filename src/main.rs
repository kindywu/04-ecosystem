extern crate dotenv;

use anyhow::Result;
use dotenv::dotenv;
use sqlx::{query_as, FromRow, PgPool, Pool, Postgres};
use std::env;

#[derive(Debug, FromRow)]
pub struct Url {
    #[sqlx(default)]
    id: String,
    #[sqlx(default)]
    url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let pg_url = env::var("DATABASE_URL").expect("无法读取数据库连接");

    let pool = PgPool::connect(&pg_url).await?;
    {
        let id = "X_2C4H";
        let url_record = query_as!(Url, r#"SELECT id, url FROM urls WHERE id = $1"#, id)
            .fetch_optional(&pool)
            .await?;
        println!("{:?} ", url_record);
    }

    {
        let id = "id_not_exist";
        if let Some(url) = get_url(&pool, id).await? {
            println!("{}=>{}", url.id, url.url)
        } else {
            println!("{} not found", id)
        }
    }

    {
        let uri = "www.bing.com";

        let id = shorten(&pool, uri).await?;
        println!("id is {}", id);
    }

    Ok(())
}

async fn get_url(executor: &Pool<Postgres>, id: &str) -> Result<Option<Url>> {
    let url_record = query_as!(Url, r#"SELECT id, url FROM urls WHERE id = $1"#, id)
        .fetch_optional(executor)
        .await?;
    Ok(url_record)
}

#[derive(Debug, FromRow)]
pub struct Id {
    id: String,
}

async fn shorten(executor: &Pool<Postgres>, uri: &str) -> Result<String> {
    let id = nanoid::nanoid!(6);

    for _ in 1..3 {
        let result = query_as!(
            Id,
            r#"
                INSERT INTO urls (id, url) VALUES ($1, $2)
                ON CONFLICT(url) DO UPDATE SET url=EXCLUDED.url RETURNING id
                "#,
            &id,
            &uri
        )
        .fetch_one(executor)
        .await;

        match result {
            Ok(url) => return Ok(url.id),
            Err(sqlx::Error::Database(e)) => {
                if let Some(code) = e.code() {
                    if code == "23505" {
                        continue;
                    }
                }
                return Err(anyhow::anyhow!("max try"));
            }
            Err(e) => return Err(e.into()),
        }
    }
    Err(anyhow::anyhow!("max try"))
}
